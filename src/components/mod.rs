use ratatui::{
    layout::{Constraint, Flex, Layout, Margin, Spacing},
    widgets::{Block, StatefulWidget, Widget},
};
use serde::Deserialize;

use crate::{
    app::Meta,
    components::{
        cpu::CPU,
        date::{Date, DateWidget},
        diagnostics::Diagnostics,
        net::Net,
        niri_info::NiriInfo,
        now_playing::{NowPlaying, NowPlayingState, Preference},
        ram::RAM,
        visualizer::Visualizer,
    },
};

pub mod cpu;
pub mod date;
pub mod diagnostics;
pub mod net;
pub mod niri_info;
pub mod now_playing;
pub mod ram;
pub mod visualizer;

#[derive(Debug, Deserialize)]
pub struct BarComponent {
    #[serde(default)]
    pub constraint: Constraint,
    #[serde(default)]
    pub block: Option<ConfigBlock>,
    pub component_type: BarComponentType,
}

#[derive(Debug, Deserialize)]
pub struct ConfigBlock {
    title: String,
}

#[derive(Debug, Deserialize)]
pub enum BarComponentType {
    Group {
        #[serde(default)]
        flex: Flex,
        #[serde(default)]
        spacing: Spacing,
        components: Vec<BarComponent>,
    },
    Date(#[serde(default)] Date),
    Cpu {},
    Ram {},
    Net {
        adapter: String,
    },
    NowPlaying {
        preference: Preference,
        #[serde(default)]
        #[serde(skip)]
        state: NowPlayingState,
    },
    Diagnosticts {},
    Visualizer {},
    NiriInfo {},
}

pub struct BarComponentWidget<'a> {
    inner: &'a mut BarComponent,
    meta: &'a Meta,
}

impl BarComponent {
    pub fn constraint(&self) -> Constraint {
        self.constraint
    }
    pub fn as_widget<'a>(&'a mut self, meta: &'a Meta) -> BarComponentWidget<'a> {
        BarComponentWidget { inner: self, meta }
    }
}
impl ConfigBlock {
    pub fn to_block<'a>(&'a self) -> Block<'a> {
        Block::bordered().title(self.title.as_str())
    }
}

impl<'a> Widget for &mut BarComponentWidget<'a> {
    fn render(self, mut area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let meta = self.meta;
        if let Some(block) = self.inner.block.as_ref().map(ConfigBlock::to_block) {
            (&block).render(area, buf);
            area = block.inner(area);
        }
        match &mut self.inner.component_type {
            BarComponentType::Group {
                components,
                flex,
                spacing,
            } => {
                let spacing = spacing.clone();
                // if let Some(block) = block.as_ref().map(ConfigBlock::to_block) {
                //     (&block).render(area, buf);
                //     area = block.inner(area);
                // }
                let layout = Layout::horizontal(components.iter().map(BarComponent::constraint))
                    .flex(*flex)
                    .horizontal_margin(1)
                    .spacing(spacing);
                let rects = area.layout_vec(&layout);

                for (component, area) in components.iter_mut().zip(rects) {
                    component.as_widget(self.meta).render(area, buf);
                }
            }
            BarComponentType::Date(date) => {
                DateWidget {
                    date,
                    meta: &self.meta.time,
                }
                .render(
                    area.inner(Margin {
                        horizontal: 1,
                        vertical: 0,
                    }),
                    buf,
                );
            }
            BarComponentType::Cpu {} => {
                CPU {
                    meta: &self.meta.cpu,
                }
                .render(
                    area.inner(Margin {
                        horizontal: 1,
                        vertical: 0,
                    }),
                    buf,
                );
            }
            BarComponentType::Ram {} => {
                RAM {
                    meta: &self.meta.ram,
                }
                .render(
                    area.inner(Margin {
                        horizontal: 1,
                        vertical: 0,
                    }),
                    buf,
                );
            }
            BarComponentType::Net { adapter } => {
                Net {
                    meta: &self.meta.net,
                    adapter,
                }
                .render(
                    area.inner(Margin {
                        horizontal: 1,
                        vertical: 0,
                    }),
                    buf,
                );
            }
            BarComponentType::NowPlaying { preference, state } => {
                NowPlaying {
                    meta: &self.meta.now_playing,
                    preference,
                }
                .render(area, buf, state);
            }
            BarComponentType::Diagnosticts {} => {
                Diagnostics {
                    meta: &self.meta.diagnostics,
                }
                .render(
                    area.inner(Margin {
                        horizontal: 1,
                        vertical: 0,
                    }),
                    buf,
                );
            }
            BarComponentType::Visualizer {} => {
                Visualizer {
                    meta: &self.meta.visualizer,
                }
                .render(area, buf);
            }
            BarComponentType::NiriInfo {} => {
                NiriInfo { meta }.render(
                    area.inner(Margin {
                        horizontal: 1,
                        vertical: 0,
                    }),
                    buf,
                );
            }
        }
    }
}
