use chrono::Duration;
use compact_str::{format_compact, CompactString};

pub mod fx;
pub mod popup;
mod stateful_widgets;
pub mod widget;

pub use stateful_widgets::StatefulWidgets;

pub fn format_duration(duration: Duration) -> CompactString {
    let total_seconds = duration.abs().num_seconds();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    match () {
        _ if hours > 0 => format_compact!("{}:{:02}:{:02}", hours, minutes, seconds),
        _ if minutes > 0 => format_compact!("{}:{:02}", minutes, seconds),
        _ => format_compact!("0:{:02}", seconds),
    }
}
