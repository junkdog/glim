use std::sync::mpsc::Sender;

use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    dispatcher::Dispatcher,
    event::GlimEvent,
    id::{PipelineId, ProjectId},
    input::InputProcessor,
    ui::StatefulWidgets,
};

pub struct ProjectDetailsProcessor {
    sender: Sender<GlimEvent>,
    project_id: ProjectId,
    selected: Option<PipelineId>,
}

impl ProjectDetailsProcessor {
    pub fn new(sender: Sender<GlimEvent>, project_id: ProjectId) -> Self {
        Self { sender, project_id, selected: None }
    }

    fn process(&self, event: &KeyEvent, ui: &mut StatefulWidgets) {
        match event.code {
            KeyCode::Esc => self
                .sender
                .dispatch(GlimEvent::CloseProjectDetails),
            KeyCode::Char('q') => self
                .sender
                .dispatch(GlimEvent::CloseProjectDetails),
            KeyCode::Up => ui.handle_pipeline_selection(-1),
            KeyCode::Down => ui.handle_pipeline_selection(1),
            KeyCode::Char('k') => ui.handle_pipeline_selection(-1),
            KeyCode::Char('j') => ui.handle_pipeline_selection(1),
            KeyCode::Enter if self.selected.is_some() => self
                .sender
                .dispatch(GlimEvent::OpenPipelineActions(self.project_id, self.selected.unwrap())),
            KeyCode::Char('o') if self.selected.is_some() => self
                .sender
                .dispatch(GlimEvent::OpenPipelineActions(self.project_id, self.selected.unwrap())),
            _ => (),
        }
    }
}

impl InputProcessor for ProjectDetailsProcessor {
    fn apply(&mut self, event: &GlimEvent, ui: &mut StatefulWidgets) {
        match event {
            GlimEvent::SelectedPipeline(pipeline) => self.selected = Some(*pipeline),
            GlimEvent::Key(e) => self.process(e, ui),
            _ => (),
        }
    }

    fn on_pop(&self) {}
    fn on_push(&self) {}
}
