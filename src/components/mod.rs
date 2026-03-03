use ratatui::{
    layout::{Constraint, Flex, Layout, Margin, Spacing},
    widgets::{Block, StatefulWidget, Widget},
};
use serde::Deserialize;
use tokio::sync::mpsc::Sender;

use crate::{
    app::Meta,
    components::{
        diagnostics::Diagnostics,
        provider::{ProviderLayout, ProviderLayoutType, ProviderWidget},
        visualizer::Visualizer,
    },
    event::Request,
};

pub mod diagnostics;
pub mod provider;
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
    Provider {
        provider: String,
        layout: Vec<ProviderLayoutType>,
    },
    Diagnosticts {},
    Visualizer {},
}

pub struct BarComponentWidget<'a> {
    inner: &'a mut BarComponent,
    requests: &'a mut Sender<Request>,
    meta: &'a mut Meta,
}

impl BarComponent {
    pub fn constraint(&self) -> Constraint {
        self.constraint
    }
    pub fn as_widget<'a>(
        &'a mut self,
        meta: &'a mut Meta,
        requests: &'a mut Sender<Request>,
    ) -> BarComponentWidget<'a> {
        BarComponentWidget {
            inner: self,
            meta,
            requests,
        }
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
                let layout = Layout::horizontal(components.iter().map(BarComponent::constraint))
                    .flex(*flex)
                    .horizontal_margin(1)
                    .spacing(spacing);
                let rects = area.layout_vec(&layout);

                for (component, area) in components.iter_mut().zip(rects) {
                    component
                        .as_widget(self.meta, self.requests)
                        .render(area, buf);
                }
            }
            BarComponentType::Provider { provider, layout } => {
                if let Some(meta) = self.meta.provider.providers.get(provider) {
                    ProviderWidget {
                        meta,
                        images: &mut self.meta.provider.images,
                        layout: layout.as_mut_slice(),
                        requests: self.requests,
                    }
                    .render(
                        area.inner(Margin {
                            horizontal: 1,
                            vertical: 0,
                        }),
                        buf,
                    );
                }
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
        }
    }
}
