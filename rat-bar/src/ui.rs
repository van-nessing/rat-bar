use ratatui::{
    buffer::Buffer,
    layout::{Layout, Rect},
    widgets::{StatefulWidget, Widget},
};

use crate::{
    app::App,
    layout::{BarElement, ElementWidget},
};

pub struct Ui {
    pub components: Vec<BarElement>,
}

impl Widget for &mut App {
    /// Renders the user interface widgets.
    ///
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui/ratatui/tree/master/examples
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = area.layout_vec(&Layout::horizontal(
            self.ui.components.iter().map(|component| component.width),
        ));
        for (component, area) in self.ui.components.iter_mut().zip(layout.into_iter()) {
            component.render(area, buf, &mut self.state);
        }
        self.state.mouse.events.clear();
    }
}
