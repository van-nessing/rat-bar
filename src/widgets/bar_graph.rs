use ratatui::{
    style::Color,
    widgets::{
        Widget,
        canvas::{self, Canvas},
    },
};

pub struct BarGraph<'a> {
    pub data: &'a [f32],
}
impl<'a> BarGraph<'a> {
    const A_MAPPING: [u8; 5] = [0x0, 0x40, 0x44, 0x46, 0x47];
    const B_MAPPING: [u8; 5] = [0x0, 0x80, 0xA0, 0xB0, 0xB8];
    fn gen_char(a: u8, b: u8) -> char {
        assert!(a < 5 && b < 5);

        let base: u32 = 0x2800;

        let a = Self::A_MAPPING[a as usize];
        let b = Self::B_MAPPING[b as usize];

        let offset = a + b;
        let char = base + offset as u32;
        char::from_u32(char).unwrap()
    }
    fn chars(&'a self, height: u16) -> impl Iterator<Item = impl Iterator<Item = char>> {
        self.data.windows(2).map(move |d| {
            let mut d = d.iter().map(|d| (d * height as f32 / 100.0) as u16 * 4);
            let a = d.next().unwrap();
            let b = d.next().unwrap();
            (0..height).map(move |h| {
                let h = h * 4;
                let a = a.saturating_sub(h).min(4) as u8;
                let b = b.saturating_sub(h).min(4) as u8;
                Self::gen_char(a, b)
            })
        })
    }
    fn lines(&'a self, height: u16) -> impl Iterator<Item = String> {
        (0..height).map(move |h| {
            let h = h * 4;
            let chars = self.data.windows(2).map(|d| {
                let mut d = d.iter().map(|d| (d * height as f32 / 100.0) as u16 * 4);

                let a = d.next().unwrap();
                let b = d.next().unwrap();

                let a = a.saturating_sub(h).min(4) as u8;
                let b = b.saturating_sub(h).min(4) as u8;

                Self::gen_char(a, b)
            });
            chars.collect::<String>()
        })
    }
}
impl Widget for &BarGraph<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let canvas = Canvas::default()
            .y_bounds([0.0, 100.0])
            .x_bounds([0.0, self.data.len() as f64])
            .paint(|ctx| {
                for (i, d) in self.data.iter().enumerate() {
                    ctx.draw(&canvas::Line {
                        x1: i as f64,
                        y1: 0.0,
                        x2: i as f64,
                        y2: *d as f64,
                        color: Color::default(),
                    });
                }
            });
        canvas.render(area, buf);
        // let rows = area.rows().rev();
        // let lines = self.lines(area.height);

        // for (line, area) in lines.zip(rows) {
        //     line.render(area, buf);
        // }
    }
}
