use derive_builder::Builder;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::prelude::Line;
use ratatui::style::Style;
use ratatui::widgets::WidgetRef;
use ratatui::widgets::Widget;
use tui_input::Input;

#[derive(Clone, Builder)]
#[builder(pattern = "owned")]
pub struct InputField {
    pub label: &'static str,
    pub description: Option<Line<'static>>,
    pub input: Input,
    input_style: Style,
    #[builder(default)]
    mask_input: bool,
}

impl InputField {
    pub fn builder() -> InputFieldBuilder {
        InputFieldBuilder::default()
    }

    pub fn sanitized_input_display(&self) -> String {
        if self.mask_input {
            self.input.value().chars().map(|_| '*').collect()
        } else {
            self.input.value().to_string()
        }
    }
}

impl From<InputFieldBuilder> for InputField {
    fn from(value: InputFieldBuilder) -> Self {
        value.build().unwrap()
    }
}

impl WidgetRef for InputField {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        if let Some(description) = &self.description {
            let mut rows = area.rows();
            rows.next().map(|row| self.label.render_ref(row, buf));
            rows.next().map(|row| description.render_ref(row, buf));
            rows.next().map(|row| {
                let input = self.sanitized_input_display();
                Line::from(input).style(self.input_style).render(row, buf);
            });
        } else {
            self.label.render_ref(area, buf);
            let label_width = self.label.width();
            buf.cell_mut(Position::new(area.x + label_width, area.y)).set_char(':');
        }
    }
}