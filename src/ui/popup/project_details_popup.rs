use compact_str::ToCompactString;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    prelude::{Line, StatefulWidget, Text},
    text::Span,
    widgets::{TableState, Widget},
};
use tachyonfx::RefRect;

use crate::{
    domain::{Pipeline, Project},
    theme::theme,
    ui::{fx::popup_window, popup::utility::CenteredShrink, widget::PipelineTable},
};

/// project details popup
pub struct ProjectDetailsPopup {}

/// state of the project details popup
pub struct ProjectDetailsPopupState {
    pub project: Project,
    project_namespace: Text<'static>,
    project_stat_summary: Text<'static>,
    pub pipelines: PipelineTable, // widget
    pub pipelines_table_state: TableState,
    pub popup_area: RefRect,
}

impl ProjectDetailsPopup {
    pub fn new() -> ProjectDetailsPopup {
        Self {}
    }
}

impl ProjectDetailsPopupState {
    pub fn with_project(&self, project: Project) -> Self {
        Self::new(project, self.popup_area.clone())
    }

    pub fn new(project: Project, popup_area: RefRect) -> ProjectDetailsPopupState {
        let (namespace, name) = project.path_and_name();
        let description = match &project.description {
            Some(d) => d.to_string(),
            None => String::new(),
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

        let pipelines: Vec<&Pipeline> = project.recent_pipelines();
        let pipelines = PipelineTable::new(&pipelines);

        ProjectDetailsPopupState {
            project,
            project_namespace,
            project_stat_summary,
            pipelines,
            pipelines_table_state: TableState::default().with_selected(0),
            popup_area,
        }
    }

    fn commit_count_line(commit_count: u32) -> Line<'static> {
        Line::from(vec![
            Span::from(commit_count.to_compact_string()).style(theme().project_commits[0]),
            Span::from(" commits").style(theme().project_commits[1]),
        ])
    }

    fn storage_size_line(size_kb: u64, label: &str) -> Line<'static> {
        let size = size_kb;
        let (size, unit) = match size {
            s if s < 1024 => (s as f32, "kb"),
            s if s < 1024 * 1024 => (s as f32 / 1024.0, "mb"),
            s => (s as f32 / (1024.0 * 1024.0), "gb"),
        };

        Line::from(format!("{size:.2}{unit} {label}")).style(theme().project_size[0])
    }

    pub fn update_popup_area(&self, screen: Rect) -> Rect {
        let pipeline_table_h = 2 * self.pipelines.rows.len() as u16;
        let project_details_h = 4;
        let total_height = 2 + project_details_h + pipeline_table_h;

        let a = screen.inner_centered(screen.width, total_height);
        self.popup_area.set(a);
        a
    }
}

impl StatefulWidget for ProjectDetailsPopup {
    type State = ProjectDetailsPopupState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let pipeline_table_h = 2 * state.pipelines.rows.len() as u16;
        let project_details_h = 4;

        let area = state.update_popup_area(area);

        popup_window(
            "Project Details",
            Some(vec![
                ("ESC", "close"),
                ("↑ ↓", "selection"),
                ("↵", "actions..."),
            ]),
        )
        .render(area, buf);

        let content_area = area.inner(Margin::new(2, 1));
        let outer_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(project_details_h),
                Constraint::Length(pipeline_table_h),
            ])
            .split(content_area);

        let project_details_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100), Constraint::Length(22)])
            .split(outer_layout[0]);

        state
            .project_namespace
            .clone()
            .render(project_details_layout[0], buf);
        state
            .project_stat_summary
            .clone()
            .render(project_details_layout[1], buf);

        PipelineTable::new(&state.project.recent_pipelines()).render(
            outer_layout[1],
            buf,
            &mut state.pipelines_table_state,
        );
    }
}
