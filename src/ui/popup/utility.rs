use ratatui::layout::Rect;
use ratatui::prelude::Color;

pub trait CenteredShrink {
    fn inner_centered(&self, width: u16, height: u16) -> Rect;
}

impl CenteredShrink for Rect {
    fn inner_centered(&self, width: u16, height: u16) -> Rect {
        let x = self.x + (self.width.saturating_sub(width) / 2);
        let y = self.y + (self.height.saturating_sub(height) / 2);
        Rect::new(x, y, width.min(self.width), height.min(self.height))
    }
}

pub trait AsIndexedColor {
    fn as_indexed_color(&self) -> Color;
}

impl AsIndexedColor for Color {
    fn as_indexed_color(&self) -> Color {
        match self {
            Color::Rgb(ri, gi, bi) => {
                let c = colorsys::Rgb::from([*ri as f64, *gi as f64, *bi as f64]);
                let ansi256 = colorsys::Ansi256::from(c);
                Color::Indexed(ansi256.code())
            }
            _ => *self
        }
    }
}