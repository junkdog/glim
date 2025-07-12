use std::fmt::Debug;

use ratatui::{buffer::Buffer, layout::Rect};
use tachyonfx::{CellFilter, ColorSpace, Duration, Effect, EffectTimer, Shader};

use crate::ui::widget::RefRect;

/// A shader wrapper that applies effects to a dynamically changing rectangular area.
///
/// `DynamicArea` solves the problem of effects needing to adapt to changing layout areas
/// in real-time. Unlike regular effects which have static areas, `DynamicArea` uses a
/// shared, mutable area reference that can be updated during effect execution.
///
/// This is particularly useful in terminal UIs where widget areas frequently change due to:
/// - Window resizing
/// - Dynamic layout updates  
/// - Responsive design adjustments
/// - Content-driven sizing
///
/// # Examples
///
/// ```rust
/// use ratatui::layout::Rect;
/// use tachyonfx::fx;
/// use crate::ui::fx::DynamicArea;
///
/// // Create an effect that will adapt to area changes
/// let mut dynamic_effect = DynamicArea::new(
///     Rect::new(0, 0, 20, 5),
///     fx::fade_in(1000)
/// );
///
/// // Later, if the widget area changes, update the effect area
/// dynamic_effect.set_area(Rect::new(0, 0, 30, 8));
/// // The effect will now use the new area for subsequent processing
/// ```
///
/// # Architecture
///
/// `DynamicArea` acts as a wrapper around any `Effect`, delegating most `Shader` trait
/// methods while overriding area-related functionality to use the dynamic area reference.
/// Multiple `DynamicArea` instances can share the same area reference using `RefRect`.
#[derive(Clone, Debug)]
pub struct DynamicArea {
    rect: RefRect,
    fx: Effect,
}

impl DynamicArea {
    /// Creates a new `DynamicArea` with the given initial area and effect.
    ///
    /// # Arguments
    ///
    /// * `area` - The initial rectangular area where the effect will be applied
    /// * `fx` - The effect to wrap with dynamic area capabilities
    ///
    /// # Returns
    ///
    /// A new `DynamicArea` that will apply the effect to the specified area,
    /// with the ability to update the area dynamically during execution.
    pub fn new(area: RefRect, fx: Effect) -> Self {
        Self { rect: area, fx }
    }
}

impl Shader for DynamicArea {
    fn name(&self) -> &'static str {
        "dynamic_area"
    }

    fn process(&mut self, duration: Duration, buf: &mut Buffer, _area: Rect) -> Option<Duration> {
        self.fx.process(duration, buf, self.rect.get())
    }

    fn done(&self) -> bool {
        self.fx.done()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        Some(self.rect.get())
    }

    fn set_area(&mut self, area: Rect) {
        self.rect.set(area)
    }

    fn filter(&mut self, filter: CellFilter) {
        self.fx.filter(filter)
    }

    fn timer(&self) -> Option<EffectTimer> {
        self.fx.timer()
    }

    fn cell_filter(&self) -> Option<CellFilter> {
        self.fx.cell_filter()
    }

    fn set_color_space(&mut self, color_space: ColorSpace) {
        self.fx.set_color_space(color_space);
    }

    fn color_space(&self) -> ColorSpace {
        self.fx.color_space()
    }

    fn reset(&mut self) {
        self.fx.reset();
    }
}
