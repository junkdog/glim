use chrono::Duration;
use ratatui::buffer::Buffer;
use ratatui::Frame;
use ratatui::layout::Rect;
use crate::shader::shader::Shader;

pub trait RenderEffect<T> {
    fn render_effect(
        &mut self,
        effect: &mut T,
        area: Rect,
        last_frame_ms: u32
    );
}

impl<S: Shader> RenderEffect<S> for Frame<'_> {
    fn render_effect(
        &mut self,
        effect: &mut S,
        area: Rect,
        last_frame_ms: u32
    ) {
        render_effect(effect, self.buffer_mut(), area, last_frame_ms);
    }
}


impl<S: Shader> RenderEffect<S> for Buffer {
    fn render_effect(
        &mut self,
        effect: &mut S,
        area: Rect,
        last_frame_ms: u32
    ) {
        render_effect(effect, self, area, last_frame_ms);
    }
}

fn render_effect<S: Shader>(
    // effect: &mut Effect,
    effect: &mut S,
    buf: &mut Buffer,
    area: Rect,
    last_frame_ms: u32,
) {
    effect.process(
        Duration::milliseconds(last_frame_ms as i64),
        buf,
        area
    );
}
