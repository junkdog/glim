use crate::dispatcher::Dispatcher;
use crate::event::GlimEvent;
use crate::input::InputProcessor;
use crate::ui::StatefulWidgets;
use crossterm::event::{KeyCode, KeyEvent};
use std::sync::mpsc::Sender;

pub struct PipelineActionsProcessor {
    sender: Sender<GlimEvent>,
}

impl PipelineActionsProcessor {
    pub fn new(sender: Sender<GlimEvent>) -> Self {
        Self { sender }
    }

    fn process(&self, event: &KeyEvent, ui: &mut StatefulWidgets) {
        match event.code {
            KeyCode::Esc       => self.sender.dispatch(GlimEvent::ClosePipelineActions),
            KeyCode::Char('q') => self.sender.dispatch(GlimEvent::ClosePipelineActions),
            KeyCode::Up        => ui.handle_pipeline_action_selection(-1),
            KeyCode::Down      => ui.handle_pipeline_action_selection(1),
            KeyCode::Char('k') => ui.handle_pipeline_action_selection(-1),
            KeyCode::Char('j') => ui.handle_pipeline_action_selection(1),
            KeyCode::Enter => {
                let state = ui.pipeline_actions.as_ref().unwrap();
                if let Some(action) = state
                    .list_state
                    .selected()
                    .map(|idx| state.copy_selected_action(idx))
                {
                    self.sender.dispatch(action)
                }

                self.sender.dispatch(GlimEvent::ClosePipelineActions)
            }
            KeyCode::Char('o') => {
                let state = ui.pipeline_actions.as_ref().unwrap();
                if let Some(action) = state.list_state.selected()
                    .map(|idx| state.copy_selected_action(idx)) { self.sender.dispatch(action) }

                self.sender.dispatch(GlimEvent::ClosePipelineActions)
            }
            _ => ()
        }
    }

}

impl InputProcessor for PipelineActionsProcessor {
    fn apply(&mut self, event: &GlimEvent, ui: &mut StatefulWidgets) {
        if let GlimEvent::Key(e) = event {
            self.process(e, ui)
        }
    }

    fn on_pop(&self) {}
    fn on_push(&self) {}
}
