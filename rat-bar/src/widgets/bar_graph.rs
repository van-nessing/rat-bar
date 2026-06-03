use itertools::Itertools;
use ratatui::{
    layout::Position,
    style::Color,
    symbols::{braille::BRAILLE, pixel::OCTANTS},
    widgets::{
        Widget,
        canvas::{Canvas, Line},
    },
};
use serde::{Deserialize, Serialize};
use strum::EnumString;

pub struct BarGraph<'a> {
    pub percentages: &'a [f32],
    pub datapoint_count: usize,
    pub color: Color,
    pub marker: Marker,
    pub fill: bool,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, EnumString)]
pub enum Marker {
    #[default]
    Braille,
    Octant,
}

impl From<Marker> for ratatui::symbols::Marker {
    fn from(value: Marker) -> Self {
        match value {
            Marker::Braille => ratatui::symbols::Marker::Braille,
            Marker::Octant => ratatui::symbols::Marker::Octant,
        }
    }
}

impl<'a> BarGraph<'a> {
    const INDICES: [[u8; 5]; 2] = [
        [0b0000000, 0b01000000, 0b01010000, 0b01010100, 0b01010101],
        [0b0000000, 0b10000000, 0b10100000, 0b10101000, 0b10101010],
    ];
    fn gen_char(a: u8, b: u8, marker: Marker) -> char {
        let a: u8 = Self::INDICES[0][a as usize];
        let b: u8 = Self::INDICES[1][b as usize];
        match marker {
            Marker::Braille => BRAILLE[(a + b) as usize],
            Marker::Octant => OCTANTS[(a + b) as usize],
        }
    }
}
impl Widget for &BarGraph<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        if self.fill {
            let interpolate = |pos: f32| {
                let index = pos * self.datapoint_count as f32;
                let fract = index.fract();
                let whole = index.floor() as usize;
                let start = self.percentages[whole];
                let end = self.percentages.get(whole + 1).copied().unwrap_or(start);
                start + (end - start) * fract
            };
            let values_of_pos = |pos: u16| {
                let to_percent = |p| p / (area.width as f32 * 2.0 + 1.0);
                let pos = pos as f32;
                let a_pos = interpolate(to_percent(pos * 2.0));
                let b_pos = interpolate(to_percent(pos * 2.0 + 1.0));

                (a_pos, b_pos)
            };

            for position in area.positions() {
                let relative =
                    Position::new(position.x - area.x, area.height - (position.y - area.y) - 1);
                let cell_height = 100.0 / area.height as f32;
                let cell_start = (relative.y as f32) * cell_height;
                let get_char_value =
                    |val: f32| ((val - cell_start) / cell_height).clamp(0.0, 1.0) * 4.0;
                let (a_val, b_val) = values_of_pos(relative.x);
                let a_val = get_char_value(a_val);
                let b_val = get_char_value(b_val);
                let char = BarGraph::gen_char(a_val.ceil() as u8, b_val.ceil() as u8, self.marker);

                buf[position].set_char(char).set_fg(self.color);
            }
        } else {
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
                            color: self.color,
                        });
                    }
                })
                .marker(self.marker.into());
            canvas.render(area, buf);
        }
    }
}
