use crate::notice_service::{Notice, NoticeMessage};
use crate::stores::ProjectStore;
use crate::theme::theme;
use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect};
use ratatui::prelude::StatefulWidget;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Widget};
use tachyonfx::{Duration, Effect, Shader};

#[derive(Clone)]
pub struct Notification {
    last_tick: Duration,
}

pub struct NotificationState {
    pub notice: Notice,
    project_name: Option<String>,
    effect: Effect,
}

impl NotificationState {
    pub fn is_done(&self) -> bool {
        self.effect.done()
    }
}


impl NotificationState {
    pub fn new(
        notice: Notice,
        project_lookup: &ProjectStore,
    ) -> Self {
        let project_name = match notice.message {
            NoticeMessage::GeneralMessage(_) |
            NoticeMessage::ConfigError(_) |
            NoticeMessage::JsonDeserializeError(_, _) => None,
            NoticeMessage::JobLogDownloaded(id, _, _) |
            NoticeMessage::GitlabGetJobsError(id, _, _) |
            NoticeMessage::GitlabGetTriggerJobsError(id, _, _) |
            NoticeMessage::GitlabGetPipelinesError(id, _, _) => project_lookup.find(id).map(|p| p.title()),
        };

        Self {
            notice,
            project_name,
            effect: effect::notification_effect(),
        }
    }
}

impl Notification {
    pub fn new(
        last_tick: Duration,
    ) -> Self {
        Self { last_tick }
    }
}

impl StatefulWidget for Notification {
    type State = NotificationState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let project: &str = if let Some(p) = &state.project_name { p } else { "" };

        let text: Line<'_> = match &state.notice.message {
            NoticeMessage::GeneralMessage(s) => Line::from(Span::from(s)),
            NoticeMessage::ConfigError(s) => Line::from(vec![
                Span::from("Config error: "),
                Span::from(s),
            ]),
            NoticeMessage::JsonDeserializeError(cat, s) => Line::from(vec![
                Span::from("Failed to parse JSON ("),
                Span::from(format!("{:?}", cat)),
                Span::from(")"),
                Span::from(s),
            ]),
            NoticeMessage::GitlabGetJobsError(_, _, s) => Line::from(vec![
                Span::from("Failed to get jobs for "),
                Span::from(project)
                    .style(theme().notification_project),
                Span::from(": "),
                Span::from(s),
            ]),
            NoticeMessage::GitlabGetTriggerJobsError(_, _, s) => Line::from(vec![
                Span::from("Failed to get trigger jobs for "),
                Span::from(project)
                    .style(theme().notification_project),
                Span::from(": "),
                Span::from(s),
            ]),
            NoticeMessage::GitlabGetPipelinesError(_, _, s) => Line::from(vec![
                Span::from("Failed to get pipelines for "),
                Span::from(project)
                    .style(theme().notification_project),
                Span::from(": "),
                Span::from(s),
            ]),
            NoticeMessage::JobLogDownloaded(_, _, _) => Line::from(vec![
                Span::from("Finished downloading job log for "),
                Span::from(project)
                    .style(theme().notification_project),
            ]),
        };

        let text_len = text.width() as u16;
        let content_area = Rect {
            x: area.x + (area.width.saturating_sub(text_len)) / 2 - 1,
            y: area.y,
            width: text_len + 2,
            height: 1,
        };

        Clear.render(content_area, buf);
        Block::new()
            .style(theme().notification)
            .render(content_area, buf);

        text.render(content_area.inner(Margin::new(1, 0)), buf);
        state.effect.process(self.last_tick, buf, content_area);
    }
}

mod effect {
    use crate::gruvbox::Gruvbox::Dark0Hard;
    use tachyonfx::Interpolation::{SineIn, SineOut};
    use tachyonfx::{fx, Duration, Effect};

    pub(super) fn notification_effect() -> Effect {
        fx::sequence(&[
            // 1. clear the border (border is already cleared, so we first fill it back in)
            fx::parallel(&[
                draw_border(Duration::from_millis(100)),
                fx::dissolve(Duration::from_millis(100))
            ]),
            // 2. fade in notification text
            fx::fade_from_fg(Dark0Hard, (250, SineOut)),
            // 3. smooth blink while notification is shown
            fx::with_duration(Duration::from_millis(6000),
                fx::repeating(fx::ping_pong(
                    fx::hsl_shift_fg([0.0, 0.0, 25.0], (500, SineOut))
                )),
            ),
            // 4. fade out notification text and then redraw border
            fx::prolong_end(Duration::from_millis(100),
                fx::fade_to_fg(Dark0Hard, (250, SineIn))),
            fx::parallel(&[
                draw_border(Duration::from_millis(150)),
                fx::coalesce(150),
            ]),
        ])
    }

    fn draw_border(duration: Duration) -> Effect {
        fx::effect_fn((), duration, |_, _, cells| {
            cells.for_each(|(_, cell)| { cell.set_char('─'); });
        })
    }
}
