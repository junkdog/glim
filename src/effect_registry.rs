//! Visual effects registry for the Glim TUI application.
//!
//! This module provides the central coordination system for all visual effects
//! in Glim, built on top of the tachyonfx library. It manages popup animations,
//! glitch effects, notifications, and screen transitions with a unified interface.
//!
//! # Architecture
//!
//! The effect system is built around several key concepts:
//!
//! - **Effect Registry**: Central coordinator for all effects
//! - **Effect IDs**: Unique identifiers preventing effect conflicts  
//! - **Dynamic Areas**: Effects that adapt to UI layout changes
//! - **Event Integration**: Effects triggered by application events
//!
//! # Key Components
//!
//! - [`EffectRegistry`]: Main registry managing all visual effects
//! - [`FxId`]: Enumeration of effect identifiers
//! - Helper functions for common effect patterns
//!
//! # Usage
//!
//! ```rust
//! # use std::sync::mpsc;
//! # use glim::effect_registry::EffectRegistry;
//! # use glim::event::GlimEvent;
//!
//! let (sender, _receiver) = mpsc::channel();
//! let mut registry = EffectRegistry::new(sender);
//!
//! // Register a default glitch effect
//! registry.register_default_glitch_effect();
//!
//! // Process effects each frame
//! // registry.process_effects(duration, buffer, screen_area);
//! ```

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

/// Unique identifiers for different visual effects in the application.
///
/// Each variant represents a specific UI component or screen area that can have
/// visual effects applied to it. Effects are managed using these IDs to ensure
/// proper isolation and lifecycle management.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FxId {
    /// Configuration popup dialog effects
    #[default]
    ConfigPopup,
    /// Global screen glitch effects
    Glitch,
    /// Notification message effects
    Notification,
    /// Pipeline actions popup dialog effects
    PipelineActionsPopup,
    /// Project details popup dialog effects
    ProjectDetailsPopup,
}

/// Central registry for managing visual effects in the Glim TUI application.
///
/// The `EffectRegistry` coordinates all visual effects using the tachyonfx library,
/// providing a unified interface for registering, updating, and processing effects.
/// It handles popup animations, glitch effects, notifications, and screen transitions.
///
/// # Key Features
///
/// - **Effect Management**: Centralized control over all visual effects
/// - **Event Integration**: Responds to application events to trigger appropriate effects
/// - **Dynamic Areas**: Supports effects that adapt to changing UI layouts
/// - **Effect Isolation**: Uses unique IDs to prevent effect conflicts
///
/// # Example
///
/// ```rust
/// let mut registry = EffectRegistry::new(event_sender);
/// registry.register_default_glitch_effect();
/// registry.process_effects(duration, buffer, screen_area);
/// ```
pub struct EffectRegistry {
    /// Internal effect manager from tachyonfx
    effects: EffectManager<FxId>,
    /// Channel for dispatching events back to the application
    sender: Sender<GlimEvent>,
    /// Reference to the current screen area for layout-aware effects
    screen_area: RefRect,
}

impl EffectRegistry {
    /// Applies visual effects in response to application events.
    ///
    /// This method serves as the main event handler for the effect system,
    /// translating application events into appropriate visual effects.
    ///
    /// # Arguments
    ///
    /// * `event` - The application event that may trigger visual effects
    ///
    /// # Supported Events
    ///
    /// - `GlitchOverride`: Triggers ramped-up glitch effects
    /// - `CloseProjectDetails`: Initiates project details popup close animation
    /// - `ClosePipelineActions`: Initiates pipeline actions popup close animation
    /// - `CloseConfig`: Initiates config popup close animation
    pub fn apply(&mut self, event: &GlimEvent) {
        match event {
            GlimEvent::GlitchOverride(g) => self.register_ramped_up_glitch_effect(*g),
            GlimEvent::ProjectDetailsClose => self.register_close_popup(FxId::ProjectDetailsPopup),
            GlimEvent::PipelineActionsClose => {
                self.register_close_popup(FxId::PipelineActionsPopup)
            },
            GlimEvent::ConfigClose => self.register_close_popup(FxId::ConfigPopup),
            _ => (),
        }
    }

    /// Updates the screen area reference for layout-aware effects.
    ///
    /// This method should be called whenever the terminal is resized or the
    /// screen layout changes to ensure effects render correctly.
    ///
    /// # Arguments
    ///
    /// * `screen_area` - The new screen dimensions
    pub fn update_screen_area(&self, screen_area: Rect) {
        self.screen_area.set(screen_area);
    }
}

impl EffectRegistry {
    /// Creates a new effect registry with the specified event sender.
    ///
    /// # Arguments
    ///
    /// * `sender` - Channel for dispatching events back to the application
    ///
    /// # Returns
    ///
    /// A new `EffectRegistry` instance ready to manage visual effects
    pub fn new(sender: Sender<GlimEvent>) -> Self {
        Self {
            effects: EffectManager::default(),
            screen_area: RefRect::default(),
            sender,
        }
    }

    /// Processes all active effects for the current frame.
    ///
    /// This method should be called once per frame to update and render
    /// all active visual effects to the terminal buffer.
    ///
    /// # Arguments
    ///
    /// * `duration` - Time elapsed since the last frame
    /// * `buf` - Mutable reference to the terminal buffer to render into
    /// * `area` - The screen area to render effects within
    pub fn process_effects(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) {
        self.effects.process_effects(duration, buf, area);
    }

    /// Creates a glitch effect based on the specified glitch state.
    ///
    /// This method generates different intensities of glitch effects depending
    /// on the current state, providing visual feedback for system activity.
    ///
    /// # Arguments
    ///
    /// * `glitch_state` - The intensity level of the glitch effect to apply
    ///
    /// # Effect Characteristics
    ///
    /// - **Inactive**: Falls back to default low-intensity glitch
    /// - **Active**: Higher intensity with more frequent glitch bursts
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

    /// Creates a default low-intensity glitch effect.
    ///
    /// This effect provides subtle visual feedback with infrequent glitch
    /// occurrences that don't interfere with normal UI usage.
    ///
    /// # Effect Characteristics
    ///
    /// - Action duration: 100-500ms
    /// - Delay between actions: 0-2000ms  
    /// - Cell glitch ratio: 0.0015 (very subtle)
    pub fn register_default_glitch_effect(&mut self) {
        let fx = Glitch::builder()
            .action_ms(100..500)
            .action_start_delay_ms(0..2000)
            .cell_glitch_ratio(0.0015)
            .build()
            .into_effect();

        self.add_unique(FxId::Glitch, fx);
    }

    /// Registers opening effects for the project details popup.
    ///
    /// # Arguments
    ///
    /// * `popup_area` - Reference to the popup's screen area
    pub fn register_project_details(&mut self, popup_area: RefRect) {
        self.register_popup(FxId::ProjectDetailsPopup, popup_area);
    }

    /// Registers opening effects for the pipeline actions popup.
    ///
    /// # Arguments
    ///
    /// * `popup_area` - Reference to the popup's screen area
    pub fn register_pipeline_actions(&mut self, popup_area: RefRect) {
        self.register_popup(FxId::PipelineActionsPopup, popup_area);
    }

    /// Registers opening effects for the configuration popup.
    ///
    /// # Arguments
    ///
    /// * `popup_area` - Reference to the popup's screen area
    pub fn register_config_popup(&mut self, popup_area: RefRect) {
        self.register_popup(FxId::ConfigPopup, popup_area);
    }

    /// Registers a generic popup with standard opening effects.
    ///
    /// Creates a combined effect that includes window opening animation
    /// and background dimming for visual focus.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the popup effect
    /// * `popup_area` - Reference to the popup's screen area
    fn register_popup(&mut self, id: FxId, popup_area: RefRect) {
        let fx = parallel(&[
            dynamic_area(popup_area.clone(), open_window_fx(Dark0)),
            dim_screen_behind_popup(self.screen_area(), popup_area),
        ]);

        self.add_unique(id, fx);
    }

    /// Creates a close effect for popups.
    ///
    /// Applies a fade-out animation to smoothly hide the popup.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the popup to close
    fn register_close_popup(&mut self, id: FxId) {
        let fx = fade_from(Dark3, Dark0Hard, (300, Interpolation::CircIn));
        self.add_unique(id, fx);
    }

    /// Creates a notification effect with fade in/out and blinking.
    ///
    /// This creates a complex animation sequence that draws attention to
    /// notifications while maintaining visual polish.
    ///
    /// # Arguments
    ///
    /// * `content_area` - Reference to the notification's display area
    ///
    /// # Animation Sequence
    ///
    /// 1. Border drawing and dissolve effect (100ms)
    /// 2. Fade in notification text (250ms)
    /// 3. Smooth blinking highlight (6 seconds)
    /// 4. Fade out text and redraw border (350ms total)
    /// 5. Dispatch close notification event
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
            self.dispatch(GlimEvent::NotificationDismiss),
        ]);

        self.add_unique(FxId::Notification, fx);
    }

    /// Creates an effect that dispatches an event back to the application.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to dispatch
    ///
    /// # Returns
    ///
    /// An effect that sends the event when executed
    fn dispatch(&mut self, event: GlimEvent) -> Effect {
        dispatch_event(self.sender.clone(), event)
    }

    /// Returns a clone of the current screen area reference.
    fn screen_area(&self) -> RefRect {
        self.screen_area.clone()
    }

    /// Adds an effect with a unique identifier, replacing any existing effect with the same ID.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the effect
    /// * `fx` - The effect to register
    fn add_unique(&mut self, id: FxId, fx: Effect) {
        self.effects.add_unique_effect(id, fx);
    }
}

/// Creates a dynamic area effect that adapts to changes in the area.
///
/// This wrapper allows effects to automatically adjust when the UI layout
/// changes, such as during terminal resizing.
///
/// # Arguments
///
/// * `area` - Reference to the area that may change size
/// * `fx` - The effect to apply within the dynamic area
///
/// # Returns
///
/// An effect that tracks area changes and adapts accordingly
fn dynamic_area(area: RefRect, fx: Effect) -> Effect {
    DynamicArea::new(area, fx).into_effect()
}

/// Creates a window opening effect with fade-in animation.
///
/// This creates a sophisticated window opening animation with separate
/// timing for borders, text, and content areas to provide a polished
/// visual experience.
///
/// # Arguments
///
/// * `bg` - Background color for the window content
///
/// # Returns
///
/// A complex parallel effect combining:
/// - Border fade-in (320ms)
/// - Title and shortcuts delayed fade-in (320ms delay + 320ms fade)
/// - Content area dissolve + sweep animation (270ms + 130ms)
///
/// # Type Parameters
///
/// * `C` - Any type that can be converted to a `Color`
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

/// Creates a background fade effect for popup windows to dim the screen behind them.
///
/// This effect provides visual focus by darkening everything except the popup area,
/// helping users concentrate on the active dialog.
///
/// # Arguments
///
/// * `screen_area` - Reference to the full screen area
/// * `popup_area` - Reference to the popup area to exclude from dimming
///
/// # Returns
///
/// An effect that dims the background after a 250ms delay
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

/// Creates a cell filter based on a reference rectangle.
///
/// This helper function creates a filter that can be used to apply effects
/// only to cells within a specific rectangular area.
///
/// # Arguments
///
/// * `ref_rect` - The reference rectangle to filter by
///
/// # Returns
///
/// A `CellFilter` that matches cells within the specified rectangle
fn ref_rect_filter(ref_rect: RefRect) -> CellFilter {
    CellFilter::PositionFn(ref_count(Box::new(move |pos| ref_rect.contains(pos))))
}

/// Creates a table fade-in effect for the projects table.
///
/// This effect provides a smooth entrance animation when the projects
/// table is first displayed or refreshed.
///
/// # Returns
///
/// A parallel effect combining coalescing and left-to-right sweep animation
pub fn fade_in_projects_table() -> Effect {
    parallel(&[
        coalesce(550),
        sweep_in(Motion::LeftToRight, 50, 0, Dark0Hard, (450, Interpolation::QuadIn)),
    ])
}

/// Helper function for drawing notification border.
///
/// Creates a simple border drawing effect that sets horizontal line
/// characters across the affected area.
///
/// # Arguments
///
/// * `duration` - How long the border drawing effect should last
///
/// # Returns
///
/// An effect that draws horizontal border characters
fn draw_border(duration: Duration) -> Effect {
    effect_fn((), duration, |_, _, cells| {
        cells.for_each(|(_, cell)| {
            cell.set_char('─');
        });
    })
}

/// Creates an effect that dispatches an event as soon as it starts.
///
/// This utility function allows effects to trigger application events,
/// enabling coordination between the visual effect system and application logic.
///
/// # Type Parameters
///
/// * `T` - Event type that implements Clone, Debug, Send, and 'static
///
/// # Arguments
///
/// * `sender` - Channel for sending the event
/// * `event` - Event to be dispatched
///
/// # Returns
///
/// An effect that dispatches the specified event immediately when started
fn dispatch_event<T: Clone + Debug + Send + 'static>(sender: Sender<T>, event: T) -> Effect {
    effect_fn_buf(Some(event), 1, move |e, _, _| {
        if let Some(e) = e.take() {
            let _ = sender.send(e);
        }
    })
}
