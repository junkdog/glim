use std::{io, panic};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::Frame;

use crate::{
    event::{EventHandler, GlimEvent},
    result::{GlimError, GlimError::GeneralError},
};

pub type CrosstermTerminal = ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>;

/// Representation of a terminal user interface.
///
/// It is responsible for setting up the terminal,
/// initializing the interface and handling the draw events.
pub struct Tui {
    /// Interface to the Terminal.
    terminal: CrosstermTerminal,
    /// Terminal event handler.
    events: EventHandler,
}

impl Tui {
    pub fn new(terminal: CrosstermTerminal, events: EventHandler) -> Self {
        Self { terminal, events }
    }

    pub fn draw(&mut self, render_ui: impl FnOnce(&mut Frame)) -> Result<(), GlimError> {
        self.terminal
            .draw(render_ui)
            .map_err(|_| GeneralError("failed to draw UI".into()))?;
        Ok(())
    }

    /// iterates over all currently available events; waits
    /// until at least one event is available.
    pub fn receive_events<F>(&self, mut f: F)
    where
        F: FnMut(GlimEvent),
    {
        let mut apply_event = |e| match e {
            GlimEvent::ProjectsLoaded(p) if p.is_empty() => (),
            GlimEvent::PipelinesLoaded(p) if p.is_empty() => (),
            GlimEvent::JobsLoaded(_, _, j) if j.is_empty() => (),
            _ => f(e),
        };

        apply_event(self.events.next().unwrap());
        while let Some(event) = self.events.try_next() {
            apply_event(event)
        }
    }

    pub fn enter(&mut self) -> Result<(), GlimError> {
        terminal::enable_raw_mode()
            .map_err(|_| GeneralError("failed to initialize raw mode".into()))?;

        crossterm::execute!(io::stderr(), EnterAlternateScreen, EnableMouseCapture)
            .map_err(|_| GeneralError("failed to enter alternate screen".into()))?;

        // Define a custom panic hook to reset the terminal properties.
        // This way, you won't have your terminal messed up if an unexpected error happens.
        let panic_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic| {
            Self::reset().expect("failed to reset the terminal");
            panic_hook(panic);
        }));

        self.terminal
            .hide_cursor()
            .map_err(|_| GeneralError("failed to hide cursor".into()))?;
        self.terminal
            .clear()
            .map_err(|_| GeneralError("failed to clear the screen".into()))?;
        Ok(())
    }

    fn reset() -> Result<(), GlimError> {
        terminal::disable_raw_mode()
            .map_err(|_| GeneralError("failed to disable raw mode".into()))?;
        crossterm::execute!(io::stderr(), LeaveAlternateScreen, DisableMouseCapture)
            .map_err(|_| GeneralError("failed to leave alternate screen".into()))?;

        Ok(())
    }

    pub fn exit(&mut self) -> Result<(), GlimError> {
        Self::reset()?;
        self.terminal
            .show_cursor()
            .map_err(|_| GeneralError("failed to show cursor".into()))?;
        Ok(())
    }
}
