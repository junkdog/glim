use chrono::Duration;
use ratatui::buffer::Buffer;
use ratatui::layout::{Rect};
use crate::shader::effect::FilterMode;
use crate::shader::fx::internal::MappedColor;
use crate::shader::shader::Shader;
use crate::ui::popup::AsIndexedColor;


#[derive(Clone, Default)]
pub struct Ansi256 {
    area: Option<Rect>,
}

impl Shader for Ansi256 {
    fn process(
        &mut self,
        _duration: Duration,
        buf: &mut Buffer,
        area: Rect,
    ) -> Option<Duration> {
        let mut fg_mapper = MappedColor::default();
        let mut bg_mapper = MappedColor::default();
        
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let cell = buf.get_mut(x, y);
                let fg = fg_mapper.map(cell.fg, 0.0, |c| c.as_indexed_color());
                let bg = bg_mapper.map(cell.bg, 0.0, |c| c.as_indexed_color());

                cell.set_fg(fg);
                cell.set_bg(bg);
            }
        }

        None
    }

    fn done(&self) -> bool { false }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        self.area
    }

    fn set_area(&mut self, area: Rect) {
        self.area = Some(area);
    }

    fn cell_selection(&mut self, strategy: FilterMode) {
        todo!()
    }
}