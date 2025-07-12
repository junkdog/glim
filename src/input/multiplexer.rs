use std::sync::mpsc::Sender;

use crate::{
    event::GlimEvent,
    input::{
        processor::{ConfigProcessor, PipelineActionsProcessor, ProjectDetailsProcessor},
        InputProcessor,
    },
    ui::StatefulWidgets,
};

pub struct InputMultiplexer {
    sender: Sender<GlimEvent>,
    processors: Vec<Box<dyn InputProcessor>>,
}

impl InputMultiplexer {
    pub fn new(sender: Sender<GlimEvent>) -> Self {
        Self { sender, processors: Vec::new() }
    }

    pub fn push(&mut self, processor: Box<dyn InputProcessor>) {
        self.processors.push(processor);
        if let Some(processor) = self.processors.last() {
            processor.on_push()
        }
    }

    pub fn pop_processor(&mut self) {
        if let Some(processor) = self.processors.last() {
            processor.on_pop();
        }
        self.processors.pop();
    }

    pub fn apply(&mut self, event: &GlimEvent, ui: &mut StatefulWidgets) {
        match event {
            // project details popup
            GlimEvent::OpenProjectDetails(id) => {
                self.push(Box::new(ProjectDetailsProcessor::new(self.sender.clone(), *id)));
            },
            GlimEvent::CloseProjectDetails => self.pop_processor(),

            // pipeline actions popup
            GlimEvent::OpenPipelineActions(_, _) => {
                self.push(Box::new(PipelineActionsProcessor::new(self.sender.clone())));
            },
            GlimEvent::ClosePipelineActions => self.pop_processor(),

            // config
            GlimEvent::DisplayConfig => {
                self.push(Box::new(ConfigProcessor::new(self.sender.clone())));
            },
            GlimEvent::CloseConfig => self.pop_processor(),

            _ => (),
        }

        if let Some(processor) = self.processors.last_mut() {
            processor.apply(event, ui)
        }
    }
}
