use chrono::Local;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, StatefulWidget, Widget},
};
use tui_input::Input;

use crate::{
    domain::{MrNote, MrView},
    id::ProjectId,
    theme::theme,
};

#[derive(Debug, Clone, PartialEq)]
pub enum MrViewMode {
    Scrolling,
    Composing,
    AtlantisAction { note_idx: usize, selected: usize },
}

pub struct MrViewState {
    pub project_id: ProjectId,
    pub mr: Option<MrView>,
    pub loading: bool,
    pub list_state: ListState,
    pub mode: MrViewMode,
    pub comment_input: Input,
}

impl MrViewState {
    pub fn new(project_id: ProjectId) -> Self {
        Self {
            project_id,
            mr: None,
            loading: true,
            list_state: ListState::default(),
            mode: MrViewMode::Scrolling,
            comment_input: Input::default(),
        }
    }

    pub fn set_mr(&mut self, mr: MrView) {
        self.mr = Some(mr);
        self.loading = false;
    }

    pub fn set_notes(&mut self, notes: Vec<MrNote>) {
        if let Some(mr) = self.mr.as_mut() {
            mr.notes = notes;
            if !mr.notes.is_empty() {
                self.list_state.select(Some(0));
            }
        }
    }

    pub fn scroll_up(&mut self) {
        if let Some(mr) = self.mr.as_ref() {
            if mr.notes.is_empty() {
                return;
            }
            let i = self.list_state.selected().unwrap_or(0);
            let new = if i == 0 { mr.notes.len() - 1 } else { i - 1 };
            self.list_state.select(Some(new));
        }
    }

    pub fn scroll_down(&mut self) {
        if let Some(mr) = self.mr.as_ref() {
            if mr.notes.is_empty() {
                return;
            }
            let i = self.list_state.selected().unwrap_or(0);
            let new = (i + 1) % mr.notes.len();
            self.list_state.select(Some(new));
        }
    }

    pub fn selected_note(&self) -> Option<&MrNote> {
        self.mr.as_ref().and_then(|mr| {
            self.list_state
                .selected()
                .and_then(|i| mr.notes.get(i))
        })
    }

    pub fn cursor_position(&self, input_area: Rect) -> (u16, u16) {
        let x = input_area.x + 1 + self.comment_input.visual_cursor() as u16;
        let y = input_area.y + 1;
        (x.min(input_area.right().saturating_sub(1)), y)
    }
}

pub struct MrViewPopup;

impl StatefulWidget for MrViewPopup {
    type State = MrViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Merge Request ");
        let inner = block.inner(area);
        block.render(area, buf);

        if state.loading || state.mr.is_none() {
            let loading = Paragraph::new("Loading MR...").style(theme().project_name);
            loading.render(inner, buf);
            return;
        }

        let mr = state.mr.as_ref().unwrap();

        let [header_area, notes_area, input_area] = Layout::vertical([
            Constraint::Length(4),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .areas(inner);

        render_mr_header(mr, header_area, buf);
        render_notes(mr, notes_area, buf, &mut state.list_state, &state.mode);
        render_input(state, input_area, buf);
    }
}

fn render_mr_header(mr: &MrView, area: Rect, buf: &mut Buffer) {
    let state_style = match mr.state.as_str() {
        "opened" => Style::default().fg(ratatui::style::Color::Green),
        "merged" => Style::default().fg(ratatui::style::Color::Magenta),
        _ => Style::default().fg(ratatui::style::Color::Red),
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(format!("!{}  ", mr.iid), theme().pipeline_branch),
            Span::styled(mr.title.as_str(), theme().project_name),
        ]),
        Line::from(vec![
            Span::raw("State: "),
            Span::styled(mr.state.as_str(), state_style),
            Span::raw("   Author: @"),
            Span::styled(mr.author_username.as_str(), theme().pipeline_job),
        ]),
        Line::from(vec![
            Span::raw("URL: "),
            Span::styled(mr.web_url.as_str(), theme().pipeline_branch),
        ]),
        Line::from(vec![Span::styled(
            format!(
                "{} note(s) — j/k scroll  Tab compose  a atlantis  w open  Esc close",
                mr.notes.len()
            ),
            theme().date,
        )]),
    ];

    Paragraph::new(lines).render(area, buf);
}

fn render_notes(
    mr: &MrView,
    area: Rect,
    buf: &mut Buffer,
    list_state: &mut ListState,
    mode: &MrViewMode,
) {
    let block = Block::default()
        .borders(Borders::TOP)
        .title(format!(" Comments ({}) ", mr.notes.len()));
    let inner = block.inner(area);
    block.render(area, buf);

    let notes: Vec<ListItem> = mr
        .notes
        .iter()
        .enumerate()
        .map(|(idx, note)| {
            let is_atlantis_actions = matches!(
                mode,
                MrViewMode::AtlantisAction { note_idx, .. } if *note_idx == idx
            );

            let mut lines: Vec<Line> = Vec::new();

            let date_str = note
                .created_at
                .map(|dt| dt.with_timezone(&Local).format("%d %b %H:%M").to_string())
                .unwrap_or_default();

            if note.is_atlantis {
                lines.push(Line::from(vec![
                    Span::styled(
                        "[ATLANTIS] ",
                        Style::default()
                            .fg(ratatui::style::Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("@{}  {}", note.author_username, date_str),
                        theme().date,
                    ),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("@{}  ", note.author_username),
                        theme().pipeline_job,
                    ),
                    Span::styled(date_str, theme().date),
                ]));
            }

            for body_line in note.body.lines().take(8) {
                lines.push(Line::from(Span::raw(body_line.to_string())));
            }

            if note.is_atlantis {
                let (plan_style, apply_style) = if is_atlantis_actions {
                    match mode {
                        MrViewMode::AtlantisAction { selected, .. } => {
                            if *selected == 0 {
                                (
                                    Style::default()
                                        .fg(ratatui::style::Color::Black)
                                        .bg(ratatui::style::Color::Yellow),
                                    Style::default().fg(ratatui::style::Color::Yellow),
                                )
                            } else {
                                (
                                    Style::default().fg(ratatui::style::Color::Yellow),
                                    Style::default()
                                        .fg(ratatui::style::Color::Black)
                                        .bg(ratatui::style::Color::Yellow),
                                )
                            }
                        },
                        _ => (
                            Style::default().fg(ratatui::style::Color::Yellow),
                            Style::default().fg(ratatui::style::Color::Yellow),
                        ),
                    }
                } else {
                    (
                        Style::default().fg(ratatui::style::Color::Yellow),
                        Style::default().fg(ratatui::style::Color::Yellow),
                    )
                };

                lines.push(Line::from(vec![
                    Span::styled("[ atlantis plan ]", plan_style),
                    Span::raw("  "),
                    Span::styled("[ atlantis apply ]", apply_style),
                ]));
            }

            lines.push(Line::from(""));

            ListItem::new(Text::from(lines))
        })
        .collect();

    let list = List::new(notes).highlight_style(Style::default().add_modifier(Modifier::BOLD));

    StatefulWidget::render(list, inner, buf, list_state);
}

fn render_input(state: &MrViewState, area: Rect, buf: &mut Buffer) {
    let title = match state.mode {
        MrViewMode::Composing => " New Comment (Enter submit  Esc cancel) ",
        _ => " New Comment (Tab to compose) ",
    };

    let input_style = match state.mode {
        MrViewMode::Composing => Style::default().fg(ratatui::style::Color::Yellow),
        _ => Style::default().fg(ratatui::style::Color::DarkGray),
    };

    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    block.render(area, buf);

    let text = state.comment_input.value();
    Paragraph::new(text).style(input_style).render(inner, buf);
}
