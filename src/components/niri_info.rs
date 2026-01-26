use async_channel::Sender;
use niri_ipc::{Request, Response};
use ratatui::{
    layout::{Constraint, Layout},
    widgets::Widget,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::{
    app::Meta,
    event::{AppEvent, Event},
};

pub struct NiriInfo<'a> {
    pub meta: &'a Meta,
}

impl Widget for &NiriInfo<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let window = self
            .meta
            .niri_state
            .windows
            .windows
            .values()
            .find(|window| window.is_focused);

        if let Some(window) = window {
            let format = match (&window.app_id, &window.title) {
                (None, None) => None,
                (None, Some(title)) => Some((title, title)),
                (Some(app_id), None) => Some((app_id, app_id)),
                (Some(app_id), Some(title)) => Some((app_id, title)),
            };
            if let Some((app_id, title)) = format {
                match area.height {
                    1 => {
                        format!("{app_id} | {title}").render(area, buf);
                    }
                    2 => {
                        let [app_id_rect, title_rect] = area.layout(&Layout::vertical([
                            Constraint::Fill(1),
                            Constraint::Fill(1),
                        ]));
                        app_id.as_str().render(app_id_rect, buf);
                        title.as_str().render(title_rect, buf);
                    }
                    _ => {
                        let [app_id_rect, title_rect] = area.layout(&Layout::vertical([
                            Constraint::Fill(1),
                            Constraint::Fill(1),
                        ]));
                        app_id.as_str().render(app_id_rect, buf);
                        title.as_str().render(title_rect, buf);
                    }
                }
            }
        }
    }
}
pub async fn niri_events(sender: Sender<Event>) -> color_eyre::Result<()> {
    // tokio::time::sleep(Duration::from_secs_f64(2.0)).await;
    // return Err(color_eyre::eyre::eyre!("double goob"));
    let mut buf = String::new();

    let event_stream_request = serde_json::to_vec(&Request::EventStream)?;
    let niri_adrr = std::env::var("NIRI_SOCKET")?;
    let niri_socket = tokio::net::UnixStream::connect(niri_adrr.clone()).await?;

    let mut stream_reader = {
        let mut niri_stream = niri_socket;

        niri_stream.write_all(&event_stream_request).await?;
        niri_stream.shutdown().await?;

        let mut stream_reader = BufReader::new(niri_stream);

        stream_reader.read_line(&mut buf).await?;
        let response = serde_json::from_str::<Result<Response, String>>(&buf)?;
        buf.clear();

        match response {
            Ok(Response::Handled) => stream_reader,
            Ok(response) => {
                return Err(std::io::Error::other(format!(
                    "invalid response {:#?}, expected `Response::Handled`",
                    response
                ))
                .into());
            }
            Err(err) => return Err(std::io::Error::other(err.as_str()).into()),
        }
    };

    loop {
        let _ = stream_reader.read_line(&mut buf).await;
        let response = serde_json::from_str::<niri_ipc::Event>(&buf)?;
        buf.clear();
        sender.send(Event::NiriEvent { event: response }).await?;
    }
}
