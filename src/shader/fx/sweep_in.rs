use std::ops::Sub;

use chrono::Duration;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::Color;

use crate::interpolation::Interpolation;
use crate::interpolation::Interpolation::Smooth;
use crate::shader::effect::FilterMode;
use crate::shader::fx::internal::MappedColor;
use crate::shader::lifetime::EffectTimer;
use crate::shader::Shader;

#[derive(Clone)]
pub struct SweepIn {
    gradient_length: u16,
    faded_color: Color,
    lifetime: EffectTimer,
    area: Option<Rect>,
    cell_filter: FilterMode,
}

impl SweepIn {
    pub fn new(
        gradient_length: u16,
        faded_color: Color,
        duration: u32,
        algo: Interpolation,
    ) -> Self {
        Self {
            gradient_length,
            faded_color,
            lifetime: EffectTimer::from_ms(duration, algo),
            area: None,
            cell_filter: FilterMode::All,
        }
    }
}

impl Shader for SweepIn {
    fn process(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect,
    ) -> Option<Duration> {
        let alpha = self.lifetime.alpha();
        let remainder = self.lifetime.process(&duration);

        // gradient starts outside the area
        let gradient_len = self.gradient_length;
        let gradient_start: f32 = ((area.width + gradient_len) as f32 * alpha)
            .round()
            .sub(gradient_len as f32);

        let gradient_range = gradient_start..(gradient_start + gradient_len as f32);
        let window_alpha = |x: u16| {
            // fade in, left to right using a linear gradient
            match x as f32 {
                x if gradient_range.contains(&x) => 1.0 - (x - gradient_start) / gradient_len as f32,
                x if x < gradient_range.start    => 1.0,
                _                                => 0.0,
            }
        };
        
        let cell_filter = self.cell_filter.selector(area);
        
        let mut fg_mapper = MappedColor::default();
        let mut bg_mapper = MappedColor::default();
        
        self.cell_iter(buf, area)
            .filter(|(pos, cell)| cell_filter.is_valid(*pos, cell))
            .for_each(|(pos, cell)| {
                let a = window_alpha(pos.x);
                
                if a != 1.0 {
                    let fg = fg_mapper
                        .map(cell.fg, a, |c| Smooth.interpolate(self.faded_color, c, a));
                    let bg = bg_mapper
                        .map(cell.bg, a, |c| Smooth.interpolate(self.faded_color, c, a));

                    cell.set_fg(fg);
                    cell.set_bg(bg);
                }
            });
        
        remainder
    }

    fn done(&self) -> bool {
        self.lifetime.done()
    }

    fn clone_box(&self) -> Box<dyn Shader> {
        Box::new(self.clone())
    }

    fn area(&self) -> Option<Rect> {
        self.area
    }

    fn set_area(&mut self, area: Rect) {
        self.area = Some(area)
    }

    fn cell_selection(&mut self, strategy: FilterMode) {
        self.cell_filter = strategy;
    }
}