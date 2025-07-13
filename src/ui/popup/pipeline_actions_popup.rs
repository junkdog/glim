use ratatui::{
    buffer::Buffer,
    layout::{Margin, Rect},
    prelude::{Line, StatefulWidget, Widget},
    widgets::{List, ListState},
};

use crate::{
    event::GlimEvent,
    theme::theme,
    ui::{fx::popup_window, popup::utility::CenteredShrink, widget::RefRect},
};

/// pipeline actions popup
pub struct PipelineActionsPopup {}

/// state of the pipeline actions popup
pub struct PipelineActionsPopupState {
    pub actions: Vec<GlimEvent>,
    pub list_state: ListState,
    popup_area: RefRect,
}

impl PipelineActionsPopupState {
    pub fn new(
        actions: Vec<GlimEvent>,
        popup_area: RefRect,
    ) -> Self {
        Self {
            actions,
            list_state: ListState::default().with_selected(Some(0)),
            popup_area,
        }
    }

    pub fn copy_selected_action(&self, selected_action: usize) -> GlimEvent {
        self.actions[selected_action].clone()
    }

    fn actions_as_lines(&self) -> Vec<Line<'static>> {
        self.actions
            .iter()
            .map(|action| {
                let action = match action {
                    GlimEvent::BrowseToJob(_, _, _) => "browse to failed job",
                    GlimEvent::BrowseToPipeline(_, _) => "browse to pipeline",
                    GlimEvent::BrowseToProject(_) => "browse to project",
                    GlimEvent::DownloadErrorLog(_, _) => "download failed job log to clipboard",
                    _ => panic!("unsupported action"),
                };
                Line::from(action).style(theme().pipeline_action)
            })
            .collect()
    }

    pub fn update_popup_area(&self, screen: Rect) -> Rect {
        let area = screen.inner_centered(40, 2 + self.actions.len() as u16);
        self.popup_area.set(area);
        area
    }
}

impl PipelineActionsPopup {
    pub fn new() -> PipelineActionsPopup {
        Self {}
    }
}

impl StatefulWidget for PipelineActionsPopup {
    type State = PipelineActionsPopupState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let area = state.update_popup_area(area);

        popup_window(
            "Pipeline Actions",
            Some(vec![("ESC", "close"), ("↑ ↓", "selection"), ("↵", "apply")]),
        )
        .render(area, buf);

        let actions = state.actions_as_lines();
        let actions_list = List::new(actions)
            .style(theme().table_row_b)
            .highlight_style(theme().pipeline_action_selected);

        let inner_area = area.inner(Margin::new(1, 1));
        StatefulWidget::render(actions_list, inner_area, buf, &mut state.list_state);
    }
}
