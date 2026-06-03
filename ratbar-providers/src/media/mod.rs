use crate::{
    Provider,
    media::dbus::{
        media_player2::MediaPlayer2Proxy,
        player::{Metadata, MicroDuration, PlaybackStatus, PlayerProxy},
    },
};
use color_eyre::{self as eyre, Section, SectionExt};
use futures_concurrency::prelude::*;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use smol::{
    Unblock,
    channel::{Receiver, Sender},
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    stream::StreamExt,
};
use std::{
    collections::HashMap,
    io::{BufRead, Read, stdin, stdout},
    sync::Arc,
    time::{Duration, Instant},
};
use zbus::{
    blocking::{
        Connection,
        // fdo::{DBusProxy, PropertiesProxy},
    },
    fdo::{DBusProxy, PropertiesProxy},
    names::OwnedBusName,
    zvariant::{Type, as_value::optional},
};

pub mod dbus;

#[derive(Debug)]
pub struct Media {
    args: MediaArgs,
    rx: Receiver<Event>,
    tx: Sender<Event>,
    players: HashMap<Arc<OwnedBusName>, Player>,
}

#[derive(Debug)]
struct Player {
    quit: Sender<()>,
    requests: Sender<Request>,
    metadata: Metadata,
    identity: String,
    position: MicroDuration,
    last_unpaused: Instant,
    status: PlaybackStatus,
}

#[derive(clap::Args, Debug)]
pub struct MediaArgs {
    /// Amount of time between writing to stdout
    #[arg(value_parser = humantime::parse_duration)]
    duration: Duration,
    #[arg(long, default_value_t = '⏵')]
    playing_symbol: char,
    #[arg(long, default_value_t = '⏸')]
    pause_symbol: char,
    #[arg(long, default_value_t = '⏹')]
    stop_symbol: char,
    /// Status to consider a player valid for selection
    priority: PlaybackStatus,
    /// List of players to choose in order of preference
    players: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct MediaFormat<'a> {
    length: String,
    position: String,
    progress: f32,
    title: String,
    album: String,
    artist: String,
    art: &'a str,
    button_symbol: String,
}

enum Event {
    AddPlayer {
        name: Arc<OwnedBusName>,
        player: Player,
    },
    RemovePlayer {
        name: Arc<OwnedBusName>,
    },
    UpdatePlayer {
        name: Arc<OwnedBusName>,
        data: Data,
    },
    Seeked {
        name: Arc<OwnedBusName>,
        position: MicroDuration,
    },
    Tick,
    PlayerRequest(RequestKind),
    Error(eyre::Report),
}

enum RequestKind {
    Next,
    Prev,
    Play,
    Seek(f32),
}
pub struct Request {
    length: MicroDuration,
    kind: RequestKind,
}

#[derive(Deserialize, Type, Debug)]
struct Signal<'a> {
    _address: &'a str,
    data: Data,
    _f3: Vec<&'a str>,
}

#[derive(Debug, Deserialize)]
struct ButtonMessage {
    next: Option<f32>,
    prev: Option<f32>,
    play: Option<f32>,
    seek: Option<f32>,
}

#[derive(Deserialize, Debug, Type, Default)]
#[zvariant(signature = "dict")]
#[serde(default, rename_all = "PascalCase")]
pub struct Data {
    #[serde(with = "optional")]
    playback_status: Option<PlaybackStatus>,
    #[serde(with = "optional")]
    metadata: Option<Metadata>,
    #[serde(with = "optional")]
    rate: Option<f64>,
}

impl Default for MediaFormat<'_> {
    fn default() -> Self {
        Self {
            length: "xx:xx".into(),
            position: "xx:xx".into(),
            progress: 0.0,
            title: String::new(),
            album: String::new(),
            artist: String::new(),
            button_symbol: String::from("⏵ "),
            art: "",
        }
    }
}

async fn listen_player(
    conn: Arc<zbus::Connection>,
    bus: Arc<OwnedBusName>,
    events: Sender<Event>,
    requests: Receiver<Request>,
    quit: Receiver<()>,
) {
    let result = async {
        let player_proxy = PlayerProxy::new(&conn, &*bus).await?;
        (
            async {
                let props = PropertiesProxy::new(&conn, &*bus, "/org/mpris/MediaPlayer2").await?;
                let mut props = props.receive_properties_changed().await?;
                while let Some(props) = props.next().await {
                    let body = props.message().body();
                    let deser = body.deserialize::<Signal>()?;
                    events
                        .send(Event::UpdatePlayer {
                            name: bus.clone(),
                            data: deser.data,
                        })
                        .await?;
                }
                Ok(())
            },
            async {
                let mut signals = player_proxy.receive_seeked().await?;
                while let Some(seeked) = signals.next().await {
                    let position = seeked.message().body().deserialize::<MicroDuration>()?;
                    events
                        .send(Event::Seeked {
                            name: bus.clone(),
                            position,
                        })
                        .await?;
                }
                Ok(())
            },
            async {
                while let Ok(request) = requests.recv().await {
                    let _ = match request.kind {
                        RequestKind::Seek(difference) => {
                            if let Some(length) = request.length.0 {
                                let sign = if difference.signum() == 1.0 { 1 } else { -1 };
                                let offset = Duration::from_secs_f32(
                                    (length.as_secs_f32() * difference).abs(),
                                )
                                .as_micros();
                                if let Ok(offset) = i64::try_from(offset) {
                                    let _ = player_proxy.seek(offset * sign).await;
                                }
                            }
                            Ok(())
                        }
                        RequestKind::Next => player_proxy.next().await,
                        RequestKind::Prev => player_proxy.previous().await,
                        RequestKind::Play => player_proxy.play_pause().await,
                    };
                }
                Ok(())
            },
            async {
                quit.recv().await?;
                Ok(())
            },
        )
            .race()
            .await
    };
    if let Err(report) = result.await {
        let _ = events.send(Event::Error(report)).await;
    }
}

async fn listen_names(conn: Arc<zbus::Connection>, tx: Sender<Event>) -> eyre::Result<()> {
    let proxy = zbus::fdo::DBusProxy::new(&conn).await?;
    let mut names = proxy.receive_name_owner_changed().await?;
    while let Some(name) = names.next().await {
        let args = name.args()?;
        if args.name.starts_with("org.mpris.MediaPlayer2") {
            let bus = Arc::new(OwnedBusName::from(args.name));
            match (args.old_owner.is_some(), args.new_owner.is_some()) {
                // removed
                (true, false) => {
                    tx.send(Event::RemovePlayer { name: bus }).await?;
                }
                // added
                (false, true) => {
                    tx.send(Event::AddPlayer {
                        name: bus.clone(),
                        player: fetch_player(conn.clone(), tx.clone(), bus).await?,
                    })
                    .await?;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

async fn fetch_player(
    conn: Arc<zbus::Connection>,
    events: Sender<Event>,
    bus: Arc<OwnedBusName>,
) -> eyre::Result<Player> {
    let proxy = PlayerProxy::new(&conn, &*bus).await?;
    let metadata = proxy.metadata().await?;
    let position = proxy.position().await?;
    let status = proxy.playback_status().await?;
    drop(proxy);

    let proxy = MediaPlayer2Proxy::new(&conn, &*bus).await?;
    let identity = proxy.identity().await?;
    drop(proxy);

    let (quit_sender, quit_receiver) = smol::channel::bounded(1);
    let (requests_sender, requests_receiver) = smol::channel::bounded(4);
    smol::spawn(listen_player(
        conn,
        bus,
        events,
        requests_receiver,
        quit_receiver,
    ))
    .detach();

    Ok(Player {
        requests: requests_sender,
        identity,
        quit: quit_sender,
        last_unpaused: Instant::now(),
        metadata,
        position: position.into(),
        status,
    })
}

async fn ticker(events: Sender<Event>, duration: Duration) -> eyre::Result<()> {
    let mut timer = smol::Timer::interval(duration);
    loop {
        events.send(Event::Tick).await?;
        timer.next().await;
    }
}

impl Provider for Media {
    type Args = MediaArgs;
    type Fmt<'a> = MediaFormat<'a>;

    fn init(args: Self::Args) -> eyre::Result<Self> {
        let (tx, rx) = smol::channel::unbounded();
        let conn = Connection::session()?;
        let conn = Arc::new(conn.into_inner());
        let players = get_players(&conn)?
            .into_iter()
            .map::<eyre::Result<_>, _>(|bus| {
                let bus = Arc::new(bus);
                smol::block_on(fetch_player(conn.clone(), tx.clone(), bus.clone()))
                    .map(|p| (bus, p))
            })
            .collect::<eyre::Result<HashMap<_, _>>>()?;

        smol::spawn(ticker(tx.clone(), args.duration)).detach();
        smol::spawn(listen_names(conn.clone(), tx.clone())).detach();

        Ok(Media {
            args,
            rx,
            tx,
            players,
        })
    }
    fn run(mut self) -> eyre::Result<()> {
        let tx = self.tx.clone();
        smol::block_on(
            (
                async {
                    let mut stdout = stdout().lock();
                    loop {
                        self.send(&mut stdout)?;
                        match self.rx.recv().await? {
                            Event::AddPlayer { name, player } => {
                                self.players.insert(name, player);
                            }
                            Event::RemovePlayer { name } => {
                                if let Some(player) = self.players.remove(&name) {
                                    player.quit.send(()).await?;
                                }
                            }
                            Event::UpdatePlayer { name, data } => {
                                let player = self.players.get_mut(&name).ok_or_else(|| {
                                    eyre::eyre::eyre!(
                                        "received update for non existing player: {name}"
                                    )
                                })?;
                                if let Some(metadata) = data.metadata {
                                    player.metadata = metadata;
                                }
                                if let Some(status) = data.playback_status {
                                    match status {
                                        PlaybackStatus::Playing => {
                                            player.last_unpaused = Instant::now()
                                        }
                                        PlaybackStatus::Stopped => {
                                            player.position.0 = Some(Duration::ZERO)
                                        }
                                        // handled by seeked
                                        PlaybackStatus::Paused => (),
                                    }
                                    player.status = status;
                                }
                            }
                            Event::Seeked { name, position } => {
                                let player = self.players.get_mut(&name).ok_or_else(|| {
                                    eyre::eyre::eyre!(
                                        "received update for non existing player: {name}"
                                    )
                                })?;
                                player.last_unpaused = Instant::now();
                                player.position = position;
                            }
                            Event::Error(report) => {
                                Err(report)?;
                            }
                            Event::PlayerRequest(request) => {
                                if let Some((_, player)) = self.active_player_mut() {
                                    player
                                        .requests
                                        .send(Request {
                                            length: player.metadata.length,
                                            kind: request,
                                        })
                                        .await?
                                }
                            }
                            Event::Tick => (),
                        }
                    }
                },
                async {
                    let mut buf = String::new();
                    let mut stdin = Unblock::new(stdin());
                    let mut reader = BufReader::new(&mut stdin);
                    loop {
                        buf.clear();
                        reader.read_line(&mut buf).await?;
                        if let Ok(message) = serde_json::from_str::<ButtonMessage>(&buf)
                            .with_section(|| buf.clone().header("message"))
                        {
                            if let Some(_) = message.next {
                                tx.send(Event::PlayerRequest(RequestKind::Next)).await?;
                            }
                            if let Some(_) = message.prev {
                                tx.send(Event::PlayerRequest(RequestKind::Prev)).await?;
                            }
                            if let Some(_) = message.play {
                                tx.send(Event::PlayerRequest(RequestKind::Play)).await?;
                            }
                            if let Some(delta) = message.seek {
                                tx.send(Event::PlayerRequest(RequestKind::Seek(delta / 100.0)))
                                    .await?;
                            }
                        }
                    }
                },
            )
                .race(),
        )
    }
    fn format<'a>(&'a self) -> eyre::Result<Self::Fmt<'a>> {
        let player = self.active_player();

        if let Some((_, player)) = player {
            let position = if player.status == PlaybackStatus::Playing {
                player
                    .position
                    .0
                    .map(|d| d + player.last_unpaused.elapsed())
            } else {
                player.position.0
            };

            let length = player.metadata.length.0;
            Ok(MediaFormat {
                length: format_time(length),
                position: format_time(position),
                progress: ((100 * position.unwrap_or_default().as_secs())
                    .checked_div(length.as_ref().map(Duration::as_secs).unwrap_or(1))
                    .unwrap_or_default()) as f32,
                title: player
                    .metadata
                    .title
                    .replace(")", r"\)")
                    .replace("(", r"\("),
                album: player
                    .metadata
                    .album
                    .replace(")", r"\)")
                    .replace("(", r"\("),
                artist: player
                    .metadata
                    .artists
                    .join(", ")
                    .replace(")", r"\)")
                    .replace("(", r"\("),
                art: player.metadata.art.strip_prefix("file://").unwrap_or(""),
                button_symbol: self.button(player).to_string(),
            })
        } else {
            Ok(MediaFormat::init(self.args.stop_symbol))
        }
    }
    fn update(&mut self) -> eyre::Result<()> {
        unreachable!()
    }
    fn duration(&self) -> Option<std::time::Duration> {
        Some(self.args.duration)
    }
}
impl MediaFormat<'_> {
    pub fn init(stop_symbol: char) -> Self {
        Self {
            length: "xx:xx".into(),
            position: "xx:xx".into(),
            progress: 0.0,
            title: String::new(),
            album: String::new(),
            artist: String::new(),
            button_symbol: String::from(stop_symbol),
            art: "",
        }
    }
}

impl Media {
    fn button(&self, active_player: &Player) -> char {
        match active_player.status {
            PlaybackStatus::Playing => self.args.playing_symbol,
            PlaybackStatus::Paused => self.args.pause_symbol,
            PlaybackStatus::Stopped => self.args.stop_symbol,
        }
    }
    fn active_player(&self) -> Option<(&Arc<OwnedBusName>, &Player)> {
        self.players
            .iter()
            .sorted_by_key(|(bus, _)| {
                self.args
                    .players
                    .iter()
                    .position(|prio| bus.contains(prio))
                    .unwrap_or(usize::MAX)
            })
            .find(|(_, player)| player.status <= self.args.priority)
    }
    fn active_player_mut(&mut self) -> Option<(&Arc<OwnedBusName>, &mut Player)> {
        self.players
            .iter_mut()
            .sorted_by_key(|(bus, _)| {
                self.args
                    .players
                    .iter()
                    .position(|prio| bus.contains(prio))
                    .unwrap_or(usize::MAX)
            })
            .find(|(_, player)| player.status <= self.args.priority)
    }
}

fn get_players(conn: &zbus::Connection) -> eyre::Result<Vec<OwnedBusName>> {
    smol::block_on(async {
        Ok(DBusProxy::new(conn)
            .await?
            .list_names()
            .await?
            .into_iter()
            .filter(|name| name.starts_with("org.mpris.MediaPlayer2."))
            .collect::<Vec<_>>())
    })
}

fn format_time(duration: Option<Duration>) -> String {
    if let Some(duration) = duration {
        let mut duration = duration.as_secs();
        let secs = duration % 60;
        duration /= 60;
        let mins = duration % 60;
        duration /= 60;
        let hours = duration;

        if hours != 0 {
            format!("{hours}:{mins:0>2}:{secs:0>2}")
        } else {
            format!("{mins}:{secs:0>2}")
        }
    } else {
        String::from("xx:xx")
    }
}
