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
    io::{AsyncBufReadExt, BufReader},
    stream::StreamExt,
};
use std::{
    collections::HashMap,
    io::{BufRead, stdin, stdout},
    sync::Arc,
    time::{Duration, Instant},
};
use zbus::{
    blocking::{Connection, Proxy},
    fdo::{DBusProxy, PropertiesProxy},
    names::OwnedBusName,
    zvariant::{OwnedObjectPath, Type, as_value::optional},
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
    state: PlayerState,
    last_unpaused: Instant,
}

#[derive(Debug, Clone)]
struct PlayerState {
    metadata: Metadata,
    identity: String,
    position: MicroDuration,
    status: PlaybackStatus,
    volume: f64,
}

#[derive(clap::Args, Debug)]
pub struct MediaArgs {
    /// Amount of time between writing to stdout
    #[arg(value_parser = humantime::parse_duration)]
    duration: Duration,
    #[arg(long, default_value_t = String::from("⏵"))]
    /// Symbol used to show playing state
    playing_symbol: String,
    #[arg(long, default_value_t = String::from("⏸"))]
    /// Symbol used to show paused state
    pause_symbol: String,
    #[arg(long, default_value_t = String::from("⏹"))]
    /// Symbol used to show stopped state
    stop_symbol: String,
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
    volume: u8,
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
        kind: UpdateKind,
    },
    Seeked {
        name: Arc<OwnedBusName>,
        position: MicroDuration,
    },
    Tick,
    PlayerRequest(RequestKind),
    Error(eyre::Report),
}
enum UpdateKind {
    Metadata(Metadata),
    PlaybackStatus(PlaybackStatus),
    Volume(f64),
}

enum RequestKind {
    Next,
    Prev,
    Play,
    SetPos(f32),
    AddVol(f32),
}
pub struct Request {
    state: PlayerState,
    kind: RequestKind,
}

#[derive(Debug, Deserialize)]
struct ButtonMessage {
    next: Option<f32>,
    prev: Option<f32>,
    play: Option<f32>,
    set_pos: Option<f32>,
    add_vol: Option<f32>,
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
    #[serde(with = "optional")]
    volume: Option<f64>,
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
            volume: 0,
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
                let mut signals = player_proxy.receive_playback_status_changed().await;
                while let Some(status) = signals.next().await {
                    let status = dbg!(status.get().await?);
                    events
                        .send(Event::UpdatePlayer {
                            name: bus.clone(),
                            kind: UpdateKind::PlaybackStatus(status),
                        })
                        .await?;
                }
                Ok(())
            },
            async {
                let mut signals = player_proxy.receive_metadata_changed().await;
                while let Some(metadata) = signals.next().await {
                    let metadata = metadata.get().await?;
                    events
                        .send(Event::UpdatePlayer {
                            name: bus.clone(),
                            kind: UpdateKind::Metadata(metadata),
                        })
                        .await?;
                }
                Ok(())
            },
            async {
                let mut signals = player_proxy.receive_volume_changed().await;
                while let Some(volume) = signals.next().await {
                    let volume = volume.get().await?;
                    events
                        .send(Event::UpdatePlayer {
                            name: bus.clone(),
                            kind: UpdateKind::Volume(volume),
                        })
                        .await?;
                }
                Ok(())
            },
            async {
                while let Ok(request) = requests.recv().await {
                    let _ = match request.kind {
                        RequestKind::AddVol(percent) => {
                            let volume = request.state.volume + (percent as f64 / 100.0);
                            player_proxy.set_volume(volume).await
                        }
                        RequestKind::SetPos(percent) => {
                            let Some(len) = request.state.metadata.length.0 else {
                                continue;
                            };
                            let delta = Duration::from_secs_f32(
                                len.as_secs_f32() * (percent / 100.0).abs(),
                            );
                            let position = if percent.is_sign_positive() {
                                request.state.position.0.map(|pos| pos + delta)
                            } else {
                                request.state.position.0.map(|pos| pos - delta)
                            };

                            let Some(position) = position else {
                                continue;
                            };

                            player_proxy
                                .set_position(
                                    &request.state.metadata.track_id,
                                    position.as_micros() as i64,
                                )
                                .await
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
//
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
    let volume = proxy.volume().await?;
    let status = proxy.playback_status().await?;

    let proxy = MediaPlayer2Proxy::new(&conn, &*bus).await?;
    let identity = proxy.identity().await?;

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
        quit: quit_sender,
        last_unpaused: Instant::now(),
        state: PlayerState {
            metadata,
            identity,
            position: position.into(),
            status,
            volume,
        },
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

    fn init(mut args: Self::Args) -> eyre::Result<Self> {
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

        args.players
            .iter_mut()
            .for_each(|player| *player = player.to_lowercase());
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
                            Event::UpdatePlayer { name, kind } => {
                                let player = self.players.get_mut(&name).ok_or_else(|| {
                                    eyre::eyre::eyre!(
                                        "received update for non existing player: {name}"
                                    )
                                })?;
                                match kind {
                                    UpdateKind::Metadata(metadata) => {
                                        player.state.metadata = metadata
                                    }
                                    UpdateKind::PlaybackStatus(status) => {
                                        player.state.status = status
                                    }
                                    UpdateKind::Volume(volume) => player.state.volume = volume,
                                }
                            }
                            Event::AddPlayer { name, player } => {
                                self.players.insert(name, player);
                            }
                            Event::RemovePlayer { name } => {
                                if let Some(player) = self.players.remove(&name) {
                                    player.quit.send(()).await?;
                                }
                            }
                            Event::Seeked { name, position } => {
                                let player = self.players.get_mut(&name).ok_or_else(|| {
                                    eyre::eyre::eyre!(
                                        "received update for non existing player: {name}"
                                    )
                                })?;
                                player.last_unpaused = Instant::now();
                                player.state.position = position;
                            }
                            Event::Error(report) => {
                                Err(report)?;
                            }
                            Event::PlayerRequest(request) => {
                                if let Some((_, player)) = self.active_player_mut() {
                                    player
                                        .requests
                                        .send(Request {
                                            state: player.state.clone(),
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
                            if message.next.is_some() {
                                tx.send(Event::PlayerRequest(RequestKind::Next)).await?;
                            }
                            if message.prev.is_some() {
                                tx.send(Event::PlayerRequest(RequestKind::Prev)).await?;
                            }
                            if message.play.is_some() {
                                tx.send(Event::PlayerRequest(RequestKind::Play)).await?;
                            }
                            if let Some(pos) = message.set_pos {
                                tx.send(Event::PlayerRequest(RequestKind::SetPos(pos)))
                                    .await?;
                            }
                            if let Some(vol) = message.add_vol {
                                tx.send(Event::PlayerRequest(RequestKind::AddVol(vol)))
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
            let position = if player.state.status == PlaybackStatus::Playing {
                player
                    .state
                    .position
                    .0
                    .map(|d| d + player.last_unpaused.elapsed())
            } else {
                player.state.position.0
            };

            let length = player.state.metadata.length.0;
            Ok(MediaFormat {
                length: format_time(length),
                position: format_time(position),
                progress: position
                    .as_ref()
                    .map(Duration::as_secs)
                    .zip(length.as_ref().map(Duration::as_secs))
                    .and_then(|(len, pos)| pos.checked_div(len))
                    .unwrap_or(0) as f32,
                title: player
                    .state
                    .metadata
                    .title
                    .replace(")", r"\)")
                    .replace("(", r"\("),
                album: player
                    .state
                    .metadata
                    .album
                    .replace(")", r"\)")
                    .replace("(", r"\("),
                artist: player
                    .state
                    .metadata
                    .artists
                    .join(", ")
                    .replace(")", r"\)")
                    .replace("(", r"\("),
                art: player
                    .state
                    .metadata
                    .art
                    .strip_prefix("file://")
                    .unwrap_or(""),
                button_symbol: self.button(player).to_string(),
                volume: (player.state.volume * 100.0) as u8,
            })
        } else {
            Ok(MediaFormat::init(&self.args.stop_symbol))
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
    pub fn init(stop_symbol: &str) -> Self {
        Self {
            length: "xx:xx".into(),
            position: "xx:xx".into(),
            progress: 0.0,
            title: String::new(),
            album: String::new(),
            artist: String::new(),
            button_symbol: String::from(stop_symbol),
            art: "",
            volume: 0,
        }
    }
}

impl Media {
    fn button(&self, active_player: &Player) -> &str {
        match active_player.state.status {
            PlaybackStatus::Playing => &self.args.playing_symbol,
            PlaybackStatus::Paused => &self.args.pause_symbol,
            PlaybackStatus::Stopped => &self.args.stop_symbol,
        }
    }
    fn active_player(&self) -> Option<(&Arc<OwnedBusName>, &Player)> {
        self.players
            .iter()
            .sorted_by_key(|(bus, player)| {
                self.args
                    .players
                    .iter()
                    .position(|prio| {
                        bus.contains(prio) | player.state.identity.to_lowercase().contains(prio)
                    })
                    .unwrap_or(usize::MAX)
            })
            .find(|(_, player)| player.state.status <= self.args.priority)
    }
    fn active_player_mut(&mut self) -> Option<(&Arc<OwnedBusName>, &mut Player)> {
        self.players
            .iter_mut()
            .sorted_by_key(|(bus, player)| {
                self.args
                    .players
                    .iter()
                    .position(|prio| {
                        bus.contains(prio) | player.state.identity.to_lowercase().contains(prio)
                    })
                    .unwrap_or(usize::MAX)
            })
            .find(|(_, player)| player.state.status <= self.args.priority)
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
