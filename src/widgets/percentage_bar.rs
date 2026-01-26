use std::iter;

use ratatui::{
    layout::Direction,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};

pub struct BlockPercentageBar {
    pub style: Style,
    pub percentage: f32,
    pub direction: Direction,
}
pub struct LinePercentageBar {
    pub style: Style,
    pub percentage: f32,
    pub direction: Direction,
}
impl Widget for &BlockPercentageBar {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let gen_chars = |parts: &[char], len: u16| {
            let parts_len = parts.len() as u16;
            let total_divisions = len * parts_len;

            let active_divisions = (total_divisions as f32 * (self.percentage / 100.0)) as u16;

            let remaining_character = (active_divisions % parts_len) as usize;
            let character_count = (active_divisions / parts_len) as usize;
            // total_divisions.to_string().render(area, buf);

            iter::repeat_n(*parts.last().unwrap(), character_count)
                .chain(iter::once(parts[remaining_character]))
        };
        buf.set_style(area, self.style);
        match self.direction {
            Direction::Horizontal => {
                let parts = ['▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];
                let len = area.width;
                let chars = gen_chars(&parts, len);
                let string = chars.collect::<String>();
                let line = Line::raw(&string).style(self.style);

                for row in area.rows() {
                    (&line).render(row, buf);
                }
            }
            Direction::Vertical => {
                let parts = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
                let len = area.height;
                let chars = gen_chars(&parts, len);
                let mut line = String::with_capacity(area.width as usize);
                for (row, char) in area.rows().rev().zip(chars) {
                    line.clear();

                    let chars = iter::repeat_n(char, row.width as usize);
                    line.extend(chars);

                    Span::raw(line.as_str()).style(self.style).render(row, buf);
                }
            }
        }
    }
}
