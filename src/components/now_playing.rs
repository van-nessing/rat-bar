use std::{collections::HashMap, fmt::Debug, time::Duration};

use image::{DynamicImage, load_from_memory};
use ratatui::{
    layout::{Constraint, Direction, Flex, Layout, Size},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{StatefulWidget, Widget},
};
use ratatui_image::{FilterType, Resize, picker::Picker, protocol::Protocol};
use serde::Deserialize;
use tokio::sync::mpsc::Sender;
use zbus::{
    Connection,
    fdo::DBusProxy,
    names::OwnedBusName,
    zvariant::{Array, Str, Value},
};

use crate::{
    dbus_integration::{media_player2::MediaPlayer2Proxy, player::PlayerProxy},
    event::Event,
    widgets::{
        kv_bar::{KVBar, KVBarFormat, KVPair},
        percentage_bar::BlockPercentageBar,
        scroll_text::{ScrollText, ScrollTextState},
    },
};

pub struct NowPlaying<'a> {
    pub preference: &'a Preference,
    pub meta: &'a NowPlayingMeta,
}

#[derive(Debug)]
pub struct NowPlayingMeta {
    pub players: HashMap<String, PlayerInfo>,
}
#[derive(Default)]
pub struct SongMetadata {
    pub title: String,
    pub album: String,
    pub artist: String,
    pub length: Duration,
    pub position: Duration,
    pub cover_art: Option<Protocol>,
    pub cover_url: String,
}
#[derive(Debug)]
pub struct PlayerInfo {
    pub name: String,
    pub metadata: SongMetadata,
    pub state: PlayerState,
}
#[derive(Debug, Default)]
pub struct NowPlayingState {
    pub scrolling_album_artist_text: ScrollTextState,
    pub scrolling_title_text: ScrollTextState,
}

#[derive(Debug, Deserialize)]
pub struct Preference {
    players: Vec<String>,
    #[serde(default = "bool_true")]
    allow_any: bool,
    at_least: PlayerState,
}

fn bool_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub enum PrefType {
    Paused,
    Playing,
    AnyPlaying(Vec<String>),
    AnyPaused(Vec<String>),
}
#[derive(Debug, Clone, Copy, Deserialize)]
pub enum PlayerState {
    Playing,
    Paused,
    Stopped,
}
impl PlayerState {
    pub fn filter(&self, by: PlayerState) -> bool {
        match by {
            PlayerState::Playing => matches!(self, PlayerState::Playing),
            PlayerState::Paused => matches!(self, PlayerState::Playing | PlayerState::Paused),
            PlayerState::Stopped => matches!(
                self,
                PlayerState::Playing | PlayerState::Paused | PlayerState::Stopped
            ),
        }
    }
}
impl SongMetadata {
    // pub async fn patch(&mut self, other:)
    pub async fn load_cover_art(
        picker: &Picker,
        path: &str,
    ) -> color_eyre::Result<Option<Protocol>> {
        let image = tokio::fs::read(path)
            .await
            .ok()
            .and_then(|buf| load_from_memory(&buf).ok());
        let protocol = image.and_then(|image| {
            picker
                .new_protocol(
                    image,
                    Size::new(5, 2).into(),
                    Resize::Scale(Some(FilterType::Lanczos3)),
                )
                .ok()
        });
        Ok(protocol)
    }

    pub async fn update(&mut self, picker: &Picker, other: SongMetadata) -> color_eyre::Result<()> {
        let cover_art = if self.cover_url != other.cover_url {
            Self::load_cover_art(picker, &other.cover_url).await?
        } else {
            self.cover_art.take()
        };

        *self = other;
        self.cover_art = cover_art;

        Ok(())
    }
}
impl Debug for SongMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SongMetadata")
            .field("title", &self.title)
            .field("album", &self.album)
            .field("artist", &self.artist)
            .field("length", &self.length)
            .field("position", &self.position)
            .field("cover_art", &"protocol")
            .field("cover_url", &self.cover_url)
            .finish()
    }
}
pub fn min_secs(duration: &Duration) -> (u16, u16) {
    let total_secs = duration.as_secs();
    let secs = total_secs % 60;
    let mins = total_secs / 60;

    (mins as u16, secs as u16)
}
impl StatefulWidget for &NowPlaying<'_> {
    type State = NowPlayingState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut NowPlayingState,
    ) where
        Self: Sized,
    {
        let valid_players = self
            .meta
            .players
            .values()
            .filter(|player| player.state.filter(self.preference.at_least));
        let player = self
            .preference
            .players
            .iter()
            .find_map(|pref_player| valid_players.clone().find(|info| info.name == *pref_player))
            .or_else(|| {
                if self.preference.allow_any {
                    self.meta.players.values().next()
                } else {
                    None
                }
            });
        if let Some(player_info) = player {
            let artists = [Span::raw(player_info.metadata.artist.as_str())];
            let artist = KVPair {
                key: Span::raw("ARTIST"),
                values: artists.as_slice().into(),
            };
            let title = [Span::raw(player_info.metadata.title.as_str())];
            let title = KVPair {
                key: Span::raw("TITLE"),
                values: title.as_slice().into(),
            };

            let album = [Span::raw(player_info.metadata.album.as_str())];
            let album = KVPair {
                key: Span::raw("ALBUM"),
                values: album.as_slice().into(),
            };

            match area.height {
                0 => {}
                1 => {}
                2 => {
                    let title = &player_info.metadata.title;
                    let album = player_info.metadata.album.as_str().underlined();
                    let artist = player_info.metadata.artist.as_str().underlined();

                    let graph = BlockPercentageBar {
                        style: Style::new().bg(Color::DarkGray),
                        percentage: (player_info.metadata.position.as_secs_f32()
                            / player_info.metadata.length.as_secs_f32())
                            * 100.0,
                        direction: Direction::Horizontal,
                    };
                    let (pos_min, pos_sec) = min_secs(&player_info.metadata.position);
                    let (len_min, len_sec) = min_secs(&player_info.metadata.length);

                    let time = Span::raw(format!("{pos_min}:{pos_sec:02}/{len_min}:{len_sec:02}"));
                    let playing_state = match player_info.state {
                        PlayerState::Playing => "||",
                        PlayerState::Paused => "❙❯",
                        PlayerState::Stopped => "██",
                    };
                    let icons = format!("⏵⏵ {} ⏴⏴", playing_state);

                    let title = Span::raw(title).into_centered_line();
                    let title = ScrollText { line: title };

                    let album_artist = Line::from(vec![album, Span::raw(" ― "), artist]);
                    let album_artist = ScrollText { line: album_artist };

                    let [image_area, text_area, graph_area] = area.layout(
                        &Layout::horizontal([
                            Constraint::Length(5),
                            Constraint::Percentage(60),
                            Constraint::Percentage(40),
                        ])
                        .spacing(1),
                    );

                    let [title_area, rest_area] = text_area.layout(&Layout::vertical([
                        Constraint::Length(1),
                        Constraint::Length(1),
                    ]));
                    let title_area = title_area
                        .centered_horizontally(Constraint::Length(title.line.width() as u16));

                    let rest_area = rest_area.centered_horizontally(Constraint::Length(
                        album_artist.line.width() as u16,
                    ));

                    let [info_area, graph_area] = graph_area.layout(&Layout::vertical([
                        Constraint::Length(1),
                        Constraint::Length(1),
                    ]));

                    let [icon_area, time_area] = info_area.layout(
                        &Layout::horizontal([
                            Constraint::Length(icons.len() as u16),
                            Constraint::Length(time.width() as u16),
                        ])
                        .spacing(1)
                        .flex(Flex::SpaceBetween),
                    );

                    if let Some(image) = &player_info.metadata.cover_art {
                        ratatui_image::Image::new(image).render(image_area, buf);
                    }

                    icons.render(icon_area, buf);
                    time.render(time_area, buf);
                    title.render(title_area, buf, &mut state.scrolling_title_text);
                    album_artist.render(rest_area, buf, &mut state.scrolling_album_artist_text);
                    graph.render(graph_area, buf);
                }
                _ => {
                    let pairs = [title, album, artist];
                    let bar = KVBar {
                        pairs: pairs.as_slice().into(),
                        format: KVBarFormat::Vertical,
                        delimiter: Some(":".into()),
                        spacing: 1,
                        show_keys: true,
                    };
                    bar.render(area, buf);
                }
            }
        }
    }
}
async fn get_players(connection: &Connection) -> zbus::Result<impl Iterator<Item = OwnedBusName>> {
    let dbus = DBusProxy::new(connection).await?;
    let names = dbus.list_names().await?;
    let names = names
        .into_iter()
        .filter(|name| name.starts_with("org.mpris.MediaPlayer2."));

    Ok(names)
}
pub async fn now_playing_events(sender: Sender<Event>) -> color_eyre::Result<()> {
    let connection = Connection::session().await?;
    let tick_rate = Duration::from_millis(1000);
    let mut timer = tokio::time::interval(tick_rate);
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        let destinations = get_players(&connection).await?;

        let mut players = HashMap::new();
        for destination in destinations {
            let player = PlayerProxy::new(&connection, &destination).await?;
            let media_player = MediaPlayer2Proxy::new(&connection, &destination).await?;

            let name = media_player.identity().await?;
            let player_id = destination.to_string();
            let mut metadata = player.metadata().await?;

            let title = metadata
                .remove("xesam:title")
                .map(Value::from)
                .and_then(|title| Str::try_from(title).ok().map(|str| str.to_string()));
            let album = metadata
                .remove("xesam:album")
                .map(Value::from)
                .and_then(|album| Str::try_from(album).ok().map(|str| str.to_string()));
            let artist = metadata
                .remove("xesam:artist")
                .map(Value::from)
                .and_then(|artists| {
                    Array::try_from(artists).ok().and_then(|artists| {
                        artists
                            .first()
                            .and_then(|artist| Str::try_from(artist).ok())
                            .map(|artist| artist.to_string())
                    })
                });
            let playback = player.playback_status().await?;
            let playing = match playback.as_str() {
                "Playing" => PlayerState::Playing,
                "Paused" => PlayerState::Paused,
                "Stopped" => PlayerState::Stopped,
                _ => PlayerState::Stopped,
            };
            let position = player.position().await?;
            let length = metadata
                .remove("mpris:length")
                .map(Value::from)
                .and_then(|length| i64::try_from(length).ok().map(|num| num as u64));

            let art_url = metadata
                .remove("mpris:artUrl")
                .map(Value::from)
                .and_then(|v| Str::try_from(v).ok())
                .map(|s| s.trim_start_matches("file://").to_string());
            let player_info = PlayerInfo {
                name,
                metadata: SongMetadata {
                    title: title.unwrap_or_default(),
                    album: album.unwrap_or_default(),
                    artist: artist.unwrap_or_default(),
                    length: length.map(Duration::from_micros).unwrap_or(Duration::ZERO),
                    cover_art: None,
                    cover_url: art_url.unwrap_or(String::new()),
                    position: if position.is_positive() {
                        Duration::from_micros(position as u64)
                    } else {
                        Duration::ZERO
                    },
                },
                state: playing,
            };
            players.insert(player_id, player_info);
        }

        sender.send(Event::UpdatePlayers { players }).await?;
        timer.tick().await;
    }
}
