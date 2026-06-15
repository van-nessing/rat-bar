use std::convert::Infallible;

use crate::{
    app::State,
    layout::{
        Constraint, ElementWidget, ProviderMessage,
        style::{ParseStyle, interpolate_string, style_string},
    },
    widgets::scroll_text::{ScrollText, ScrollTextState},
};
use ratatui::{layout::Constraint as RatConstraint, style::Style, widgets::StatefulWidget};

#[derive(knuffel::Decode)]
pub struct Text {
    #[knuffel(argument)]
    string: String,
    #[knuffel(property, str)]
    width: Option<Constraint>,
    #[knuffel(property, default = true)]
    scroll: bool,
    #[knuffel(property, str)]
    on_click: Option<ProviderMessage>,
    #[knuffel(property, str)]
    on_scroll: Option<ProviderMessage>,
    #[knuffel(property, str, default = Some(ParseStyle(Style::new().reversed())))]
    click_style: Option<ParseStyle>,
    scroll_state: ScrollTextState,
}
struct TextString(String);
impl std::str::FromStr for TextString {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(TextString(s.replace(' ', " ")))
    }
}

impl ElementWidget for Text {
    fn width(&self, state: &crate::app::State) -> RatConstraint {
        self.width.map(RatConstraint::from).unwrap_or_else(|| {
            let string = interpolate_string(&self.string, state);
            RatConstraint::Length(style_string(string.into()).width() as u16)
        })
    }

    fn height(&self) -> RatConstraint {
        RatConstraint::Length(1)
    }
}

impl StatefulWidget for &mut Text {
    type State = State;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let string = interpolate_string(&self.string, state);
        let line = style_string(string.into());

        ScrollText { line }.render(area, buf, &mut self.scroll_state);

        self.interact(self.on_click.clone(), self.on_scroll.clone(), area, state);
        self.apply_click_style(buf, area, self.click_style.as_deref().copied(), state);
    }
}
