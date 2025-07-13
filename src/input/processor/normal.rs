use std::sync::mpsc::Sender;

use compact_str::ToCompactString;
use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    dispatcher::Dispatcher, event::GlimEvent, id::ProjectId, input::InputProcessor,
    ui::StatefulWidgets,
};

pub struct NormalModeProcessor {
    sender: Sender<GlimEvent>,
    selected: Option<ProjectId>,
}

impl NormalModeProcessor {
    pub fn new(sender: Sender<GlimEvent>) -> Self {
        Self { sender, selected: None }
    }

    fn process(&self, event: &KeyEvent) {
        if let Some(e) = match event.code {
            KeyCode::Enter if self.selected.is_some() => {
                Some(GlimEvent::ProjectDetailsOpen(self.selected.unwrap()))
            },
            KeyCode::Char('o') if self.selected.is_some() => {
                Some(GlimEvent::ProjectDetailsOpen(self.selected.unwrap()))
            },
            KeyCode::Char('a') => Some(GlimEvent::NotificationLast),
            KeyCode::Char('c') => Some(GlimEvent::ConfigOpen),
            KeyCode::Char('f') => Some(GlimEvent::FilterMenuShow),
            KeyCode::Char('/') => Some(GlimEvent::FilterMenuShow),
            KeyCode::Char('p') => self.selected.map(GlimEvent::PipelinesFetch),
            KeyCode::Char('q') => Some(GlimEvent::AppExit),
            KeyCode::Char('r') => Some(GlimEvent::ProjectsFetch),
            KeyCode::Char('w') => self.selected.map(GlimEvent::ProjectOpenUrl),
            KeyCode::F(12) => Some(GlimEvent::ScreenCapture),
            KeyCode::Up => Some(GlimEvent::ProjectPrevious),
            KeyCode::Down => Some(GlimEvent::ProjectNext),
            KeyCode::Char('k') => Some(GlimEvent::ProjectPrevious),
            KeyCode::Char('j') => Some(GlimEvent::ProjectNext),
            KeyCode::Esc => Some(GlimEvent::FilterClear),
            _ => None,
        } {
            self.dispatch(e)
        }
    }

    fn process_filter_input(&self, event: &KeyEvent, _widgets: &mut StatefulWidgets) {
        match event.code {
            KeyCode::Enter => {
                // Filter is already applied, just close the input
                self.dispatch(GlimEvent::FilterMenuClose);
            },
            KeyCode::Esc => {
                // Cancel filter and reset to no filter
                self.dispatch(GlimEvent::ApplyTemporaryFilter(None));
                self.dispatch(GlimEvent::FilterMenuClose);
            },
            KeyCode::Backspace => {
                self.dispatch(GlimEvent::FilterInputBackspace);
            },
            KeyCode::Char(c) => {
                self.dispatch(GlimEvent::FilterInputChar(c.to_compact_string()));
            },
            _ => {},
        }
    }
}

impl InputProcessor for NormalModeProcessor {
    fn apply(&mut self, event: &GlimEvent, ui: &mut StatefulWidgets) {
        match event {
            GlimEvent::ProjectSelected(id) => self.selected = Some(*id),
            GlimEvent::InputKey(e) => {
                if ui.filter_input_active {
                    self.process_filter_input(e, ui);
                } else {
                    self.process(e);
                }
            },
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
