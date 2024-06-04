use chrono::{DateTime, Local};
use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect};
use ratatui::prelude::{Line, Span, StatefulWidget};
use ratatui::widgets::{Block, Borders, BorderType, Clear, List, ListState, Widget};
use crate::theme::theme;

/// logs widget
pub struct LogsWidget<'a> {
    logs: Vec<Line<'a>>,
}

impl<'a> LogsWidget<'a> {
    pub fn from(logs: &'a [(DateTime<Local>, &str)]) -> Self {
        Self {
            logs: logs.iter()
                .map(|(dt, log)| {
                    Line::from(vec![
                        Span::from(dt.time().format("%H:%M:%S").to_string()).style(theme().time),
                        Span::from(" "),
                        Span::from(*log).style(theme().log_message),
                    ])
                })
                .collect()
        }
    }
}

impl<'a> StatefulWidget for LogsWidget<'a> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        Clear.render(area, buf);

        Block::new()
            .title(" internal logs ")
            .title_style(theme().border_title)
            .borders(Borders::ALL)
            .border_style(theme().table_border)
            .border_type(BorderType::Plain)
            .render(area, buf);

        let content_area = area.inner(&Margin::new(2, 1));
        let logs = List::from_iter(self.logs);

        StatefulWidget::render(logs, content_area, buf, state);
    }
}
