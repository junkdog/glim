use derive_builder::Builder;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, BorderType, Clear};
use ratatui::widgets::Widget;
use tachyonfx::{CellFilter, CellIterator, Duration, Effect, IntoEffect, Shader};
use crate::ui::widget::Shortcuts;

#[derive(Builder, Clone)]
#[builder(pattern = "owned")]
pub struct PopupWindow {
    title: Line<'static>,
    #[builder(default, setter(strip_option))]
    open_window_fx: Option<Effect>,
    #[builder(default, setter(strip_option))]
    parent_window_fx: Option<Effect>,
    border_style: Style,
    border_type: BorderType,
    background: Style,

    #[builder(default)]
    shortcuts: Option<Shortcuts<'static>>
}


impl From<PopupWindowBuilder> for Effect {
    fn from(value: PopupWindowBuilder) -> Self {
        value.build().unwrap().into_effect()
    }
}

impl PopupWindow {
    pub fn builder() -> PopupWindowBuilder {
        PopupWindowBuilder::default()
    }

    pub fn screen_area(&mut self, area: Rect) {
        if let Some(fx) = self.parent_window_fx.as_mut() {
            fx.set_area(area);
        }
    }

    fn window_block(&self) -> Block {
        let w = Block::new()
            .borders(Borders::ALL)
            .title_style(self.border_style)
            .title(self.title.clone())
            .border_style(self.border_style)
            .border_type(self.border_type)
            .style(self.background);

        match self.shortcuts.as_ref() {
            Some(shortcuts) => w.title_bottom(shortcuts.as_line()),
            None            => w,
        }
    }

    pub fn process_opening(&mut self, _duration: Duration, _buf: &mut Buffer, _area: Rect) {
        // TODO: Implement opening effects - currently disabled for refactoring
    }
}

impl Shader for PopupWindow {
    fn name(&self) -> &'static str {
        "open_window"
    }

    fn process(
        &mut self,
        _duration: Duration,
        buf: &mut Buffer,
        area: Rect
    ) -> Option<Duration> {
        // TODO: Implement parent window effects - currently disabled for refactoring
        Clear.render(area, buf);
        self.window_block().render(area, buf);
        None
    }

    fn execute(&mut self, _alpha: f32, _area: Rect, _cell_iter: CellIterator) {}


    fn done(&self) -> bool {
        // TODO: Implement proper done check - currently always true for refactoring
        true
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        // TODO: Implement proper area calculation - currently returns None for refactoring
        None
    }

    fn set_area(&mut self, _area: Rect) {
        // TODO: Implement proper area setting - currently no-op for refactoring
    }

    fn set_cell_selection(&mut self, _strategy: CellFilter) {
        todo!()
    }
}