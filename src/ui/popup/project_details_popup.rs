use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::prelude::{Line, StatefulWidget, Text};
use ratatui::text::Span;
use ratatui::widgets::{TableState, Widget};

use crate::domain::{Pipeline, Project};
use crate::shader::{open_window, RenderEffect};
use crate::shader::fx::OpenWindow;
use crate::theme::theme;
use crate::ui::popup::utility::CenteredShrink;
use crate::ui::widget::PipelineTable;

/// project details popup
pub struct ProjectDetailsPopup {
    last_frame_ms: u32,
}

/// state of the project details popup
pub struct ProjectDetailsPopupState {
    pub project: Project,
    // duration_ms: u32,
    project_namespace: Text<'static>,
    project_stat_summary: Text<'static>,
    last_activity: Line<'static>,
    pub pipelines: PipelineTable, // widget
    pub pipelines_table_state: TableState,
    window_fx: OpenWindow,
}

impl ProjectDetailsPopup {
    pub fn new(
        last_frame_ms: u32
    ) -> ProjectDetailsPopup {
        Self {
            last_frame_ms
        }
    }
}

impl ProjectDetailsPopupState {
    pub fn with_project(&self, project: Project) -> Self {
        let mut state = Self::new(project);
        state.window_fx = self.window_fx.clone();
        state
    }

    pub fn new(
        project: Project,
    ) -> ProjectDetailsPopupState {
        let (namespace, name) = project.path_and_name();
        let description = match &project.description {
            Some(d) => d.clone(),
            None => "".to_string(),
        };
        let project_namespace = Text::from(vec![
            Line::from(name.to_string()).style(theme().project_name),
            Line::from(namespace.to_string()).style(theme().project_parents),
            Line::from(description).style(theme().project_description),
        ]);

        let project_stat_summary = Text::from(vec![
            Self::commit_count_line(project.commit_count),
            Self::storage_size_line(project.repo_size_kb, "in repository"),
            Self::storage_size_line(project.artifacts_size_kb, "in artifacts"),
        ]);

        let last_activity = Line::from(project.last_activity_at.to_string());

        let pipelines: Vec<&Pipeline> = project.recent_pipelines();
        let pipelines = PipelineTable::new(&pipelines);

        ProjectDetailsPopupState {
            project,
            project_namespace,
            project_stat_summary,
            last_activity,
            pipelines,
            pipelines_table_state: TableState::default().with_selected(0),
            window_fx: open_window("project details", Some(vec![
                ("ESC", "close"),
                ("↑ ↓", "selection"),
                ("↵",   "actions..."),
            ])),
        }
    }

    fn commit_count_line(commit_count: u32) -> Line<'static> {
        Line::from(vec![
            Span::from(commit_count.to_string())
                .style(theme().project_commits[0]),
            Span::from(" commits")
                .style(theme().project_commits[1]),
        ])
    }

    fn storage_size_line(size_kb: u64, label: &str) -> Line<'static> {
        let size = size_kb;
        let (size, unit) = match size {
            s if s < 1024        => (s as f32, "kb"),
            s if s < 1024 * 1024 => (s as f32 / 1024.0, "mb"),
            s                    => (s as f32 / (1024.0 * 1024.0), "gb"),
        };

        Line::from(vec![
            Span::from(format!("{:.2}{unit} ", size))
                .style(theme().project_size[0]),
            Span::from(label.to_string())
                .style(theme().project_size[1]),
        ])
    }

    pub fn popup_area(&self, screen: Rect) -> Rect {
        let pipeline_table_h = 2 * self.pipelines.rows.len() as u16;
        let project_details_h = 4;
        let total_height = 2 + project_details_h + pipeline_table_h;

        screen.inner_centered(screen.width, total_height)
    }
}

impl StatefulWidget for ProjectDetailsPopup {
    type State = ProjectDetailsPopupState;

    fn render(
        self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut Self::State
    ) {
        let pipeline_table_h = 2 * state.pipelines.rows.len() as u16;
        let project_details_h = 4;

        let area = state.popup_area(area);

        state.window_fx.screen_area(buf.area); // for the parent window fx
        buf.render_effect(&mut state.window_fx, area, self.last_frame_ms);
        
        let content_area = area.inner(&Margin::new(2, 1));
        let outer_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(project_details_h),
                Constraint::Length(pipeline_table_h),
            ])
            .split(content_area);

        let project_details_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(100),
                Constraint::Length(22),
            ])
            .split(outer_layout[0]);

        state.project_namespace.clone()
            .render(project_details_layout[0], buf);
        state.project_stat_summary.clone()
            .render(project_details_layout[1], buf);

        PipelineTable::new(&state.project.recent_pipelines())
            .render(outer_layout[1], buf, &mut state.pipelines_table_state);

        state.window_fx.process_opening(self.last_frame_ms, buf, area);
    }
}
