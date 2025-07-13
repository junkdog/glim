use std::sync::mpsc::Sender;

use crossterm::event::{Event as CrosstermEvent, KeyCode};
use tui_input::backend::crossterm::EventHandler;

use crate::{dispatcher::Dispatcher, event::GlimEvent, input::InputProcessor, ui::StatefulWidgets};

pub struct ConfigProcessor {
    sender: Sender<GlimEvent>,
}

impl ConfigProcessor {
    pub fn new(sender: Sender<GlimEvent>) -> Self {
        Self { sender }
    }
}

impl InputProcessor for ConfigProcessor {
    fn apply(&mut self, event: &GlimEvent, widgets: &mut StatefulWidgets) {
        if let GlimEvent::InputKey(code) = event {
            let popup = widgets.config_popup_state.as_mut().unwrap();
            match code.code {
                KeyCode::Enter => self.sender.dispatch(GlimEvent::ConfigApply),
                KeyCode::Esc => self.sender.dispatch(GlimEvent::ConfigClose),
                KeyCode::Down => popup.select_next_input(),
                KeyCode::Up => popup.select_previous_input(),
                KeyCode::Char('j') => popup.select_next_input(),
                KeyCode::Char('k') => popup.select_previous_input(),
                KeyCode::Tab => popup.select_next_input(),
                KeyCode::BackTab => popup.select_previous_input(),
                KeyCode::Left => {
                    if popup.is_current_field_dropdown() {
                        popup.cycle_dropdown_prev();
                    } else {
                        popup
                            .input_mut()
                            .handle_event(&CrosstermEvent::Key(*code));
                    }
                },
                KeyCode::Right => {
                    if popup.is_current_field_dropdown() {
                        popup.cycle_dropdown_next();
                    } else {
                        popup
                            .input_mut()
                            .handle_event(&CrosstermEvent::Key(*code));
                    }
                },
                KeyCode::F(12) => self.sender.dispatch(GlimEvent::ScreenCapture),
                _ => {
                    if !popup.is_current_field_dropdown() {
                        popup
                            .input_mut()
                            .handle_event(&CrosstermEvent::Key(*code));
                    }
                },
            }
        }
    }

    fn on_pop(&self) {}
    fn on_push(&self) {}
}
