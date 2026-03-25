use crate::{
    Provider,
    media::dbus::{
        media_player2::MediaPlayer2Proxy,
        player::{Metadata, MicroDuration, PlaybackStatus, PlayerProxy},
    },
};
use color_eyre as eyre;
use futures_concurrency::prelude::*;
use serde::{Deserialize, Serialize};
use smol::{
    channel::{Receiver, Sender},
    stream::StreamExt,
};
use std::{
    collections::HashMap,
    io::stdout,
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
    rx: Receiver<Event>,
    duration: Duration,
    prefer: PlaybackStatus,
    priority: Vec<String>,
    players: HashMap<Arc<OwnedBusName>, Player>,
}

#[derive(Debug)]
struct Player {
    // listener: Task<eyre::Result<()>>,
    quit: Sender<()>,
    metadata: Metadata,
    identity: String,
    position: MicroDuration,
    last_unpaused: Instant,
    status: PlaybackStatus,
}

#[derive(clap::Args)]
pub struct MediaArgs {
    /// Amount of time between writing to stdout
    #[arg(value_parser = humantime::parse_duration)]
    duration: Duration,
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
    title: &'a str,
    album: &'a str,
    artist: String,
    art: &'a str,
    buttons: String,
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
    Error(eyre::Report),
}

#[derive(Deserialize, Type, Debug)]
struct Signal<'a> {
    _address: &'a str,
    data: Data,
    _f3: Vec<&'a str>,
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
            title: "",
            album: "",
            artist: String::new(),
            buttons: String::from("⏵⏵ ██ ⏴⏴"),
            art: "",
        }
    }
}

async fn listen_player(
    conn: Arc<zbus::Connection>,
    bus: Arc<OwnedBusName>,
    events: Sender<Event>,
    quit: Receiver<()>,
) {
    let result = (
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
            let signals = PlayerProxy::new(&conn, &*bus).await?;
            let mut signals = signals.receive_seeked().await?;
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
            quit.recv().await?;
            Ok(())
        },
    )
        .race()
        .await;
    if let Err(report) = result {
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
    smol::spawn(listen_player(conn, bus, events, quit_receiver)).detach();
    Ok(Player {
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
        let (tx, rx) = smol::channel::bounded(8);
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
            rx,
            players,
            duration: args.duration,
            priority: vec!["cider".into(), "firefox".into()],
            prefer: PlaybackStatus::Paused,
        })
    }
    fn run(mut self) -> eyre::Result<()> {
        smol::block_on(async {
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
                            eyre::eyre::eyre!("received update for non existing player: {name}")
                        })?;
                        if let Some(metadata) = data.metadata {
                            player.metadata = metadata;
                        }
                        if let Some(status) = data.playback_status {
                            match status {
                                PlaybackStatus::Playing => player.last_unpaused = Instant::now(),
                                PlaybackStatus::Paused | PlaybackStatus::Stopped => {
                                    player.position.0 += player.last_unpaused.elapsed()
                                }
                            }
                            player.status = status;
                        }
                    }
                    Event::Seeked { name, position } => {
                        let player = self.players.get_mut(&name).ok_or_else(|| {
                            eyre::eyre::eyre!("received update for non existing player: {name}")
                        })?;
                        player.last_unpaused = Instant::now();
                        player.position = position;
                    }
                    Event::Error(report) => {
                        Err(report)?;
                    }
                    Event::Tick => (),
                }
            }
        })
    }
    fn format<'a>(&'a self) -> eyre::Result<Self::Fmt<'a>> {
        let mut valid_players = self
            .players
            .iter()
            .filter(|(_name, player)| player.status <= self.prefer);

        let player = self
            .priority
            .iter()
            .find_map(|name| {
                valid_players.clone().find(|(bus, p)| {
                    p.identity.to_lowercase().contains(name.as_str())
                        | bus
                            .strip_prefix("org.mpris.MediaPlayer2.")
                            .unwrap()
                            .contains(name.as_str())
                })
            })
            .or_else(|| valid_players.next());

        if let Some((_, player)) = player {
            let position = if player.status == PlaybackStatus::Playing {
                player.position.0 + player.last_unpaused.elapsed()
            } else {
                player.position.0
            };

            let length = player.metadata.length.0;
            Ok(MediaFormat {
                length: format_time(length),
                position: format_time(position),
                progress: ((100 * position.as_secs())
                    .checked_div(length.as_secs())
                    .unwrap_or_default()) as f32,
                title: &player.metadata.title,
                album: &player.metadata.album,
                artist: player.metadata.artists.join(", "),
                art: player.metadata.art.strip_prefix("file://").unwrap_or(""),
                buttons: format!("⏵⏵ {} ⏴⏴", player.status.button()),
            })
        } else {
            Ok(MediaFormat::default())
        }
    }
    fn update(&mut self) -> eyre::Result<()> {
        unreachable!()
    }
    fn duration(&self) -> Option<std::time::Duration> {
        Some(self.duration)
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

fn format_time(duration: Duration) -> String {
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
}
