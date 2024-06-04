use std::sync::mpsc::Sender;
use crate::dispatcher::Dispatcher;
use crate::event::GlimEvent;
use crate::glim_app::StatefulWidgets;
use crate::input::InputProcessor;

pub struct AlertProcessor {
    sender: Sender<GlimEvent>
}

impl AlertProcessor {
    pub fn new(sender: Sender<GlimEvent>) -> Self {
        Self { sender }
    }
}

impl InputProcessor for AlertProcessor {
    fn apply(&mut self, event: &GlimEvent, _: &mut StatefulWidgets) {
        if let GlimEvent::Key(_e) = event {
            self.sender.dispatch(GlimEvent::CloseAlert)
        }
    }

    fn on_pop(&self) {}
    fn on_push(&self) {}
}


