use std::sync::mpsc::Sender;

use compact_str::ToCompactString;
use crossterm::event::KeyCode;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

use crate::{
    dispatcher::Dispatcher,
    event::{AtlantisAction, GlimEvent},
    id::{MrIid, ProjectId},
    input::InputProcessor,
    ui::StatefulWidgets,
};

pub struct MrViewProcessor {
    sender: Sender<GlimEvent>,
    project_id: ProjectId,
    mr_iid: Option<MrIid>,
}

impl MrViewProcessor {
    pub fn new(sender: Sender<GlimEvent>, project_id: ProjectId) -> Self {
        Self { sender, project_id, mr_iid: None }
    }
}

impl InputProcessor for MrViewProcessor {
    fn apply(&mut self, event: &GlimEvent, ui: &mut StatefulWidgets) {
        match event {
            GlimEvent::MrLoaded(_, mr) => {
                self.mr_iid = Some(mr.iid);
            },
            GlimEvent::InputKey(key) => {
                if let Some(state) = ui.mr_view.as_mut() {
                    use crate::ui::popup::MrViewMode;
                    match state.mode.clone() {
                        MrViewMode::Scrolling => match key.code {
                            KeyCode::Esc | KeyCode::Char('q') => {
                                self.dispatch(GlimEvent::MrViewClose);
                            },
                            KeyCode::Char('j') | KeyCode::Down => state.scroll_down(),
                            KeyCode::Char('k') | KeyCode::Up => state.scroll_up(),
                            KeyCode::Tab | KeyCode::Char('i') => {
                                state.mode = MrViewMode::Composing;
                            },
                            KeyCode::Char('a') => {
                                if let Some(selected_idx) = state.list_state.selected() {
                                    if let Some(mr) = state.mr.as_ref() {
                                        if mr
                                            .notes
                                            .get(selected_idx)
                                            .map(|n| n.is_atlantis)
                                            .unwrap_or(false)
                                        {
                                            state.mode = MrViewMode::AtlantisAction {
                                                note_idx: selected_idx,
                                                selected: 0,
                                            };
                                        }
                                    }
                                }
                            },
                            KeyCode::Char('w') => {
                                if let Some(mr) = state.mr.as_ref() {
                                    let _ = open::that(mr.web_url.as_str());
                                }
                            },
                            _ => {},
                        },
                        MrViewMode::Composing => match key.code {
                            KeyCode::Esc => {
                                state.mode = MrViewMode::Scrolling;
                            },
                            KeyCode::Enter => {
                                if let Some(mr_iid) = self.mr_iid {
                                    let body: compact_str::CompactString =
                                        state.comment_input.value().to_compact_string();
                                    if !body.is_empty() {
                                        self.dispatch(GlimEvent::MrNotePost(
                                            self.project_id,
                                            mr_iid,
                                            body,
                                        ));
                                        state.comment_input = Input::default();
                                    }
                                }
                                state.mode = MrViewMode::Scrolling;
                            },
                            _ => {
                                state.comment_input.handle_event(
                                    &crossterm::event::Event::Key(*key),
                                );
                            },
                        },
                        MrViewMode::AtlantisAction { note_idx, selected } => {
                            match key.code {
                                KeyCode::Esc => {
                                    state.mode = MrViewMode::Scrolling;
                                },
                                KeyCode::Left | KeyCode::Char('h') => {
                                    state.mode =
                                        MrViewMode::AtlantisAction { note_idx, selected: 0 };
                                },
                                KeyCode::Right | KeyCode::Char('l') => {
                                    state.mode =
                                        MrViewMode::AtlantisAction { note_idx, selected: 1 };
                                },
                                KeyCode::Enter => {
                                    if let Some(mr_iid) = self.mr_iid {
                                        let action = if selected == 0 {
                                            AtlantisAction::Plan
                                        } else {
                                            AtlantisAction::Apply
                                        };
                                        self.dispatch(GlimEvent::MrAtlantisAction(
                                            self.project_id,
                                            mr_iid,
                                            action,
                                        ));
                                    }
                                    state.mode = MrViewMode::Scrolling;
                                },
                                _ => {},
                            }
                        },
                    }
                }
            },
            _ => {},
        }
    }

    fn on_push(&self) {}
    fn on_pop(&self) {}
}

impl Dispatcher for MrViewProcessor {
    fn dispatch(&self, event: GlimEvent) {
        self.sender.dispatch(event);
    }
}
