use compact_str::CompactString;
use ratatui::{
    buffer::Buffer,
    layout::{Margin, Rect},
    prelude::StatefulWidget,
    text::{Line, Span},
    widgets::{Block, Clear, Widget},
};

use crate::{
    notice_service::{Notice, NoticeMessage},
    stores::ProjectStore,
    theme::theme,
    ui::widget::RefRect,
};

#[derive(Clone)]
pub struct Notification {}

pub struct NotificationState {
    pub notice: Notice,
    project_name: Option<CompactString>,
    content_area: RefRect,
}

impl NotificationState {
    pub fn new(notice: Notice, project_lookup: &ProjectStore, content_area: RefRect) -> Self {
        let project_name = match notice.message {
            NoticeMessage::GeneralMessage(_)
            | NoticeMessage::ConfigError(_)
            | NoticeMessage::JsonDeserializeError(_, _)
            | NoticeMessage::ScreenCaptured => None,

            NoticeMessage::JobLogDownloaded(id, _, _)
            | NoticeMessage::GitlabGetJobsError(id, _, _)
            | NoticeMessage::GitlabGetTriggerJobsError(id, _, _)
            | NoticeMessage::GitlabGetPipelinesError(id, _, _) => {
                project_lookup.find(id).map(|p| p.title())
            },
        };

        Self { notice, project_name, content_area }
    }
}

impl Notification {
    pub fn new() -> Self {
        Self {}
    }
}

impl StatefulWidget for Notification {
    type State = NotificationState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let project: &str = if let Some(p) = &state.project_name { p } else { "<unknown project>" };

        let text: Line<'_> = match &state.notice.message {
            NoticeMessage::GeneralMessage(s) => Line::from(Span::from(s)),
            NoticeMessage::ConfigError(s) => {
                Line::from(vec![Span::from("Config error: "), Span::from(s)])
            },
            NoticeMessage::JsonDeserializeError(cat, s) => Line::from(vec![
                Span::from("Failed to parse JSON ("),
                Span::from(format!("{cat:?}")),
                Span::from(")"),
                Span::from(s),
            ]),
            NoticeMessage::GitlabGetJobsError(_, _, s) => Line::from(vec![
                Span::from("Failed to get jobs for "),
                Span::from(project).style(theme().notification_project),
                Span::from(": "),
                Span::from(s),
            ]),
            NoticeMessage::GitlabGetTriggerJobsError(_, _, s) => Line::from(vec![
                Span::from("Failed to get trigger jobs for "),
                Span::from(project).style(theme().notification_project),
                Span::from(": "),
                Span::from(s),
            ]),
            NoticeMessage::GitlabGetPipelinesError(_, _, s) => Line::from(vec![
                Span::from("Failed to get pipelines for "),
                Span::from(project).style(theme().notification_project),
                Span::from(": "),
                Span::from(s),
            ]),
            NoticeMessage::JobLogDownloaded(_, _, _) => Line::from(vec![
                Span::from("Finished downloading job log for "),
                Span::from(project).style(theme().notification_project),
            ]),
            NoticeMessage::ScreenCaptured => {
                Line::from(vec![Span::from("Screen contents copied to clipboard")])
            },
        };

        let text_len = (text.width() as u16).min(area.width - 2);
        let content_area = Rect {
            x: area.x + (area.width - text_len) / 2 - 1,
            y: area.y,
            width: text_len + 2,
            height: 1,
        };
        state.content_area.set(content_area);

        Clear.render(content_area, buf);
        Block::new()
            .style(theme().notification)
            .render(content_area, buf);

        text.render(content_area.inner(Margin::new(1, 0)), buf);
    }
}
