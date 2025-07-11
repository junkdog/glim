use std::sync::mpsc::Sender;

use compact_str::ToCompactString;
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::{
    client::{ClientConfig, GitlabPoller, GitlabService},
    dispatcher::Dispatcher,
    effect_registry::EffectRegistry,
    event::{EventHandler, GlimEvent},
    glim_app::{GlimApp, GlimConfig},
    logging::{init_logging, LoggingConfig},
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
}

pub async fn initialize_app(
    config_path: std::path::PathBuf,
    config: GlimConfig,
    debug: bool,
) -> Result<AppComponents> {
    color_eyre::install().expect("failed to install color_eyre");

    let event_handler = EventHandler::new(std::time::Duration::from_millis(33));
    let sender = event_handler.sender();

    initialize_logging(sender.clone())?;
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "Glim TUI starting up");

    let tui = initialize_terminal(event_handler)?;
    let widget_states = StatefulWidgets::new(sender.clone());

    let (service, poller) = create_gitlab_service_and_poller(sender.clone(), config, debug).await?;
    let app = GlimApp::new(sender.clone(), config_path, service);
    app.dispatch(GlimEvent::RequestProjects);

    let mut effects = EffectRegistry::new(app.sender());
    effects.register_default_glitch_effect();

    Ok(AppComponents { app, tui, widget_states, effects, poller })
}

fn initialize_logging(sender: Sender<GlimEvent>) -> Result<()> {
    let logging_config = LoggingConfig::from_env();
    let _log_guard = init_logging(logging_config, Some(sender)).map_err(|e| {
        GlimError::GeneralError(format!("Failed to initialize logging: {e}").into())
    })?;
    Ok(())
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
    let client_config = ClientConfig::from(config)
        .with_debug_logging(debug);
    
    let service = GitlabService::new(client_config.clone(), sender.clone())?;
    
    // Create a second service instance for the poller
    let poller_service = GitlabService::new(client_config.clone(), sender)?;
    let poller = GitlabPoller::new(
        std::sync::Arc::new(poller_service), 
        client_config.polling.clone()
    );
    
    Ok((service, poller))
}
