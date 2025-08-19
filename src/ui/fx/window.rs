use derive_builder::Builder;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, Widget},
};

use crate::ui::widget::Shortcuts;

#[derive(Builder, Clone, Debug)]
#[builder(pattern = "owned")]
pub struct PopupWindow {
    title: Line<'static>,
    border_style: Style,
    border_type: BorderType,
    background: Style,

    #[builder(default)]
    shortcuts: Option<Shortcuts<'static>>,
}

impl PopupWindow {
    pub fn builder() -> PopupWindowBuilder {
        PopupWindowBuilder::default()
    }

    fn window_block(&self) -> Block<'_> {
        let w = Block::new()
            .borders(Borders::ALL)
            .title_style(self.border_style)
            .title(self.title.clone())
            .border_style(self.border_style)
            .border_type(self.border_type)
            .style(self.background);

        match self.shortcuts.as_ref() {
            Some(shortcuts) => w.title_bottom(shortcuts.as_line()),
            None => w,
        }
    }

    // pub fn process_opening(&mut self, _duration: Duration, _buf: &mut Buffer, _area: Rect)
    // {     // TODO: Implement opening effects - currently disabled for refactoring
    // }
}

impl Widget for PopupWindow {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        Clear.render(area, buf);
        self.window_block().render(area, buf);
    }
}

// impl Shader for PopupWindow {
//     fn name(&self) -> &'static str {
//         "open_window"
//     }
//
//     fn process(&mut self, _duration: Duration, buf: &mut Buffer, area: Rect) ->
// Option<Duration> {         // TODO: Implement parent window effects - currently
// disabled for refactoring         Clear.render(area, buf);
//         self.window_block().render(area, buf);
//         None
//     }
//
//     fn done(&self) -> bool {
//         // TODO: Implement proper done check - currently always true for refactoring
//         false
//     }
//
//     fn clone_box(&self) -> Box<dyn Shader> {
//         Box::new(self.clone())
//     }
//
//     fn area(&self) -> Option<Rect> {
//         None
//     }
//
//     fn set_area(&mut self, _area: Rect) {
//     }
//
//     fn filter(&mut self, filter: CellFilter) {
//         self.filter = Some(filter);
//     }
// }
