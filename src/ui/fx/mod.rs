mod window;

pub use window::*;
pub use crate::effects::{create_window_title};

// TODO: Re-implement open_window function that returns PopupWindow with proper effects
use crate::ui::widget::Shortcuts;

pub fn open_window(
    title: &'static str,
    shortcuts: Option<Vec<(&'static str, &'static str)>>,
) -> PopupWindow {
    // Temporarily create a PopupWindow without effects for refactoring
    PopupWindow::builder()
        .title(create_window_title(title))
        .border_style(crate::theme::theme().border.config_border)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .background(crate::theme::theme().background)
        .shortcuts(shortcuts.map(Shortcuts::from))
        .build()
        .unwrap()
}