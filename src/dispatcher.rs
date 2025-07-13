use std::sync::mpsc;

use crate::event::GlimEvent;

pub trait Dispatcher {
    fn dispatch(&self, event: GlimEvent);
}

impl Dispatcher for mpsc::Sender<GlimEvent> {
    fn dispatch(&self, event: GlimEvent) {
        let _ = self.send(event);
    }
}
