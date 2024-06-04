use chrono::Duration;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::Modifier;
use crate::shader::{FilterMode, Shader};

#[derive(Clone, Default)]
pub struct AddModifier {
    modifier: Modifier,
    area: Option<Rect>,
    cell_filter: FilterMode,
}

impl AddModifier {
    pub fn new(modifier: Modifier) -> Self {
        Self { modifier, ..Self::default() }
    }
}

impl Shader for AddModifier {
    fn process(&mut self, _duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        let cell_filter = self.cell_filter.selector(area);
        self.cell_iter(buf, area)
            .filter(|(pos, cell)| cell_filter.is_valid(*pos, cell))
            .for_each(|(_, c)| {
                c.set_style(c.style().add_modifier(self.modifier));
            });
        
        None
    }

    fn done(&self) -> bool {
        false
    }

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
        self.cell_filter = strategy;
    }
}