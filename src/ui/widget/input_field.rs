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
    #[builder(default)]
    dropdown_options: Option<Vec<&'static str>>,
    #[builder(default)]
    selected_option_index: usize,
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
        } else if let Some(options) = &self.dropdown_options {
            if self.selected_option_index < options.len() {
                format!("{} (←→ to change)", options[self.selected_option_index]).into()
            } else {
                "Invalid selection".into()
            }
        } else {
            self.input.value().into()
        }
    }

    pub fn is_dropdown(&self) -> bool {
        self.dropdown_options.is_some()
    }

    pub fn cycle_dropdown_next(&mut self) {
        if let Some(options) = &self.dropdown_options {
            self.selected_option_index = (self.selected_option_index + 1) % options.len();
            if let Some(selected_value) = options.get(self.selected_option_index) {
                self.input = Input::new(selected_value.to_string());
            }
        }
    }

    pub fn cycle_dropdown_prev(&mut self) {
        if let Some(options) = &self.dropdown_options {
            self.selected_option_index = if self.selected_option_index == 0 {
                options.len() - 1
            } else {
                self.selected_option_index - 1
            };
            if let Some(selected_value) = options.get(self.selected_option_index) {
                self.input = Input::new(selected_value.to_string());
            }
        }
    }

    pub fn set_dropdown_value(&mut self, value: &str) {
        if let Some(options) = &self.dropdown_options {
            if let Some(index) = options.iter().position(|&option| option == value) {
                self.selected_option_index = index;
                self.input = Input::new(value.to_string());
            }
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
