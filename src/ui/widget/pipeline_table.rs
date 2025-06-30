use crate::domain::{IconRepresentable, Pipeline};
use crate::id::PipelineId;
use crate::theme::theme;
use crate::ui::format_duration;
use crate::ui::widget::text_from;
use chrono::Local;
use compact_str::ToCompactString;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Rect};
use ratatui::prelude::{Line, Span, StatefulWidget, Text};
use ratatui::widgets::{Cell, Row, Table, TableState};

/// pipelines widget. used inside the project details popup.
///
/// Each pipeline is represented as a row in the table, with the following format:
/// ```
/// #BRANCH| PIPELNE/JOB | TIME   | %DONE | COMMENT
/// main   | 🔵🔵🔵🔵🔵 | 14m24s  | ~72%  | Merge branch 'renovate/all-minor-dependencies'
///        | deploy-prod |  3m23s | ~40%  |  into 'main'
/// ```
#[derive(Clone)]
pub struct PipelineTable {
    pub constraints: [Constraint; 5],
    pub rows: Vec<Row<'static>>,
    pub ids: Vec<PipelineId>,
}

impl PipelineTable {
    pub fn new(pipelines: &[&Pipeline]) -> Self {
        let (max_branch, max_job_name, max_failed_job_name, max_duration) =
            pipelines.iter().fold((5, 12, 12, 4), |(b, j, f, d), p| {
                (
                    b.max(p.branch.chars().count()),
                    j.max(p.active_job_name().chars().count())
                        .max(p.jobs.clone().map(|j| j.len() * 2).unwrap_or(0)),
                    f.max(p.failing_job_name().map(|j| j.chars().count()).unwrap_or(0)),
                    d.max(format_duration(p.duration()).chars().count()),
                    // pe.max("NA%".chars().count()),
                )
            });

        Self {
            constraints: [
                Constraint::Length(12),
                Constraint::Length(max_branch as u16),
                Constraint::Length(max_job_name.max(max_failed_job_name) as u16),
                Constraint::Length(max_duration as u16),
                Constraint::Percentage(100),
            ],
            rows: pipelines
                .iter()
                .map(|p| Self::parse_row(p))
                .enumerate()
                .map(|(idx, r)| r.style(theme().table_row(idx)))
                .collect(),
            ids: pipelines.iter().map(|p| p.id).collect(),
        }
    }

    fn parse_row(p: &Pipeline) -> Row<'static> {
        let branch = p.branch.clone();

        let comment = if let Some(commit) = &p.commit {
            commit.title.clone()
        } else {
"".to_compact_string()
        };

        let branch_text = branch.to_string();
        let source_text = p.source.to_string().to_string();
        let branch_cell = Cell::from(Text::from(vec![
            Line::from(branch_text).style(theme().pipeline_branch),
            Line::from(source_text).style(theme().pipeline_source),
        ]));

        Row::new(vec![
            Cell::from(text_from(p.created_at.with_timezone(&Local))),
            branch_cell,
            Self::pipeline_jobs_cell(p),
            Self::pipeline_duration_cell(p),
            // Self::pipeline_percentages_cell(p),
            Cell::from(Span::from(comment).style(theme().commit_title)),
        ])
        .height(2)
    }

    fn pipeline_jobs_cell(p: &Pipeline) -> Cell<'static> {
        // let branch_name = if Some(p.failing_job_name()) {
        //
        // }

        let branch_name = if let Some(name) = p.failing_job_name() {
            Line::from(name.to_string()).style(theme().pipeline_job_failed)
        } else {
            Line::from(p.active_job_name().to_string()).style(theme().pipeline_job)
        };

        let content = Text::from(vec![Line::from(p.icon().to_string()), branch_name]);

        Cell::from(content)
    }

    fn pipeline_duration_cell(p: &Pipeline) -> Cell<'static> {
        let active_job_duration = p
            .active_job()
            .map(|j| j.duration())
            .map(format_duration)
            .unwrap_or("".to_compact_string());

        let duration = p.duration();
        let content = Text::from(vec![
            Line::from(format_duration(duration).to_string())
                .style(theme().time)
                .alignment(Alignment::Right),
            Line::from(active_job_duration.to_string())
                .style(theme().time)
                .alignment(Alignment::Right),
        ]);

        Cell::from(content)
    }
}

impl StatefulWidget for PipelineTable {
    type State = TableState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let table = Table::new(self.rows, self.constraints)
            .row_highlight_style(theme().highlight_symbol)
            .column_spacing(1);

        StatefulWidget::render(table, area, buf, state);
    }
}
