use std::sync::mpsc::Sender;

use compact_str::ToCompactString;
use ratatui::{backend::CrosstermBackend, Terminal};
use tracing_appender::non_blocking::WorkerGuard;

use crate::{
    client::{ClientConfig, GitlabApi, GitlabPoller, GitlabService},
    dispatcher::Dispatcher,
    effect_registry::EffectRegistry,
    event::{EventHandler, GlimEvent},
    glim_app::{GlimApp, GlimConfig},
    logging::{init_logging, LoggingConfig, LoggingReloadHandle},
    result::{GlimError, Result},
    tui::Tui,
    ui::StatefulWidgets,
};

pub struct AppComponents {
    pub app: GlimApp,
    pub tui: Tui,
    pub widget_states: StatefulWidgets,
    pub effects: EffectRegistry,
    pub poller: GitlabPoller,
    pub _log_guard: Option<WorkerGuard>,
}

pub async fn initialize_app(
    config_path: std::path::PathBuf,
    config: GlimConfig,
    debug: bool,
) -> Result<AppComponents> {
    color_eyre::install().expect("failed to install color_eyre");

    let event_handler = EventHandler::new(std::time::Duration::from_millis(33));
    let sender = event_handler.sender();

    let (log_guard, log_reload_handle) = initialize_logging(sender.clone(), &config)?;
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "Glim TUI starting up");

    let tui = initialize_terminal(event_handler)?;
    let widget_states = StatefulWidgets::new(sender.clone());

    let (service, poller) =
        create_gitlab_service_and_poller(sender.clone(), config.clone(), debug).await?;
    
    // We need to move the log_reload_handle into the app, so we can't use it in AppComponents
    // Instead, we'll create a separate handle for the app and keep one for external use
    let app = GlimApp::new(sender.clone(), config_path, service, log_reload_handle);
    app.dispatch(GlimEvent::ProjectsFetch);
    if config == GlimConfig::default() {
        app.dispatch(GlimEvent::ConfigOpen);
    }

    let mut effects = EffectRegistry::new(app.sender());
    effects.register_default_glitch_effect();

    Ok(AppComponents {
        app,
        tui,
        widget_states,
        effects,
        poller,
        _log_guard: log_guard,
    })
}

fn initialize_logging(
    sender: Sender<GlimEvent>,
    glim_config: &GlimConfig,
) -> Result<(Option<WorkerGuard>, LoggingReloadHandle)> {
    let mut logging_config = LoggingConfig::from_env();

    // Override with config if specified
    if let Some(log_level) = &glim_config.log_level {
        if let Ok(level) = log_level.parse() {
            logging_config.console_level = level;
            logging_config.file_level = level;
        }

        // Disable logging if set to "Off"
        if log_level == "Off" {
            logging_config.log_dir = None;
        }
    }

    let (log_guard, log_reload_handle) = init_logging(logging_config, Some(sender)).map_err(|e| {
        GlimError::GeneralError(format!("Failed to initialize logging: {e}").into())
    })?;
    Ok((log_guard, log_reload_handle))
}

fn initialize_terminal(event_handler: EventHandler) -> Result<Tui> {
    let backend = CrosstermBackend::new(std::io::stdout());
    let terminal = Terminal::new(backend).map_err(|_| {
        GlimError::GeneralError("failed to initialize terminal".to_compact_string())
    })?;
    let mut tui = Tui::new(terminal, event_handler);
    tui.enter()?;
    Ok(tui)
}

async fn create_gitlab_service_and_poller(
    sender: Sender<GlimEvent>,
    config: GlimConfig,
    debug: bool,
) -> Result<(GitlabService, GitlabPoller)> {
    let client_config = ClientConfig::from(config).with_debug_logging(debug);

    // Create a shared GitlabApi instance
    let api = std::sync::Arc::new(GitlabApi::force_new(client_config.clone())?);

    // Create service using shared API
    let service = GitlabService::from_api(api.clone(), sender.clone())?;

    // Create poller using shared API
    let poller = GitlabPoller::new(api, sender, client_config.polling.clone());

    Ok((service, poller))
}
