use std::{path::PathBuf, process::exit};

use clap::Parser;

use crate::{
    app_init::{initialize_app, AppComponents},
    config::default_config_path,
    glim_app::GlimConfig,
    rendering::render_main_ui,
    result::Result,
};

mod app_init;
mod client;
mod config;
mod dispatcher;
mod domain;
mod effect_registry;
mod event;
mod glim_app;
mod gruvbox;
mod id;
mod input;
mod logging;
mod notice_service;
mod rendering;
mod result;
mod stores;
mod theme;
mod tui;
mod ui;

/// A TUI for monitoring GitLab CI/CD pipelines and projects
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Alternate path to the configuration file.
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
    /// Print the path to the configuration file and exit.
    #[arg(short, long)]
    print_config_path: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let config_path = args.config.unwrap_or_else(default_config_path);

    if args.print_config_path {
        println!("{}", config_path.display());
        exit(0);
    }

    let debug = std::env::var("GLIM_DEBUG").is_ok();

    let mut config = if config_path.exists() {
        confy::load_path(&config_path)
            .map_err(|e| crate::result::GlimError::config_load_error(config_path.clone(), e))?
    } else {
        GlimConfig::default()
    };

    // Allow env vars to override config file values
    if let Ok(token) = std::env::var("GITLAB_TOKEN") {
        config.gitlab_token = token.into();
    }
    if let Ok(url) = std::env::var("GITLAB_URL") {
        config.gitlab_url = url.into();
    }

    // Validate that credentials are available from some source
    if config.gitlab_token.is_empty() {
        eprintln!("Error: GitLab token is not configured.\n");
        eprintln!("Set it via environment variable:  export GITLAB_TOKEN=<your-token>");
        eprintln!("Or add to config file ({}):", config_path.display());
        eprintln!("  gitlab_token = \"<your-token>\"");
        exit(1);
    }
    if config.gitlab_url.is_empty() {
        eprintln!("Error: GitLab URL is not configured.\n");
        eprintln!("Set it via environment variable:  export GITLAB_URL=https://gitlab.com");
        eprintln!("Or add to config file ({}):", config_path.display());
        eprintln!("  gitlab_url = \"https://gitlab.com\"");
        exit(1);
    }

    // Create a shared runtime for async operations
    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        crate::result::GlimError::GeneralError(format!("Failed to create runtime: {e}").into())
    })?;

    let AppComponents {
        mut app,
        mut tui,
        mut widget_states,
        mut effects,
        poller,
        _log_guard,
    } = rt.block_on(async { initialize_app(config_path, config, debug).await })?;

    // Start the poller in the background
    rt.spawn(async move {
        if let Err(e) = poller.start().await {
            tracing::error!("GitLab poller failed: {}", e);
        }
    });

    while app.is_running() {
        widget_states.last_frame = app.process_timers();
        tui.receive_events(|event| {
            widget_states.apply(&app, &mut effects, &event);
            app.apply(event, &mut widget_states, &mut effects);
        });
        tui.draw(|f| render_main_ui(f, &app, &mut widget_states, &mut effects))?;
    }

    tui.exit()?;
    Ok(())
}
