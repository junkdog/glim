use std::{fmt::Debug, sync::mpsc::Sender};

use ratatui::{
    buffer::Buffer,
    layout::{Margin, Rect},
    style::Color,
};
use tachyonfx::{
    fx::*,
    ref_count, CellFilter,
    CellFilter::{AllOf, Inner, Not, Outer, Text},
    Duration, Effect, EffectManager, Interpolation, IntoEffect, Motion,
};

use crate::{
    event::{GlimEvent, GlitchState},
    gruvbox::Gruvbox::{Dark0, Dark0Hard, Dark3},
    ui::{fx::dynamic_area::DynamicArea, widget::RefRect},
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FxId {
    #[default]
    ConfigPopup,
    Glitch,
    Notification,
    PipelineActionsPopup,
    ProjectDetailsPopup,
    Screen,
}

pub struct EffectRegistry {
    effects: EffectManager<FxId>,
    sender: Sender<GlimEvent>,
    screen_area: RefRect,
}

impl EffectRegistry {
    pub fn apply(&mut self, event: &GlimEvent) {
        match event {
            GlimEvent::GlitchOverride(g) => self.register_ramped_up_glitch_effect(*g),
            GlimEvent::CloseProjectDetails => self.register_close_popup(FxId::ProjectDetailsPopup),
            GlimEvent::ClosePipelineActions => {
                self.register_close_popup(FxId::PipelineActionsPopup)
            },
            GlimEvent::CloseConfig => self.register_close_popup(FxId::ConfigPopup),
            _ => (),
        }
    }

    pub fn update_screen_area(&self, screen_area: Rect) {
        self.screen_area.set(screen_area);
    }
}

impl EffectRegistry {
    pub fn new(sender: Sender<GlimEvent>) -> Self {
        Self {
            effects: EffectManager::default(),
            screen_area: RefRect::default(),
            sender,
        }
    }

    pub fn process_effects(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) {
        self.effects.process_effects(duration, buf, area);
    }

    /// Creates a glitch effect based on the glitch state
    pub fn register_ramped_up_glitch_effect(&mut self, glitch_state: GlitchState) {
        let fx = match glitch_state {
            GlitchState::Inactive => {
                return self.register_default_glitch_effect();
            },
            GlitchState::Active => Glitch::builder()
                .action_ms(100..200)
                .action_start_delay_ms(0..500)
                .cell_glitch_ratio(0.05)
                .build()
                .into_effect(),
        };

        self.add_unique(FxId::Glitch, fx);
    }

    /// Creates a default glitch effect
    pub fn register_default_glitch_effect(&mut self) {
        let fx = Glitch::builder()
            .action_ms(100..500)
            .action_start_delay_ms(0..2000)
            .cell_glitch_ratio(0.0015)
            .build()
            .into_effect();

        self.add_unique(FxId::Glitch, fx);
    }

    pub fn register_project_details(&mut self, popup_area: RefRect) {
        self.register_popup(FxId::ProjectDetailsPopup, popup_area);
    }

    pub fn register_pipeline_actions(&mut self, popup_area: RefRect) {
        self.register_popup(FxId::PipelineActionsPopup, popup_area);
    }

    pub fn register_config_popup(&mut self, popup_area: RefRect) {
        self.register_popup(FxId::ConfigPopup, popup_area);
    }

    /// Registers a generic popup with standard opening effects
    fn register_popup(&mut self, id: FxId, popup_area: RefRect) {
        let fx = parallel(&[
            dynamic_area(popup_area.clone(), open_window_fx(Dark0)),
            dim_screen_behind_popup(self.screen_area(), popup_area),
        ]);

        self.add_unique(id, fx);
    }

    /// Creates a close effect for popups
    fn register_close_popup(&mut self, id: FxId) {
        let fx = fade_from(Dark3, Dark0Hard, (300, Interpolation::CircIn));
        self.add_unique(id, fx);
    }

    /// Creates a notification effect with fade in/out and blinking
    pub fn register_notification_effect(&mut self, content_area: RefRect) {
        use tachyonfx::Interpolation::{SineIn, SineOut};

        use crate::gruvbox::Gruvbox::Dark0Hard;

        let main_fx = sequence(&[
            // 1. clear the border (border is already cleared, so we first fill it back in)
            parallel(&[
                draw_border(Duration::from_millis(100)),
                dissolve(Duration::from_millis(100)),
            ]),
            // 2. fade in notification text
            fade_from_fg(Dark0Hard, (250, SineOut)),
            // 3. smooth blink while notification is shown
            with_duration(
                Duration::from_millis(6000),
                repeating(ping_pong(hsl_shift_fg([0.0, 0.0, 25.0], (500, SineOut)))),
            ),
            // 4. fade out notification text and then redraw border
            prolong_end(Duration::from_millis(100), fade_to_fg(Dark0Hard, (250, SineIn))),
            parallel(&[draw_border(Duration::from_millis(150)), coalesce(150)]),
        ]);

        // effect configuration wrapping
        let fx = sequence(&[
            // dynamically track area size in case of window resizing
            dynamic_area(content_area, main_fx),
            // lastly, dispatch a close notification event
            self.dispatch(GlimEvent::CloseNotification),
        ]);

        self.add_unique(FxId::Notification, fx);
    }

    fn dispatch(&mut self, event: GlimEvent) -> Effect {
        dispatch_event(self.sender.clone(), event)
    }

    fn screen_area(&self) -> RefRect {
        self.screen_area.clone()
    }

    fn add_unique(&mut self, id: FxId, fx: Effect) {
        self.effects.add_unique_effect(id, fx);
    }
}

/// Creates a dynamic area effect that adapts to changes in the area
fn dynamic_area(area: RefRect, fx: Effect) -> Effect {
    DynamicArea::new(area, fx).into_effect()
}

/// Creates a window opening effect with fade-in animation
fn open_window_fx<C: Into<Color>>(bg: C) -> Effect {
    let margin = Margin::new(1, 1);
    let border_text = AllOf(vec![Outer(margin), Text]);
    let border_decorations = AllOf(vec![Outer(margin), Not(Text.into())]);

    let bg = bg.into();

    // window open effect; effects run in parallel for:
    // - window borders
    // - window title and shortcuts
    // - content area
    parallel(&[
        // window borders
        fade_from(Dark0, Dark0, (320, Interpolation::QuadOut)).with_filter(border_decorations),
        // window title and shortcuts
        sequence(&[
            timed_never_complete(Duration::from_millis(320), fade_to(Dark0, Dark0, 0)),
            fade_from(Dark0, Dark0, (320, Interpolation::QuadOut)),
        ])
        .with_filter(border_text),
        // content area
        sequence(&[
            with_duration(
                Duration::from_millis(270),
                parallel(&[
                    never_complete(dissolve(0)), // hiding icons/emoji
                    never_complete(fade_to(bg, bg, 0)),
                ]),
            ),
            parallel(&[
                coalesce(Duration::from_millis(120)),
                fade_from(bg, bg, (130, Interpolation::QuadOut)),
                sweep_in(Motion::UpToDown, 10, 0, bg, (130, Interpolation::Linear)),
            ]),
        ])
        .with_filter(Inner(margin)),
    ])
}

/// Creates a background fade effect for popup windows to dim the screen behind them
fn dim_screen_behind_popup(screen_area: RefRect, popup_area: RefRect) -> Effect {
    let screen = ref_rect_filter(screen_area);
    let popup = ref_rect_filter(popup_area);

    let behind_popup = AllOf(vec![screen, Not(Box::new(popup))]);

    sequence(&[
        sleep(250),
        never_complete(fade_to(Dark3, Dark0Hard, (750, Interpolation::CircInOut))),
    ])
    .with_filter(behind_popup)
}

fn ref_rect_filter(ref_rect: RefRect) -> CellFilter {
    CellFilter::PositionFn(ref_count(Box::new(move |pos| ref_rect.contains(pos))))
}

/// Creates a table fade-in effect
pub fn fade_in_projects_table() -> Effect {
    parallel(&[
        coalesce(550),
        sweep_in(Motion::LeftToRight, 50, 0, Dark0Hard, (450, Interpolation::QuadIn)),
    ])
}

/// Helper function for drawing notification border
fn draw_border(duration: Duration) -> Effect {
    effect_fn((), duration, |_, _, cells| {
        cells.for_each(|(_, cell)| {
            cell.set_char('─');
        });
    })
}

/// Creates an effect that dispatches an event as soon as it starts.
///
/// # Type Parameters
/// * `T` - Event type that implements Clone and 'static
///
/// # Arguments
/// * `sender` - Channel for sending the event
/// * `event` - Event to be dispatched
///
/// # Returns
/// An Effect that dispatches the specified event.
fn dispatch_event<T: Clone + Debug + Send + 'static>(sender: Sender<T>, event: T) -> Effect {
    effect_fn_buf(Some(event), 1, move |e, _, _| {
        if let Some(e) = e.take() {
            sender.send(e).unwrap_or_default()
        }
    })
}
