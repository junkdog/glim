use chrono::Duration;

pub mod popup;
pub mod widget;
pub mod fx;
mod stateful_widgets;

pub use stateful_widgets::StatefulWidgets;

pub fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.abs().num_seconds();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    match () {
        _ if hours > 0   => format!("{}:{:02}:{:02}", hours, minutes, seconds),
        _ if minutes > 0 => format!("{}:{:02}", minutes, seconds),
        _                => format!("0:{:02}", seconds),
    }
}