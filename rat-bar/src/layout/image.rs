use std::str::FromStr;

use ratatui::layout::{Constraint as RatConstraint, Size};
use ratatui::style::{Color, Style};
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Widget as _;

use crate::app::State;
use crate::event::Request;
use crate::layout::style::interpolate_string;
use crate::layout::{ElementWidget, ProviderMessage};

#[derive(knuffel::Decode)]
pub struct Image {
    #[knuffel(argument)]
    var: String,
    #[knuffel(property)]
    width: u16,
    #[knuffel(property)]
    bg: Option<String>,
    #[knuffel(property, str)]
    on_click: Option<ProviderMessage>,
    #[knuffel(property, str)]
    on_scroll: Option<ProviderMessage>,
}

impl ElementWidget for Image {
    fn width(&self, _state: &crate::app::State) -> RatConstraint {
        self.width.into()
    }

    fn height(&self) -> RatConstraint {
        RatConstraint::Fill(1)
    }
}

impl StatefulWidget for &mut Image {
    type State = State;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        if let Some(bg) = &self.bg {
            let color = interpolate_string(bg, state);
            if let Ok(color) = Color::from_str(&color) {
                buf.set_style(area, Style::new().bg(color));
            }
        }
        self.interact(self.on_click.clone(), self.on_scroll.clone(), area, state);

        if let Some(path) = &state.providers.variables.get(&self.var) {
            let path = path.as_str().unwrap();
            // image is present
            if let Some(access) = state.providers.images.get_mut(path) {
                // image finished loading
                if let Some(protocol) = access.get() {
                    ratatui_image::Image::new(protocol).render(area, buf);
                }
            } else {
                let _ = state.requests.try_send(Request::LoadImage {
                    path: path.to_string(),
                    size: Size::new(self.width, area.height),
                });
            }
        }
    }
}
