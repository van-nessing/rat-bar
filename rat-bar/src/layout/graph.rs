use std::str::FromStr;

use ratatui::prelude::Widget;
use ratatui::style::Color;
use ratatui::{layout::Constraint as RatConstraint, widgets::StatefulWidget};

use crate::app::State;
use crate::layout::style::interpolate_string;
use crate::layout::{Constraint, ElementWidget, ProviderMessage};
use crate::widgets::bar_graph::{BarGraph, Marker};

#[derive(knuffel::Decode)]
pub struct Graph {
    #[knuffel(property, str, default)]
    width: Constraint,
    #[knuffel(property, str, default)]
    marker: Marker,
    #[knuffel(property, default)]
    fill: bool,
    #[knuffel(property)]
    var: String,
    #[knuffel(property)]
    fg: String,
    #[knuffel(property, str)]
    on_click: Option<ProviderMessage>,
    #[knuffel(property, str)]
    on_scroll: Option<ProviderMessage>,
}

impl ElementWidget for Graph {
    fn width(&self, _state: &crate::app::State) -> RatConstraint {
        self.width.into()
    }

    fn height(&self) -> RatConstraint {
        RatConstraint::Fill(1)
    }
}

impl StatefulWidget for &mut Graph {
    type State = State;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        if let Some(data) = state
            .providers
            .variables
            .get(&self.var)
            .and_then(|var| var.as_array())
            .and_then(|val| {
                val.iter()
                    .map(|val| val.as_f64().map(|val| val as f32))
                    .collect::<Option<Vec<_>>>()
            })
        {
            let fg = interpolate_string(&self.fg, state);
            let color = Color::from_str(&fg).unwrap_or(Color::White);

            self.interact(self.on_click.clone(), self.on_scroll.clone(), area, state);

            BarGraph {
                percentages: data.as_slice(),
                datapoint_count: data.len(),
                color,
                fill: self.fill,
                marker: self.marker,
            }
            .render(area, buf);
        }
    }
}
