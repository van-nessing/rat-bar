use ratatui::layout::Constraint as RatConstraint;
use ratatui::layout::Flex;
use ratatui::layout::Layout;
use ratatui::widgets::StatefulWidget;

use crate::app::State;
use crate::layout::Direction;
use crate::layout::{Constraint, Element, ElementWidget};

#[derive(knuffel::Decode)]
pub struct Group {
    #[knuffel(argument, str)]
    direction: Direction,
    #[knuffel(property, str, default)]
    width: Option<Constraint>,
    #[knuffel(property, str, default = Flex::SpaceBetween)]
    flex: Flex,
    #[knuffel(property, default = true)]
    center: bool,
    #[knuffel(property, default = 0)]
    spacing: u16,
    #[knuffel(children)]
    elements: Vec<Element>,
}

impl ElementWidget for Group {
    fn width(&self, state: &crate::app::State) -> ratatui::prelude::Constraint {
        match (self.direction, self.width) {
            (_, Some(width)) => width.into(),
            (Direction::Vertical, None) => self
                .elements
                .iter()
                .map(|e| e.width(state))
                .try_fold(0, |acc, c| {
                    if let RatConstraint::Length(length) = c {
                        Some(length.max(acc))
                    } else {
                        None
                    }
                })
                .map(RatConstraint::Length)
                .unwrap_or(RatConstraint::Fill(1)),
            (Direction::Horizontal, None) => RatConstraint::Fill(1),
            // (Direction::Horizontal, None) => self
            //     .elements
            //     .iter()
            //     .map(|e| e.width(state))
            //     .try_fold(0, |acc, c| {
            //         if let RatConstraint::Length(length) = c {
            //             Some(length + acc)
            //         } else {
            //             None
            //         }
            //     })
            //     .map(|sum| sum + (self.elements.len().saturating_sub(1) as u16 * self.spacing))
            //     .map(RatConstraint::Length)
            //     .unwrap_or(RatConstraint::Fill(1)),
        }
    }

    fn height(&self) -> ratatui::prelude::Constraint {
        RatConstraint::Fill(1)
    }
}

impl StatefulWidget for &mut Group {
    type State = State;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        match self.direction {
            Direction::Horizontal => {
                let constraints = self.elements.iter().map(|e| e.width(state));
                let layout = area.layout_vec(
                    &Layout::horizontal(constraints)
                        .spacing(self.spacing)
                        .flex(self.flex),
                );
                for (area, element) in layout.into_iter().zip(self.elements.iter_mut()) {
                    element.render(area, buf, state);
                }
            }
            Direction::Vertical => {
                let constraints = self.elements.iter().map(|e| e.height());
                let layout = area.layout_vec(&Layout::vertical(constraints));
                for (mut area, element) in layout.into_iter().zip(self.elements.iter_mut()) {
                    if self.center {
                        area = area.centered_horizontally(element.width(state));
                    }
                    element.render(area, buf, state);
                }
            }
        }
    }
}
