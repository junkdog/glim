use std::vec;

use compact_str::{CompactString, ToCompactString};
use ratatui::{
    buffer::Buffer,
    layout::{Margin, Position, Rect},
    prelude::{Line, StatefulWidget, Style, Text, Widget},
    text::Span,
};
use tui_input::Input;

use crate::{
    glim_app::GlimConfig,
    logging::LoggingConfig,
    theme::theme,
    ui::{
        fx::popup_window,
        popup::utility::CenteredShrink,
        widget::{InputField, RefRect},
    },
};

/// configuration popup
pub struct ConfigPopup {}

pub struct ConfigPopupState {
    // pub duration_ms: u32,
    active_input_idx: u16,
    pub cursor_position: Position,
    input_fields: Vec<InputField>,
    pub error_message: Option<CompactString>,
    pub popup_area: RefRect,
}

impl ConfigPopup {
    pub fn new() -> Self {
        Self {}
    }
}

impl ConfigPopupState {
    pub fn new(config: GlimConfig, popup_area: RefRect) -> Self {
        let log_level_options = vec!["Trace", "Debug", "Info", "Warn", "Error"];

        let current_log_level = config
            .log_level
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("Error");
        let log_level_index = log_level_options
            .iter()
            .position(|&level| level == current_log_level)
            .unwrap_or(4); // Default to "Error"

        let mut log_level_field = InputField::builder()
            .label("log level")
            .description(Some(log_level_description()))
            .input(Input::new(current_log_level.to_string()))
            .dropdown_options(Some(log_level_options))
            .selected_option_index(log_level_index)
            .build()
            .unwrap();

        log_level_field.set_dropdown_value(current_log_level);

        Self {
            // duration_ms: 0,
            active_input_idx: 0,
            cursor_position: Position::default(),
            error_message: None,
            input_fields: vec![
                InputField::builder()
                    .label("gitlab url")
                    .description(Some(url_description()))
                    .input(Input::new(config.gitlab_url.to_string()))
                    .into(),
                InputField::builder()
                    .label("gitlab token")
                    .description(Some(token_description()))
                    .input(Input::new(config.gitlab_token.to_string()))
                    .mask_input(true)
                    .into(),
                InputField::builder()
                    .label("search filter")
                    .description(Some(filter_description()))
                    .input(Input::new(
                        config
                            .search_filter
                            .as_ref()
                            .map(|s| s.to_string())
                            .unwrap_or_default(),
                    ))
                    .into(),
                log_level_field,
            ],
            popup_area,
        }
    }

    pub fn select_next_input(&mut self) {
        self.active_input_idx = (self.active_input_idx + 1) % 4;
    }

    pub fn select_previous_input(&mut self) {
        self.active_input_idx =
            if self.active_input_idx == 0 { 3 } else { self.active_input_idx - 1 };
    }

    pub fn cycle_dropdown_next(&mut self) {
        if (self.active_input_idx as usize) < self.input_fields.len() {
            self.input_fields[self.active_input_idx as usize].cycle_dropdown_next();
        }
    }

    pub fn cycle_dropdown_prev(&mut self) {
        if (self.active_input_idx as usize) < self.input_fields.len() {
            self.input_fields[self.active_input_idx as usize].cycle_dropdown_prev();
        }
    }

    pub fn is_current_field_dropdown(&self) -> bool {
        if (self.active_input_idx as usize) < self.input_fields.len() {
            self.input_fields[self.active_input_idx as usize].is_dropdown()
        } else {
            false
        }
    }

    pub fn input(&self) -> &Input {
        &self.input_fields[self.active_input_idx as usize].input
    }

    pub fn input_mut(&mut self) -> &mut Input {
        &mut self.input_fields[self.active_input_idx as usize].input
    }

    pub fn to_config(&self) -> GlimConfig {
        let values: Vec<&str> = self
            .input_fields
            .iter()
            .map(|field| field.input.value())
            .collect();

        let gitlab_url = values
            .first()
            .unwrap_or(&"")
            .trim()
            .to_compact_string();
        let gitlab_token = values
            .get(1)
            .unwrap_or(&"")
            .trim()
            .to_compact_string();
        let search_filter_value = values.get(2).unwrap_or(&"").trim();
        let log_level_value = values.get(3).unwrap_or(&"Off").trim();

        let search_filter = if search_filter_value.is_empty() {
            None
        } else {
            Some(search_filter_value.to_compact_string())
        };

        let log_level = if log_level_value.is_empty() {
            Some("Error".to_compact_string())
        } else {
            Some(log_level_value.to_compact_string())
        };

        GlimConfig {
            gitlab_url,
            gitlab_token,
            search_filter,
            log_level,
        }
    }

    /// returns the style for the input, considering the selected input field.
    fn input_style(&self, idx: u16) -> Style {
        if idx == self.active_input_idx {
            theme().input_selected
        } else {
            theme().input
        }
    }

    fn update_cursor_position(&mut self, area: &Rect) {
        let input = self.input();
        self.cursor_position = Position::new(
            area.x + 1 + input.cursor() as u16,
            area.y + 3 + self.active_input_idx * 3, // 3 elements per input field
        );
    }

    pub fn update_popup_area(&self, screen: Rect) -> Rect {
        let area = screen.inner_centered(80, 15);
        self.popup_area.set(area);
        area
    }
}

impl StatefulWidget for ConfigPopup {
    type State = ConfigPopupState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let area = state.update_popup_area(area);

        popup_window(
            "Configuration",
            Some(vec![("ESC", "close"), ("↑ ↓", "navigate"), ("← →", "change"), ("↵", "apply")]),
        )
        .render(area, buf);

        // popup content
        let content_area = area.inner(Margin::new(1, 1));
        let mut text: Vec<Line> = state
            .input_fields
            .iter()
            .enumerate()
            .flat_map(|(idx, input_field)| {
                [
                    Line::from(input_field.label).style(theme().input_label),
                    input_field
                        .description
                        .clone()
                        .unwrap_or_else(|| Line::from("")),
                    Line::from(input_field.sanitized_input_display().to_string())
                        .style(state.input_style(idx as u16)),
                ]
            })
            .collect();

        if let Some(error_message) = &state.error_message {
            text.push(Line::from(error_message.to_string()).style(theme().configuration_error));
        }

        Widget::render(Text::from(text), content_area, buf);

        state.update_cursor_position(&area);
    }
}

fn url_description() -> Line<'static> {
    Line::from(vec![
        Span::from("base url of the gitlab instance, e.g. ").style(theme().input_description),
        Span::from("https://mygitlab.com/api/v4").style(theme().input_description_em),
    ])
}

fn token_description() -> Line<'static> {
    Line::from(vec![
        Span::from("personal access token ").style(theme().input_description_em),
        Span::from("for the gitlab api; scoped to ").style(theme().input_description),
        Span::from("read_api").style(theme().input_description_em),
    ])
}

fn filter_description() -> Line<'static> {
    Line::from(vec![Span::from("optional project filter, applied to project namespace")
        .style(theme().input_description)])
}

fn log_level_description() -> Line<'static> {
    let log_dir = LoggingConfig::default_log_dir();
    let log_path = log_dir.to_string_lossy().into_owned();
    Line::from(vec![
        Span::from("logs saved to ").style(theme().input_description),
        Span::from(log_path).style(theme().input_description_em),
    ])
}
