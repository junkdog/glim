use chrono::Duration;
use crate::interpolation::Interpolation;

#[derive(Clone)]
pub struct EffectTimer {
    remaining: Duration,
    total: Duration,
    interpolation: Interpolation,
    reverse: bool
}

impl EffectTimer {
    pub fn from_ms(
        duration: u32,
        interpolation: Interpolation,
    ) -> Self {
        Self::new(Duration::milliseconds(duration as i64), interpolation)
    }

    pub fn new(
        duration: Duration,
        interpolation: Interpolation,
    ) -> Self {
        Self {
            remaining: duration,
            total: duration,
            interpolation,
            reverse: false
        }
    }
    
    pub fn reversed(self) -> Self {
        Self { reverse: !self.reverse, ..self }
    }

    pub fn started(&self) -> bool {
        self.total.num_milliseconds() != self.remaining.num_milliseconds()
    }

    pub fn alpha(&self) -> f32 {
        let total = self.total.num_milliseconds() as f32;
        if total == 0.0 {
            return 1.0;
        }

        let remaining = self.remaining.num_milliseconds() as f32;
        let inv_alpha = (remaining / total).clamp(0.0, 1.0);

        let a = if self.reverse { inv_alpha } else { 1.0 - inv_alpha };
        self.interpolation.alpha(a)
    }

    pub fn process(&mut self, duration: &Duration) -> Option<Duration> {
        self.remaining = self.remaining
            .checked_sub(duration)
            .unwrap_or_default();

        if self.remaining < Duration::zero() {
            let delta = self.remaining.abs();
            self.remaining = Duration::zero();
            Some(delta)
        } else {
            None
        }
    }
    
    pub fn done(&self) -> bool {
        self.remaining == Duration::zero()
    }
}
