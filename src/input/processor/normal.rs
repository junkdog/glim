use std::sync::mpsc::Sender;
use crossterm::event::{KeyCode, KeyEvent};
use crate::dispatcher::Dispatcher;
use crate::event::GlimEvent;
use crate::id::ProjectId;
use crate::input::InputProcessor;
use crate::ui::StatefulWidgets;

pub struct NormalModeProcessor {
    sender: Sender<GlimEvent>,
    selected: Option<ProjectId>
}

impl NormalModeProcessor {
    pub fn new(sender: Sender<GlimEvent>) -> Self {
        Self {
            sender,
            selected: None,
        }
    }

    fn process(
        &self,
        event: &KeyEvent,
    ) {
        if let Some(e) = match event.code {
            KeyCode::Enter if self.selected.is_some() =>
                Some(GlimEvent::OpenProjectDetails(self.selected.unwrap())),
            KeyCode::Char('o') if self.selected.is_some() =>
                Some(GlimEvent::OpenProjectDetails(self.selected.unwrap())),
            KeyCode::Char('a') => Some(GlimEvent::ShowLastNotification),
            KeyCode::Char('c') => Some(GlimEvent::DisplayConfig),
            KeyCode::Char('l') => Some(GlimEvent::ToggleInternalLogs),
            KeyCode::Char('p') => self.selected.map(GlimEvent::RequestPipelines),
            KeyCode::Char('q') => Some(GlimEvent::Shutdown),
            KeyCode::Char('r') => Some(GlimEvent::RequestProjects),
            KeyCode::Char('w') => self.selected.map(GlimEvent::BrowseToProject),
            KeyCode::Up        => Some(GlimEvent::SelectPreviousProject),
            KeyCode::Down      => Some(GlimEvent::SelectNextProject),
            KeyCode::Char('k')        => Some(GlimEvent::SelectPreviousProject),
            KeyCode::Char('j')      => Some(GlimEvent::SelectNextProject),
            KeyCode::F(12)     => Some(GlimEvent::ToggleColorDepth),
            _ => None
        } { self.dispatch(e) }
    }
}

impl InputProcessor for NormalModeProcessor {

    fn apply(&mut self, event: &GlimEvent, _ui: &mut StatefulWidgets) {
        match event {
            GlimEvent::SelectedProject(id)   => self.selected = Some(*id),
            GlimEvent::Key(e)                => self.process(e),
            _                                => ()
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
