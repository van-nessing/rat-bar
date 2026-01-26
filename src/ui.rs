use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use serde::Deserialize;

use crate::{app::App, components::BarComponent};

#[derive(Deserialize)]
pub struct Ui {
    pub component: BarComponent,
}

impl Widget for &mut App {
    /// Renders the user interface widgets.
    ///
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui/ratatui/tree/master/examples
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.ui.component.as_widget(&self.meta).render(area, buf);
    }
}
