use std::vec;

use derive_builder::Builder;
use itertools::Itertools;
use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Position, Rect};
use ratatui::prelude::{Line, StatefulWidget, Style, Text, Widget};
use ratatui::text::Span;
use tachyonfx::{Duration, EffectRenderer, Shader};
use tui_input::Input;

use crate::glim_app::GlimConfig;
use crate::theme::theme;
use crate::ui::fx::{open_window, OpenWindow};
use crate::ui::popup::utility::CenteredShrink;

/// configuration popup
pub struct ConfigPopup {
    last_frame_time: Duration,
}

pub struct ConfigPopupState {
    // pub duration_ms: u32,
    active_input_idx: u16,
    pub cursor_position: Position,
    input_fields: Vec<InputField>,
    pub error_message: Option<String>,
    window_fx: OpenWindow,
}

impl ConfigPopup {
    pub fn new(last_frame_time: Duration) -> Self {
        Self { last_frame_time }
    }
}

#[derive(Clone, Builder)]
#[builder(pattern = "owned")]
pub struct InputField {
    label: &'static str,
    description: Line<'static>,
    input: Input,
    #[builder(default)]
    mask_input: bool,
}

impl InputField {
    fn builder() -> InputFieldBuilder {
        InputFieldBuilder::default()
    }

    fn sanitized_input_display(&self) -> String {
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

impl ConfigPopupState {
    pub fn new(
        config: &GlimConfig
    ) -> Self {
        Self {
            // duration_ms: 0,
            active_input_idx: 0,
            cursor_position: Position::default(),
            error_message: None,
            input_fields: vec![
                InputField::builder()
                    .label("gitlab url")
                    .description(url_description())
                    .input(Input::new(config.gitlab_url.clone()))
                    .into(),
                InputField::builder()
                    .label("gitlab token")
                    .description(token_description())
                    .input(Input::new(config.gitlab_token.clone()))
                    .mask_input(true)
                    .into(),
                InputField::builder()
                    .label("search filter")
                    .description(filter_description())
                    .input(Input::new(config.search_filter.clone().unwrap_or("".to_string())))
                    .into(),
            ],
            window_fx: open_window("configuration", Some(vec![
                ("ESC", "close"),
                ("↑ ↓", "selection"),
                ("↵",   "apply"),
            ])),
        }
    }

    pub fn is_open_complete(&self) -> bool {
        self.window_fx.done()
    }
    
    pub fn select_next_input(&mut self) {
        self.active_input_idx = (self.active_input_idx + 1) % 3;
    }

    pub fn select_previous_input(&mut self) {
        self.active_input_idx = if self.active_input_idx == 0 {
            2
        } else {
            self.active_input_idx - 1
        };
    }

    pub fn input(&self) -> &Input {
        &self.input_fields[self.active_input_idx as usize].input
    }

    pub fn input_mut(&mut self) -> &mut Input {
        &mut self.input_fields[self.active_input_idx as usize].input
    }

    pub fn to_config(&self) -> GlimConfig {
        let (gitlab_url, gitlab_token, search_filter) = self.input_fields.iter()
            .map(|field| field.input.value())
            .collect_tuple()
            .unwrap();

        let search_filter = if search_filter.trim().is_empty() {
            None
        } else {
            Some(search_filter.trim().to_string())
        };

        GlimConfig {
            gitlab_url: gitlab_url.trim().to_string(),
            gitlab_token: gitlab_token.trim().to_string(),
            search_filter,
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
}

impl StatefulWidget for ConfigPopup {
    type State = ConfigPopupState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let area = area.inner_centered(80, 12);

        state.window_fx.screen_area(buf.area); // for the parent window fx
        let last_tick = self.last_frame_time;
        buf.render_effect(&mut state.window_fx, area, last_tick);
        
        // popup content
        let content_area = area.inner(Margin::new(1, 1));
        let mut text: Vec<Line> = state.input_fields.iter()
            .enumerate()
            .flat_map(|(idx, input_field)| {[
                Line::from(input_field.label).style(theme().input_label),
                input_field.description.clone(),
                Line::from(input_field.sanitized_input_display()).style(state.input_style(idx as u16)),
            ]})
            .collect();

        if let Some(error_message) = &state.error_message {
            text.push(Line::from(error_message.clone()).style(theme().configuration_error));
        }

        Widget::render(Text::from(text), content_area, buf);

        // window decoration and animation
        state.window_fx.process_opening(self.last_frame_time, buf, area);
        state.update_cursor_position(&area);
    }
}

fn url_description() -> Line<'static> {
    Line::from(vec![
        Span::from("base url of the gitlab instance, e.g. ")
            .style(theme().input_description),
        Span::from("https://mygitlab.com/api/v4")
            .style(theme().input_description_em),
    ])
}

fn token_description() -> Line<'static> {
    Line::from(vec![
        Span::from("personal access token ")
            .style(theme().input_description_em),
        Span::from("for the gitlab api; scoped to ")
            .style(theme().input_description),
        Span::from("read_api")
            .style(theme().input_description_em),
    ])
}

fn filter_description() -> Line<'static> {
    Line::from(vec![
        Span::from("optional project filter, applied to project namespace")
            .style(theme().input_description),
    ])
}