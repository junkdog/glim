use ratatui::prelude::{Line, Span};
use ratatui::widgets::BorderType;
use crate::gruvbox::Gruvbox::{Dark0, Dark0Hard, Dark3};
use crate::interpolation::Interpolation;
use crate::shader::fx::OpenWindow;
use crate::shader::{fade_to, never_complete, open_window_fx, sequence, sleep};
use crate::theme::theme;
use crate::ui::widget::Shortcuts;

pub fn open_window(
    title: &'static str,
    shortcuts: Option<Vec<(&'static str, &'static str)>>,
) -> OpenWindow {
    let fade_screen_bg = sequence(vec![
        sleep(250),
        never_complete(fade_to(Dark3, Dark0Hard, 750, Interpolation::Pow(2))),
    ]);

    let title = Line::from(vec![
        Span::from("┫").style(theme().border.config_border),
        Span::from(" ").style(theme().border.title),
        Span::from(title).style(theme().border.title),
        Span::from(" ").style(theme().border.title),
        Span::from("┣").style(theme().border.config_border),
    ]);

    OpenWindow::builder()
        .title(title)
        .border_style(theme().border.config_border)
        .border_type(BorderType::Rounded)
        .title_style(theme().border.title)
        .background(theme().background)
        .parent_window_fx(fade_screen_bg)
        .open_window_fx(open_window_fx(Dark0))
        .shortcuts(shortcuts.map(Shortcuts::from))
        .build()
        .unwrap()
}