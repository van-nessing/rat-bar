use std::str::FromStr;

use ratatui::layout::Constraint as RatConstraint;
use ratatui::layout::Position;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Widget;
use serde_json::json;

use crate::app::State;
use crate::event::Request;
use crate::layout::Direction;
use crate::layout::ProviderMessage;
use crate::layout::style::interpolate_string;
use crate::layout::{Constraint, ElementWidget};
use crate::widgets::percentage_bar::BlockPercentageBar;

#[derive(knuffel::Decode)]
pub struct Bar {
    #[knuffel(property, str, default)]
    width: Constraint,
    #[knuffel(argument, str)]
    direction: Direction,
    #[knuffel(property)]
    var: String,
    #[knuffel(property)]
    fg: String,
    #[knuffel(property)]
    bg: String,
    #[knuffel(property, str)]
    on_click: Option<ProviderMessage>,
    #[knuffel(property, str)]
    on_scroll: Option<ProviderMessage>,
}

impl ElementWidget for Bar {
    fn width(&self, _state: &crate::app::State) -> RatConstraint {
        self.width.into()
    }

    fn height(&self) -> RatConstraint {
        RatConstraint::Fill(1)
    }

    fn on_click(
        &self,
        ProviderMessage { provider, message }: ProviderMessage,
        area: ratatui::prelude::Rect,
        click_position: Position,
        _dir: f32,
        state: &mut State,
    ) {
        let percentage = state
            .providers
            .variables
            .get(&self.var)
            .and_then(|var| var.as_f64())
            .unwrap_or(0.0) as f32;
        let new_percentage = match self.direction {
            Direction::Horizontal => (click_position.x - area.x) as f32 * 100.0 / area.width as f32,
            Direction::Vertical => {
                (area.height - (click_position.y - area.y)) as f32 * 100.0 / area.height as f32
            }
        };
        let difference = new_percentage - percentage;
        let _ = state.requests.try_send(Request::MessageProvider {
            provider: provider.to_string(),
            message: serde_json::to_string(&json!({message: difference})).unwrap(),
        });
    }
}
impl StatefulWidget for &mut Bar {
    type State = State;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let percentage = state
            .providers
            .variables
            .get(&self.var)
            .and_then(|var| var.as_f64())
            .unwrap_or(0.0) as f32;
        let fg = interpolate_string(&self.fg, state);
        let bg = interpolate_string(&self.bg, state);

        let fg = Color::from_str(&fg).unwrap_or(Color::White);
        let bg = Color::from_str(&bg).unwrap_or(Color::DarkGray);

        let style = Style::new().fg(fg).bg(bg);

        self.interact(self.on_click.clone(), self.on_scroll.clone(), area, state);

        BlockPercentageBar {
            style,
            percentage,
            direction: self.direction.into(),
        }
        .render(area, buf);
    }
}
