use itertools::Itertools;
use ratatui::widgets::{
    Widget,
    canvas::{Canvas, Line},
};

pub struct GraphWidget<'a> {
    pub percentages: &'a [f32],
    pub datapoint_count: usize,
}

impl<'a> Widget for GraphWidget<'a> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let canvas = Canvas::default()
            .y_bounds([0.0, 100.0])
            .x_bounds([-((self.datapoint_count - 1) as f64), 0.0])
            .paint(|ctx| {
                for (start, end) in self
                    .percentages
                    .iter()
                    .rev()
                    .enumerate()
                    .map(|(x, y)| (-(x as f64), *y as f64))
                    .tuple_windows::<(_, _)>()
                {
                    ctx.draw(&Line {
                        x1: start.0,
                        y1: start.1,
                        x2: end.0,
                        y2: end.1,
                        color: ratatui::style::Color::White,
                    });
                }
            });
        canvas.render(area, buf);
    }
}
