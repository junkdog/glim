use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Margin, Rect};
use ratatui::prelude::StatefulWidget;
use ratatui::widgets::{Block, Borders, BorderType, Clear, Row, Table, TableState, Widget};
use crate::domain::{parse_row, Project};
use crate::theme::theme;
use crate::ui::widget::Shortcuts;

/// gitlab pipelines widget
pub struct ProjectsTable<'a> {
    rows: Vec<Row<'a>>,
}

impl<'a> ProjectsTable<'a> {
    pub fn new(
        projects: &'a [Project]
    ) -> Self {
        Self {
            rows: projects.iter()
                .map(|proj| parse_row(proj))
                .enumerate()
                .map(|(idx, r)| r.style(theme().table_row(idx)))
                .collect()
        }
    }
}

impl StatefulWidget for ProjectsTable<'_> {
    type State = TableState;

    fn render(
        self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut Self::State
    ) {
        Clear.render(area, buf);

        let shortcuts = Shortcuts::from(vec![
            ("q",   "quit"),
            ("w",   "open web"),
            ("c",   "config"),
            ("a",   "last notification"),
            ("l",   "logs"),
            ("r",   "refresh"),
            ("p",   "pipeline refresh"),
            ("↑ ↓", "selection"),
            ("↵",   "details"),
        ]);

        Block::new()
            .title(" gitlab pipelines ")
            .title_style(theme().border_title)
            .title_bottom(shortcuts.as_line())
            .borders(Borders::ALL)
            .border_style(theme().table_border)
            .style(theme().background)
            .border_type(BorderType::Plain)
            .render(area, buf);

        let content_area = area.inner(Margin::new(2, 1));
        let table = Table::new(self.rows, PROJECT_COLUMN_CONSTRAINTS)
            .highlight_style(theme().highlight_symbol)
            .column_spacing(1);

        StatefulWidget::render(table, content_area, buf, state);
    }
}

const PROJECT_COLUMN_CONSTRAINTS: [Constraint; 3] = [
    Constraint::Length(16),      // date and time
    Constraint::Min(40),         // project name
    Constraint::Percentage(100), // pipelines
];
