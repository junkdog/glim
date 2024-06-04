use std::sync::mpsc::Sender;
use crossterm::event::{KeyCode, KeyEvent};
use crate::dispatcher::Dispatcher;
use crate::event::GlimEvent;
use crate::glim_app::StatefulWidgets;
use crate::input::InputProcessor;

pub struct PipelineActionsProcessor {
    sender: Sender<GlimEvent>,
}

impl PipelineActionsProcessor {
    pub fn new(
        sender: Sender<GlimEvent>,
    ) -> Self {
        Self { sender }
    }

    fn process(
        &self,
        event: &KeyEvent,
        ui: &mut StatefulWidgets,
    ) {
        match event.code {
            KeyCode::Esc       => self.sender.dispatch(GlimEvent::ClosePipelineActions),
            KeyCode::Up        => ui.handle_pipeline_action_selection(-1),
            KeyCode::Down      => ui.handle_pipeline_action_selection(1),
            KeyCode::Enter => {
                let state = ui.pipeline_actions.as_ref().unwrap();
                if let Some(action) = state.list_state.selected()
                    .map(|_| state.copy_action()) { self.sender.dispatch(action) }

                self.sender.dispatch(GlimEvent::ClosePipelineActions)
            }
            _ => ()
        }
    }
}

impl InputProcessor for PipelineActionsProcessor {
    fn apply(&mut self, event: &GlimEvent, ui: &mut StatefulWidgets) {
        if let GlimEvent::Key(e) = event { self.process(e, ui) }
    }

    fn on_pop(&self) {}
    fn on_push(&self) {}
}

