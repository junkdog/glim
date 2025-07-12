mod input_field;
mod notification;
mod pipeline_table;
mod projects_table;
mod shortcuts;

use std::{cell::RefCell, rc::Rc};

use chrono::{DateTime, Local};
pub use input_field::*;
pub use notification::*;
pub use pipeline_table::*;
pub use projects_table::*;
use ratatui::{
    layout::{Position, Rect},
    prelude::{Line, Text},
};
pub use shortcuts::*;

use crate::theme::theme;

pub fn text_from(date: DateTime<Local>) -> Text<'static> {
    Text::from(vec![
        Line::from(date.format("%a, %d %b").to_string()).style(theme().date),
        Line::from(date.format("%H:%M:%S").to_string()).style(theme().time),
    ])
}

#[derive(Clone, Debug, Default)]
pub struct RefRect {
    rect: Rc<RefCell<Rect>>,
}

impl RefRect {
    pub fn new(rect: Rect) -> Self {
        Self { rect: Rc::new(RefCell::new(rect)) }
    }

    pub fn get(&self) -> Rect {
        *self.rect.borrow()
    }

    pub fn set(&self, rect: Rect) {
        *self.rect.borrow_mut() = rect;
    }

    pub fn contains(&self, position: Position) -> bool {
        self.rect.borrow().contains(position)
    }
}
