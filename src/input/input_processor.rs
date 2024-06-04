use crate::event::GlimEvent;
use crate::glim_app::StatefulWidgets;

pub trait InputProcessor {
    fn apply(
        &mut self,
        event: &GlimEvent,
        ui: &mut StatefulWidgets,
    );

    fn on_pop(&self);
    fn on_push(&self);
}
