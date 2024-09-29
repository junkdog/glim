use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect};
use ratatui::prelude::{Line, StatefulWidget};
use ratatui::widgets::{List, ListState};
use tachyonfx::{Duration, EffectRenderer};

use crate::event::GlimEvent;
use crate::id::{PipelineId, ProjectId};
use crate::theme::theme;
use crate::ui::fx::{open_window, OpenWindow};
use crate::ui::popup::utility::CenteredShrink;

/// pipeline actions popup
pub struct PipelineActionsPopup {
    last_frame_ms: Duration,
}

/// state of the pipeline actions popup
pub struct PipelineActionsPopupState {
    pub actions: Vec<GlimEvent>,
    pub project_id: ProjectId,
    pub pipeline_id: PipelineId,
    pub list_state: ListState,
    window_fx: OpenWindow,
}

impl PipelineActionsPopupState {
    pub fn new(
        actions: Vec<GlimEvent>,
        project_id: ProjectId,
        pipeline_id: PipelineId,
    ) -> Self {
        Self {
            actions,
            project_id,
            pipeline_id,
            list_state: ListState::default().with_selected(Some(0)),
            window_fx: open_window("pipeline actions", Some(vec![
                ("ESC", "close"),
                ("↑ ↓", "selection"),
                ("↵",   "apply"),
            ])),
        }
    }

    pub fn copy_action(&self) -> GlimEvent {
        match &self.actions[self.list_state.selected().unwrap()] {
            GlimEvent::BrowseToJob(id, p_id, j_id) =>
                GlimEvent::BrowseToJob(*id, *p_id, *j_id),
            GlimEvent::BrowseToPipeline(id, p_id) =>
                GlimEvent::BrowseToPipeline(*id, *p_id),
            GlimEvent::BrowseToProject(id) =>
                GlimEvent::BrowseToProject(*id),
            GlimEvent::DownloadErrorLog(id, pipeline_id) =>
                GlimEvent::DownloadErrorLog(*id, *pipeline_id),
            _ => panic!("unsupported action")
        }
    }

    fn actions_as_lines(&self) -> Vec<Line<'static>> {
        self.actions.iter()
            .map(|action| {
                let action = match action {
                    GlimEvent::BrowseToJob(_, _, _) =>
                        "browse to failed job".to_string(),
                    GlimEvent::BrowseToPipeline(_, _) =>
                        "browse to pipeline".to_string(),
                    GlimEvent::BrowseToProject(_) =>
                        "browse to project".to_string(),
                    GlimEvent::DownloadErrorLog(_, _) =>
                        "download failed job log to clipboard".to_string(),
                    _ => panic!("unsupported action")
                };
                Line::from(action).style(theme().pipeline_action)
            })
            .collect()
    }
}

impl PipelineActionsPopup {
    pub fn from(
        last_frame_ms: Duration,
    ) -> PipelineActionsPopup {
        Self { last_frame_ms }
    }

}


impl StatefulWidget for PipelineActionsPopup {
    type State = PipelineActionsPopupState;

    fn render(
        self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut Self::State
    ) {
        let area = area.inner_centered(40, 2 + state.actions.len() as u16);

        state.window_fx.screen_area(buf.area); // for the parent window fx
        let last_tick = self.last_frame_ms;
        buf.render_effect(&mut state.window_fx, area, last_tick);

        let actions = state.actions_as_lines();
        let actions_list = List::new(actions)
            .style(theme().table_row_b)
            .highlight_style(theme().pipeline_action_selected);

        let inner_area = area.inner(Margin::new(1, 1));
        StatefulWidget::render(actions_list, inner_area, buf, &mut state.list_state);

        // window decoration and animation
        state.window_fx.process_opening(self.last_frame_ms, buf, area);
    }
}
