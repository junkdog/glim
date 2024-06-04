use chrono::Duration;
use ratatui::layout::Margin;
use ratatui::style::Color;

pub use effect::*;
pub use render_effect::RenderEffect;
pub use shader::Shader;
pub use cell_iter::CellIterator;
use crate::gruvbox::Gruvbox::Dark0;

use crate::interpolation::Interpolation;
use crate::shader::FilterMode::{AllOf, Inner, Negate, Outer, Text};
use crate::shader::fx::{Ansi256, ConsumeTick, Dissolve, FadeColors, IntoTemporaryEffect, NeverComplete, ResizeArea, Sleep, SweepIn};
use crate::shader::fx::{ParallelEffect, SequentialEffect};
pub use crate::shader::lifetime::*;

pub mod fx;

pub use utility::*;
mod shader;
mod effect;
mod render_effect;
mod lifetime;
mod cell_iter;
mod utility;

/// An effect that resizes the area of the wrapped effect to the specified
/// dimensions. The effect will be rendered within the resized area.
pub fn resize_area(
    fx: Option<Effect>,
    initial_w: u16,
    initial_h: u16,
    lifetime: EffectTimer
) -> Effect {
    ResizeArea::new(fx, initial_w, initial_h, lifetime).into_effect()
}

/// An effect that forces the wrapped [effect] to never report completion,
/// effectively making it run indefinitely. Once the effect reaches the end,
/// it will continue to process the effect without advancing the duration. 
/// 
pub fn never_complete(effect: Effect) -> Effect {
    NeverComplete::new(effect).into_effect()
}


/// Dissolves the current text into the new text over the specified duration. The
/// `cycle_len` parameter specifies the number of cell states are tracked before
/// it cycles and repeats the previous state.
pub fn dissolve(duration: u32, cycle_len: usize, algo: Interpolation) -> Effect {
    let lifetime = EffectTimer::from_ms(duration, algo);
    Dissolve::new(lifetime, cycle_len)
        .into_effect()
}

/// The reverse of [dissolve].
pub fn coalesce(duration: u32, cycle_len: usize, algo: Interpolation) -> Effect {
    let lifetime = EffectTimer::from_ms(duration, algo).reversed();
    Dissolve::new(lifetime, cycle_len)
        .into_effect()
}

/// Wraps an effect and enforces a duration on it. Once the duration has
/// elapsed, the effect will be marked as complete.
pub fn temporary(duration: u32, effect: Effect) -> Effect {
    effect.with_duration(Duration::milliseconds(duration as i64))
}

/// Runs the effects in sequence, one after the other. Reports completion
/// once the last effect has completed.
pub fn sequence(effects: Vec<Effect>) -> Effect {
    SequentialEffect::new(effects).into_effect()
}

/// Runs the effects in parallel, all at the same time. Reports completion
/// once all effects have completed.
pub fn parallel(effects: Vec<Effect>) -> Effect {
    ParallelEffect::new(effects).into_effect()
}

/// Pauses for the specified duration.
pub fn sleep(duration: u32) -> Effect {
    Sleep::new(Duration::milliseconds(duration as i64)).into_effect()
}

/// Consumes a single tick.
pub fn consume_tick() -> Effect {
    ConsumeTick::default().into_effect()
}

/// Returns an effect that downsamples to 256 color mode.
pub fn term256_colors() -> Effect {
    Ansi256::default().into_effect()
}

/// Sweeps in a gradient from the specified color.
pub fn sweep_in<C: Into<Color>>(
    gradient_length: u16,
    faded_color: C,
    duration: u32,
    algo: Interpolation,
) -> Effect {
    SweepIn::new(gradient_length, faded_color.into(), duration, algo)
        .into_effect()
}

/// Fades the foreground color to the specified color over the specified duration.
pub fn fade_to_fg<C: Into<Color>>(
    fg: C,
    duration: u32,
    algo: Interpolation,
) -> Effect {
    fade(Some(fg), None, duration, algo, false)
}

/// Fades the foreground color from the specified color over the specified duration.
pub fn fade_from_fg<C: Into<Color>>(
    fg: C,
    duration: u32,
    algo: Interpolation,
) -> Effect {
    fade(Some(fg), None, duration, algo, true)
}

/// Fades to the specified the background and foreground colors over the specified duration.
pub fn fade_to<C: Into<Color>>(
    fg: C,
    bg: C,
    duration: u32,
    algo: Interpolation,
) -> Effect {
    fade(Some(fg), Some(bg), duration, algo, false)
}

/// Fades from the specified the background and foreground colors over the specified duration.
pub fn fade_from<C: Into<Color>>(
    fg: C,
    bg: C,
    duration: u32,
    algo: Interpolation,
) -> Effect {
    fade(Some(fg), Some(bg), duration, algo, true)
}

/// Animates and fades in a window from the specified background color.
pub fn open_window_fx<C: Into<Color>>(bg: C) -> Effect {
    let margin = Margin::new(1, 1);
    let border_text        = AllOf(vec![Outer(margin), Text]);
    let border_decorations = AllOf(vec![Outer(margin), Negate(Text.into())]);

    let bg = bg.into();
    
    // window open effect; effects run in parallel for:
    // - window borders
    // - window title and shortcuts
    // - content area
    parallel(vec![
        // window borders
        fade_from(Dark0, Dark0, 320, Interpolation::PowOut(2))
            .with_cell_selection(border_decorations),

        // window title and shortcuts
        sequence(vec![
            temporary(320, never_complete(fade_to(Dark0, Dark0, 0, Interpolation::Linear))),
            fade_from(Dark0, Dark0, 320, Interpolation::PowOut(2)),
        ]).with_cell_selection(border_text),

        // content area
        sequence(vec![
            temporary(270, parallel(vec![ 
                never_complete(dissolve(0, 1, Interpolation::Linear)), // hiding icons/emoji
                never_complete(fade_to(bg, bg, 0, Interpolation::Linear)),
            ])),
            parallel(vec![
                coalesce(120, 111, Interpolation::Linear),
                fade_from(bg, bg, 130, Interpolation::PowOut(2))
            ]),
        ]).with_cell_selection(Inner(margin)),
    ])
}

fn fade<C: Into<Color>>(
    fg: Option<C>,
    bg: Option<C>,
    duration: u32,
    algo: Interpolation,
    reverse: bool,
) -> Effect {
    let mut lifetime = EffectTimer::from_ms(duration, algo);
    if reverse {
        lifetime = lifetime.reversed()
    }
    
    FadeColors::builder()
        .fg(fg.map(|c| c.into()))
        .bg(bg.map(|c| c.into()))
        .lifetime(lifetime)
        .into()
}
