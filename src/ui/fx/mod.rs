mod window;

use ratatui::prelude::{Line, Span};
pub use window::*;

use crate::{theme::theme, ui::widget::Shortcuts};

pub fn popup_window(
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

/// Creates a popup window title line with effects (TODO: integrate with proper window
/// system)
fn create_window_title(title: &'static str) -> Line<'static> {
    Line::from(vec![
        Span::from("┫").style(theme().border.config_border),
        Span::from(" ").style(theme().border.title),
        Span::from(title).style(theme().border.title),
        Span::from(" ").style(theme().border.title),
        Span::from("┣").style(theme().border.config_border),
    ])
}
