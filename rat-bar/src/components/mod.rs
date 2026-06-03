use crossterm::event::MouseEvent;
use ratatui::{
    layout::{Constraint, Flex, Layout, Margin, Position, Spacing},
    widgets::{Block, StatefulWidget, Widget},
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::{
    app::{Meta, State},
    components::{diagnostics::Diagnostics, provider::ProviderLayoutType},
    event::Request,
};

pub mod diagnostics;
pub mod provider;

#[derive(Debug, Deserialize, Serialize)]
pub struct BarComponent {
    #[serde(default)]
    pub constraint: Constraint,
    #[serde(default)]
    pub block: Option<ConfigBlock>,
    pub layout: Vec<ProviderLayoutType>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigBlock {
    title: String,
}

impl ConfigBlock {
    pub fn to_block<'a>(&'a self) -> Block<'a> {
        Block::bordered().title(self.title.as_str())
    }
}

impl<'a> StatefulWidget for &'a mut BarComponent {
    type State = State;

    fn render(
        self,
        mut area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        if let Some(block) = self.block.as_ref().map(ConfigBlock::to_block) {
            (&block).render(area, buf);
            area = block.inner(area);
        }
        if area.height == 0 {
            return;
        }
        // let layout = self.layout.get_mut(area.height as usize - 1);
        let layout = &mut self.layout;

        let layout = if let Some(layout) = layout.get_mut(area.height as usize - 1) {
            layout
        } else {
            layout.last_mut().expect("layout should not be empty")
        };
        layout.render(area, buf, state);
    }
}
