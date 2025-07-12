use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    prelude::StatefulWidget,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Row, Table, TableState, Widget},
};

use crate::{
    domain::{parse_row, Project},
    theme::theme,
    ui::widget::Shortcuts,
};

/// gitlab pipelines widget
pub struct ProjectsTable<'a> {
    rows: Vec<Row<'a>>,
    filter_active: bool,
    filter_text: &'a str,
}

impl<'a> ProjectsTable<'a> {
    pub fn new(projects: &'a [Project], filter_active: bool, filter_text: &'a str) -> Self {
        Self {
            rows: projects
                .iter()
                .map(|proj| parse_row(proj))
                .enumerate()
                .map(|(idx, r)| r.style(theme().table_row(idx)))
                .collect(),
            filter_active,
            filter_text,
        }
    }
}

impl StatefulWidget for ProjectsTable<'_> {
    type State = TableState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        Clear.render(area, buf);

        let shortcuts = if self.filter_active {
            Shortcuts::from(vec![("↵", "finish filter"), ("esc", "cancel")])
        } else {
            Shortcuts::from(vec![
                ("q", "quit"),
                ("w", "open web"),
                ("c", "config"),
                ("a", "last notification"),
                ("f/", "filter"),
                ("r", "refresh"),
                ("s", "sort"),
                ("p", "pipeline refresh"),
                ("↑↓", "selection"),
                ("↵", "details"),
            ])
        };

        // Split area into main table and filter input if active
        let (table_area, filter_area) = if self.filter_active {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(3),    // Main table
                    Constraint::Length(3), // Filter input
                ])
                .split(area);
            (chunks[0], Some(chunks[1]))
        } else {
            (area, None)
        };

        // Render main table block
        Block::new()
            .title(" gitlab pipelines ")
            .title_style(theme().border_title)
            .title_bottom(shortcuts.as_line())
            .borders(if self.filter_active {
                Borders::TOP | Borders::LEFT | Borders::RIGHT
            } else {
                Borders::ALL
            })
            .border_style(theme().table_border)
            .style(theme().background)
            .border_type(BorderType::Plain)
            .render(table_area, buf);

        let content_area = table_area.inner(Margin::new(2, 1));
        let table = Table::new(self.rows, PROJECT_COLUMN_CONSTRAINTS)
            .row_highlight_style(theme().highlight_symbol)
            .column_spacing(1);

        StatefulWidget::render(table, content_area, buf, state);

        // Render filter input if active
        if let Some(filter_area) = filter_area {
            let filter_content = Line::from(vec![
                Span::from("Filter: "),
                Span::from(self.filter_text).style(theme().highlight_symbol),
                Span::from("█"), // Cursor
            ]);

            let filter_block = Block::new()
                .borders(Borders::ALL)
                .border_style(theme().table_border)
                .style(theme().background);

            Paragraph::new(filter_content)
                .block(filter_block)
                .render(filter_area, buf);
        }
    }
}

const PROJECT_COLUMN_CONSTRAINTS: [Constraint; 3] = [
    Constraint::Length(16),      // date and time
    Constraint::Min(40),         // project name
    Constraint::Percentage(100), // pipelines
];
