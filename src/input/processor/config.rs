use std::sync::mpsc::Sender;
use crossterm::event::KeyCode;
use tui_input::backend::crossterm::EventHandler;
use crossterm::{event::Event as CrosstermEvent};
use crate::dispatcher::Dispatcher;
use crate::event::GlimEvent;
use crate::glim_app::StatefulWidgets;
use crate::input::InputProcessor;

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
        if let GlimEvent::Key(code) = event {
            let popup = widgets.config_popup_state.as_mut().unwrap();
            match code.code {
                KeyCode::Enter => self.sender.dispatch(GlimEvent::ApplyConfiguration),
                KeyCode::Esc   => self.sender.dispatch(GlimEvent::CloseConfig),
                KeyCode::Down  => popup.select_next_input(),
                KeyCode::Up    => popup.select_previous_input(),
                _ => {
                    popup.input_mut().handle_event(&CrosstermEvent::Key(*code));
                },
            }
        }
    }

    fn on_pop(&self) {}
    fn on_push(&self) {}
}

