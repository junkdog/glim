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
pub struct OpenWindow {
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


impl From<OpenWindowBuilder> for Effect {
    fn from(value: OpenWindowBuilder) -> Self {
        value.build().unwrap().into_effect()
    }
}

impl OpenWindow {
    pub fn builder() -> OpenWindowBuilder {
        OpenWindowBuilder::default()
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

    pub fn process_opening(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) {
        if let Some(open_window_fx) = self.open_window_fx.as_mut() {
            open_window_fx.process(duration, buf, area);
            if open_window_fx.done() {
                self.open_window_fx = None;
            }
        }
    }
}

impl Shader for OpenWindow {
    fn name(&self) -> &'static str {
        "open_window"
    }

    fn process(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect
    ) -> Option<Duration> {
        let remaining = match self.parent_window_fx.as_mut() {
            Some(fx) => fx.process(duration, buf, area),
            None     => None
        };

        Clear.render(area, buf);
        self.window_block().render(area, buf);

        remaining
    }

    fn execute(&mut self, _alpha: f32, _area: Rect, _cell_iter: CellIterator) {}


    fn done(&self) -> bool {
        self.open_window_fx.is_none()
            || self.open_window_fx.as_ref().is_some_and(Effect::done)
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        self.open_window_fx.as_ref()
            .map(Effect::area)
            .unwrap_or(None)
    }

    fn set_area(&mut self, area: Rect) {
        if let Some(open_window_fx) = self.open_window_fx.as_mut() {
            open_window_fx.set_area(area);
        }
    }

    fn set_cell_selection(&mut self, _strategy: CellFilter) {
        todo!()
    }
}