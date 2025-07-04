use std::vec;

use compact_str::{CompactString, ToCompactString};
use itertools::Itertools;
use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Position, Rect};
use ratatui::prelude::{Line, StatefulWidget, Style, Text, Widget};
use ratatui::text::Span;
use tachyonfx::{Duration, EffectRenderer, Shader};
use tui_input::Input;

use crate::glim_app::GlimConfig;
use crate::theme::theme;
use crate::ui::fx::{open_window, PopupWindow};
use crate::ui::popup::utility::CenteredShrink;
use crate::ui::widget::InputField;

/// configuration popup
pub struct ConfigPopup {
    last_frame_time: Duration,
}

pub struct ConfigPopupState {
    // pub duration_ms: u32,
    active_input_idx: u16,
    pub cursor_position: Position,
    input_fields: Vec<InputField>,
    pub error_message: Option<CompactString>,
    window_fx: PopupWindow,
}

impl ConfigPopup {
    pub fn new(last_frame_time: Duration) -> Self {
        Self { last_frame_time }
    }
}

impl ConfigPopupState {
    pub fn new(config: GlimConfig) -> Self {
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
                        config.search_filter.as_ref().map(|s| s.to_string()).unwrap_or_default(),
                    ))
                    .into(),
            ],
            window_fx: open_window(
                "configuration",
                Some(vec![("ESC", "close"), ("↑ ↓", "selection"), ("↵", "apply")]),
            ),
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
        let (gitlab_url, gitlab_token, search_filter) = self
            .input_fields
            .iter()
            .map(|field| field.input.value())
            .collect_tuple()
            .unwrap();

        let search_filter = if search_filter.trim().is_empty() {
            None
        } else {
            Some(search_filter.trim().to_compact_string())
        };

        GlimConfig {
            gitlab_url: gitlab_url.trim().to_compact_string(),
            gitlab_token: gitlab_token.trim().to_compact_string(),
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

        // window decoration and animation
        state
            .window_fx
            .process_opening(self.last_frame_time, buf, area);
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
    Line::from(vec![Span::from(
        "optional project filter, applied to project namespace",
    )
    .style(theme().input_description)])
}
