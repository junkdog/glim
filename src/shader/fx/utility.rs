use chrono::Duration;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use Interpolation::Linear;

use crate::interpolation::Interpolation;
use crate::shader::{Effect, IntoEffect, Shader};
use crate::shader::effect::FilterMode;
use crate::shader::lifetime::EffectTimer;

#[derive(Clone)]
pub struct Sleep {
    timer: EffectTimer,
}

impl Sleep {
    pub fn new(duration: Duration) -> Self {
        Self { timer: EffectTimer::new(duration, Linear) }  
    }
}

impl Shader for Sleep {
    fn process(
        &mut self,
        duration: Duration,
        _buf: &mut Buffer,
        _area: Rect
    ) -> Option<Duration> {
        self.timer.process(&duration)
    }

    fn done(&self) -> bool {
        self.timer.done()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> { None }
    fn set_area(&mut self, _area: Rect) {}
    fn cell_selection(&mut self, _strategy: FilterMode) {}
}

/// consumes any remaining duration for a single tick.
#[derive(Default, Clone)]
pub struct ConsumeTick {
    has_consumed_tick: bool,
}

impl Shader for ConsumeTick {
    fn process(
        &mut self,
        _duration: Duration,
        _buf: &mut Buffer,
        _area: Rect,
    ) -> Option<Duration> {
        self.has_consumed_tick = true;
        None
    }

    fn done(&self) -> bool { self.has_consumed_tick }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> { None }
    fn set_area(&mut self, _area: Rect) {}
    fn cell_selection(&mut self, _strategy: FilterMode) {}
}

#[derive(Clone)]
pub struct TemporaryEffect {
    effect: Effect,
    duration: EffectTimer,
}

impl TemporaryEffect {
    pub fn new(effect: Effect, duration: Duration) -> Self {
        Self { effect, duration: EffectTimer::new(duration, Linear) }
    }
}

impl Shader for TemporaryEffect {
    fn process(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect
    ) -> Option<Duration> {
        let remaining = self.duration.process(&duration);

        let effect_area = self.effect.area().unwrap_or(area);
        self.effect.process(duration, buf, effect_area);

        remaining
    }

    fn done(&self) -> bool {
        self.duration.done() || self.effect.done()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        self.effect.area()
    }

    fn set_area(&mut self, area: Rect) {
        self.effect.set_area(area)
    }
    
    fn cell_selection(&mut self, strategy: FilterMode) {
        self.effect.cell_selection(strategy);
    }
}

pub trait IntoTemporaryEffect {
    fn with_duration(self, duration: Duration) -> Effect;
}

impl IntoTemporaryEffect for Effect {
    fn with_duration(self, duration: Duration) -> Effect {
        TemporaryEffect::new(self, duration).into_effect()
    }
}

#[derive(Clone)]
pub struct NeverComplete {
    effect: Effect,
}

impl NeverComplete {
    pub fn new(effect: Effect) -> Self {
        Self { effect }
    }
}

impl Shader for NeverComplete {
    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        self.effect.process(duration, buf, area);
        None
    }

    fn done(&self) -> bool                      { false }
    fn clone_box(&self) -> Box<dyn Shader>      { Box::new(self.clone()) }
    fn area(&self) -> Option<Rect>              { self.effect.area() }
    fn set_area(&mut self, area: Rect)          { self.effect.set_area(area) }

    fn cell_selection(&mut self, strategy: FilterMode) {
        self.effect.cell_selection(strategy);
    }
}