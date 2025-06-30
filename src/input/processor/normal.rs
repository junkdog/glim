use crate::dispatcher::Dispatcher;
use crate::event::GlimEvent;
use crate::id::ProjectId;
use crate::input::InputProcessor;
use crate::ui::StatefulWidgets;
use compact_str::ToCompactString;
use crossterm::event::{KeyCode, KeyEvent};
use std::sync::mpsc::Sender;

pub struct NormalModeProcessor {
    sender: Sender<GlimEvent>,
    selected: Option<ProjectId>,
}

impl NormalModeProcessor {
    pub fn new(sender: Sender<GlimEvent>) -> Self {
        Self {
            sender,
            selected: None,
        }
    }

    fn process(&self, event: &KeyEvent) {
        if let Some(e) = match event.code {
            KeyCode::Enter if self.selected.is_some() =>
                Some(GlimEvent::OpenProjectDetails(self.selected.unwrap())),
            KeyCode::Char('o') if self.selected.is_some() =>
                Some(GlimEvent::OpenProjectDetails(self.selected.unwrap())),
            KeyCode::Char('a') => Some(GlimEvent::ShowLastNotification),
            KeyCode::Char('c') => Some(GlimEvent::DisplayConfig),
            KeyCode::Char('f') => Some(GlimEvent::ShowFilterMenu),
            KeyCode::Char('/') => Some(GlimEvent::ShowFilterMenu),
            KeyCode::Char('l') => Some(GlimEvent::ToggleInternalLogs),
            KeyCode::Char('p') => self.selected.map(GlimEvent::RequestPipelines),
            KeyCode::Char('q') => Some(GlimEvent::Shutdown),
            KeyCode::Char('r') => Some(GlimEvent::RequestProjects),
            KeyCode::Char('s') => Some(GlimEvent::ShowSortMenu),
            KeyCode::Char('w') => self.selected.map(GlimEvent::BrowseToProject),
            KeyCode::Up        => Some(GlimEvent::SelectPreviousProject),
            KeyCode::Down      => Some(GlimEvent::SelectNextProject),
            KeyCode::Char('k') => Some(GlimEvent::SelectPreviousProject),
            KeyCode::Char('j') => Some(GlimEvent::SelectNextProject),
            KeyCode::Esc => Some(GlimEvent::ClearFilter),
            KeyCode::F(12)     => Some(GlimEvent::ToggleColorDepth),
            _ => None,
        } {
            self.dispatch(e)
        }
    }

    fn process_filter_input(&self, event: &KeyEvent, _widgets: &mut StatefulWidgets) {
        match event.code {
            KeyCode::Enter => {
                // Filter is already applied, just close the input
                self.dispatch(GlimEvent::CloseFilter);
            }
            KeyCode::Esc => {
                // Cancel filter and reset to no filter
                self.dispatch(GlimEvent::ApplyTemporaryFilter(None));
                self.dispatch(GlimEvent::CloseFilter);
            }
            KeyCode::Backspace => {
                self.dispatch(GlimEvent::FilterInputBackspace);
            }
            KeyCode::Char(c) => {
                self.dispatch(GlimEvent::FilterInputChar(c.to_compact_string()));
            }
            _ => {}
        }
    }
}

impl InputProcessor for NormalModeProcessor {
    fn apply(&mut self, event: &GlimEvent, ui: &mut StatefulWidgets) {
        match event {
            GlimEvent::SelectedProject(id) => self.selected = Some(*id),
            GlimEvent::Key(e) => {
                if ui.filter_input_active {
                    self.process_filter_input(e, ui);
                } else {
                    self.process(e);
                }
            }
            _ => (),
        }
    }

    fn on_pop(&self) {}
    fn on_push(&self) {}
}

impl Dispatcher for NormalModeProcessor {
    fn dispatch(&self, event: GlimEvent) {
        self.sender.dispatch(event)
    }
}
