mod input_field;
mod notification;
mod pipeline_table;
mod projects_table;
mod shortcuts;

use chrono::{DateTime, Local};
pub use input_field::*;
pub use notification::*;
pub use pipeline_table::*;
pub use projects_table::*;
use ratatui::prelude::{Line, Text};
pub use shortcuts::*;

use crate::theme::theme;

pub fn text_from(date: DateTime<Local>) -> Text<'static> {
    Text::from(vec![
        Line::from(date.format("%a, %d %b").to_string()).style(theme().date),
        Line::from(date.format("%H:%M:%S").to_string()).style(theme().time),
    ])
}
