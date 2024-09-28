mod window;

use ratatui::layout::Margin;
use ratatui::style::Color;
use ratatui::text::{Line, Span};
use ratatui::widgets::BorderType;
use tachyonfx::fx::{coalesce, Direction, dissolve, fade_from, fade_to, never_complete, parallel, sequence, sleep, sweep_in, timed_never_complete, with_duration};
use tachyonfx::{Effect, fx, Interpolation, Duration};
use tachyonfx::CellFilter::{AllOf, Inner, Not, Outer, Text};
pub use window::*;
use crate::gruvbox::Gruvbox::{Dark0, Dark0Hard, Dark3};
use crate::theme::theme;
use crate::ui::widget::Shortcuts;

pub fn open_window(
    title: &'static str,
    shortcuts: Option<Vec<(&'static str, &'static str)>>,
) -> OpenWindow {
    let fade_screen_bg = sequence(&[
        sleep(250),
        never_complete(fade_to(Dark3, Dark0Hard, (750, Interpolation::CircInOut))),
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

/// Animates and fades in a window from the specified background color.
pub fn open_window_fx<C: Into<Color>>(bg: C) -> Effect {
    let margin = Margin::new(1, 1);
    let border_text        = AllOf(vec![Outer(margin), Text]);
    let border_decorations = AllOf(vec![Outer(margin), Not(Text.into())]);

    let bg = bg.into();

    // window open effect; effects run in parallel for:
    // - window borders
    // - window title and shortcuts
    // - content area
    parallel(&[
        // window borders
        fade_from(Dark0, Dark0, (320, Interpolation::QuadOut))
            .with_cell_selection(border_decorations),

        // window title and shortcuts
        sequence(&[
            fx::timed_never_complete(Duration::from_millis(320), fade_to(Dark0, Dark0, 0)),
            fade_from(Dark0, Dark0, (320, Interpolation::QuadOut)),
        ]).with_cell_selection(border_text),

        // content area
        sequence(&[
            with_duration(Duration::from_millis(270), parallel(&[
                never_complete(dissolve(0)), // hiding icons/emoji
                never_complete(fade_to(bg, bg, 0)),
            ])),
            parallel(&[
                coalesce(Duration::from_millis(120)),
                fade_from(bg, bg, (130, Interpolation::QuadOut)),
                sweep_in(Direction::UpToDown, 10, 0, bg, (130, Interpolation::Linear)),
            ]),
        ]).with_cell_selection(Inner(margin)),
    ])
}