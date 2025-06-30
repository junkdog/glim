use ratatui::layout::Margin;
use ratatui::style::Color;
use ratatui::text::{Line, Span};
use tachyonfx::fx::{coalesce, dissolve, fade_from, fade_to, never_complete, parallel, sequence, sleep, sweep_in, with_duration};
use tachyonfx::Motion;
use tachyonfx::{Effect, fx, Interpolation, Duration};
use tachyonfx::CellFilter::{AllOf, Inner, Not, Outer, Text};
use tachyonfx::fx::Glitch;
use tachyonfx::IntoEffect;

use crate::event::GlitchState;
use crate::gruvbox::Gruvbox::{Dark0, Dark0Hard, Dark3};
use crate::theme::theme;

/// Creates a window opening effect with fade-in animation
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
            .with_filter(border_decorations),

        // window title and shortcuts
        sequence(&[
            fx::timed_never_complete(Duration::from_millis(320), fade_to(Dark0, Dark0, 0)),
            fade_from(Dark0, Dark0, (320, Interpolation::QuadOut)),
        ]).with_filter(border_text),

        // content area
        sequence(&[
            with_duration(Duration::from_millis(270), parallel(&[
                never_complete(dissolve(0)), // hiding icons/emoji
                never_complete(fade_to(bg, bg, 0)),
            ])),
            parallel(&[
                coalesce(Duration::from_millis(120)),
                fade_from(bg, bg, (130, Interpolation::QuadOut)),
                sweep_in(Motion::UpToDown, 10, 0, bg, (130, Interpolation::Linear)),
            ]),
        ]).with_filter(Inner(margin)),
    ])
}

/// Creates a popup window title line with effects (TODO: integrate with proper window system)
pub fn create_window_title(title: &'static str) -> Line<'static> {
    Line::from(vec![
        Span::from("┫").style(theme().border.config_border),
        Span::from(" ").style(theme().border.title),
        Span::from(title).style(theme().border.title),
        Span::from(" ").style(theme().border.title),
        Span::from("┣").style(theme().border.config_border),
    ])
}

/// Creates a background fade effect for popup windows
pub fn popup_background_fade_effect() -> Effect {
    sequence(&[
        sleep(250),
        never_complete(fade_to(Dark3, Dark0Hard, (750, Interpolation::CircInOut))),
    ])
}

/// Creates a notification effect with fade in/out and blinking
pub fn notification_effect() -> Effect {
    use crate::gruvbox::Gruvbox::Dark0Hard;
    use tachyonfx::Interpolation::{SineIn, SineOut};

    fx::sequence(&[
        // 1. clear the border (border is already cleared, so we first fill it back in)
        fx::parallel(&[
            draw_border(Duration::from_millis(100)),
            fx::dissolve(Duration::from_millis(100))
        ]),
        // 2. fade in notification text
        fx::fade_from_fg(Dark0Hard, (250, SineOut)),
        // 3. smooth blink while notification is shown
        fx::with_duration(Duration::from_millis(6000),
            fx::repeating(fx::ping_pong(
                fx::hsl_shift_fg([0.0, 0.0, 25.0], (500, SineOut))
            )),
        ),
        // 4. fade out notification text and then redraw border
        fx::prolong_end(Duration::from_millis(100),
            fx::fade_to_fg(Dark0Hard, (250, SineIn))),
        fx::parallel(&[
            draw_border(Duration::from_millis(150)),
            fx::coalesce(150),
        ]),
    ])
}

/// Helper function for drawing notification border
fn draw_border(duration: Duration) -> Effect {
    fx::effect_fn((), duration, |_, _, cells| {
        cells.for_each(|(_, cell)| { cell.set_char('─'); });
    })
}

/// Creates a glitch effect based on the glitch state
pub fn make_glitch_effect(glitch_state: GlitchState) -> Option<Effect> {
    match glitch_state {
        GlitchState::Inactive => None,
        GlitchState::Active => Some(Glitch::builder()
            .action_ms(100..200)
            .action_start_delay_ms(0..500)
            .cell_glitch_ratio(0.05)
            .build()
            .into_effect())
    }
}

/// Creates a default glitch effect
pub fn default_glitch_effect() -> Effect {
    Glitch::builder()
        .action_ms(100..500)
        .action_start_delay_ms(0..2000)
        .cell_glitch_ratio(0.0015)
        .build()
        .into_effect()
}

/// Creates a table fade-in effect
pub fn fade_in_projects_table() -> Effect {
    parallel(&[
        fx::coalesce(550),
        fx::sweep_in(Motion::LeftToRight, 50, 0, Dark0Hard, (450, Interpolation::QuadIn))
    ])
}

/// Creates a project details close effect
pub fn project_details_close_effect() -> Effect {
    fx::fade_from(Dark3, Dark0Hard, (300, Interpolation::CircIn))
}

// Widget-tied effect placeholders (keep function signatures but remove implementation)

/// Placeholder for config popup effects - TODO: recreate later
pub fn config_popup_effect() -> Effect {
    // TODO: Implement config popup specific effects
    fx::never_complete(fx::fade_to(Dark0Hard, Dark0Hard, 0))
}

/// Placeholder for pipeline actions popup effects - TODO: recreate later  
pub fn pipeline_actions_popup_effect() -> Effect {
    // TODO: Implement pipeline actions popup specific effects
    fx::never_complete(fx::fade_to(Dark0Hard, Dark0Hard, 0))
}

/// Placeholder for project details popup effects - TODO: recreate later
pub fn project_details_popup_effect() -> Effect {
    // TODO: Implement project details popup specific effects
    fx::never_complete(fx::fade_to(Dark0Hard, Dark0Hard, 0))
}