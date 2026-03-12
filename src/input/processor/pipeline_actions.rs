use std::sync::mpsc::Sender;

use crossterm::event::{KeyCode, KeyEvent};

use crate::{dispatcher::Dispatcher, event::GlimEvent, input::InputProcessor, ui::StatefulWidgets};

pub struct PipelineActionsProcessor {
    sender: Sender<GlimEvent>,
}

impl PipelineActionsProcessor {
    pub fn new(sender: Sender<GlimEvent>) -> Self {
        Self { sender }
    }

    fn process(&self, event: &KeyEvent, ui: &mut StatefulWidgets) {
        match event.code {
            KeyCode::Esc => self
                .sender
                .dispatch(GlimEvent::PipelineActionsClose),
            KeyCode::Char('q') => self
                .sender
                .dispatch(GlimEvent::PipelineActionsClose),
            KeyCode::Up => ui.handle_pipeline_action_selection(-1),
            KeyCode::Down => ui.handle_pipeline_action_selection(1),
            KeyCode::Char('k') => ui.handle_pipeline_action_selection(-1),
            KeyCode::Char('j') => ui.handle_pipeline_action_selection(1),
            KeyCode::Enter => {
                let state = ui.pipeline_actions.as_ref().unwrap();
                let action = state
                    .list_state
                    .selected()
                    .map(|idx| state.copy_selected_action(idx));

                // Close BEFORE dispatching the action so PipelineActionsClose
                // pops PipelineActionsProcessor, not any processor the action pushes.
                self.sender.dispatch(GlimEvent::PipelineActionsClose);
                if let Some(action) = action {
                    self.sender.dispatch(action)
                }
            },
            KeyCode::Char('o') => {
                let state = ui.pipeline_actions.as_ref().unwrap();
                let action = state
                    .list_state
                    .selected()
                    .map(|idx| state.copy_selected_action(idx));

                self.sender.dispatch(GlimEvent::PipelineActionsClose);
                if let Some(action) = action {
                    self.sender.dispatch(action)
                }
            },
            KeyCode::F(12) => self.sender.dispatch(GlimEvent::ScreenCapture),
            _ => (),
        }
    }
}

impl InputProcessor for PipelineActionsProcessor {
    fn apply(&mut self, event: &GlimEvent, ui: &mut StatefulWidgets) {
        if let GlimEvent::InputKey(e) = event {
            self.process(e, ui)
        }
    }

    fn on_pop(&self) {}
    fn on_push(&self) {}
}
