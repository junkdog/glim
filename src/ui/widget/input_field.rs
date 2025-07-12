use compact_str::CompactString;
use derive_builder::Builder;
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    prelude::Line,
    style::Style,
    widgets::{Widget, WidgetRef},
};
use tui_input::Input;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Builder)]
#[builder(pattern = "owned")]
pub struct InputField {
    pub label: &'static str,
    pub description: Option<Line<'static>>,
    pub input: Input,
    #[builder(default)]
    input_style: Style,
    #[builder(default)]
    mask_input: bool,
}

impl InputField {
    pub fn builder() -> InputFieldBuilder {
        InputFieldBuilder::default()
    }

    pub fn sanitized_input_display(&self) -> CompactString {
        if self.mask_input {
            self.input
                .value()
                .chars()
                .map(|_| '*')
                .collect::<CompactString>()
        } else {
            self.input.value().into()
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
            if let Some(row) = rows.next() {
                self.label.render_ref(row, buf)
            }
            if let Some(row) = rows.next() {
                description.render_ref(row, buf)
            }
            rows.next().map(|row| {
                let input = self.sanitized_input_display();
                Line::from(input.to_string())
                    .style(self.input_style)
                    .render(row, buf);
            });
        } else {
            self.label.render_ref(area, buf);
            let label_width = self.label.width();
            if let Some(cell) = buf.cell_mut(Position::new(area.x + label_width as u16, area.y)) {
                cell.set_char(':');
            }
        }
    }
}
