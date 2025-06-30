mod pipeline_table;
mod projects_table;
mod internal_logs;
mod shortcuts;
mod notification;
mod input_field;

use chrono::{DateTime, Local};
use ratatui::prelude::{Line, Text};
pub use pipeline_table::*;
pub use projects_table::*;
pub use internal_logs::*;
pub use shortcuts::*;
pub use notification::*;
pub use input_field::*;
use crate::theme::theme;


pub fn text_from(date: DateTime<Local>) -> Text<'static> {
    Text::from(vec![
        Line::from(date.format("%a, %d %b").to_string())
            .style(theme().date),
        Line::from(date.format("%H:%M:%S").to_string())
            .style(theme().time),
    ])
}