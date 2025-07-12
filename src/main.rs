use std::{path::PathBuf, process::exit, sync::mpsc::Sender};

use clap::Parser;
use compact_str::ToCompactString;
use directories::BaseDirs;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Margin, Rect},
    prelude::Direction,
    Frame, Terminal,
};
use tachyonfx::{Duration, EffectRenderer, Shader};

use crate::{
    client::GitlabClient,
    dispatcher::Dispatcher,
    effect_registry::EffectRegistry,
    event::{EventHandler, GlimEvent},
    glim_app::{GlimApp, GlimConfig},
    input::{processor::ConfigProcessor, InputProcessor},
    logging::{init_logging, LoggingConfig},
    result::{GlimError, Result},
    theme::theme,
    tui::Tui,
    ui::{
        popup::{ConfigPopup, ConfigPopupState, PipelineActionsPopup, ProjectDetailsPopup},
        widget::{Notification, ProjectsTable, RefRect},
        StatefulWidgets,
    },
};

mod client;
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
    color_eyre::install().expect("failed to install color_eyre");

    let args = Args::parse();
    let config_path = args.config.unwrap_or_else(default_config_path);
    if args.print_config_path {
        println!("{}", config_path.display());
        exit(0);
    }
    let debug = std::env::var("GLIM_DEBUG").is_ok();

    // event handler
    let event_handler = EventHandler::new(std::time::Duration::from_millis(33));
    let sender = event_handler.sender();

    // Initialize logging system
    let logging_config = LoggingConfig::from_env();
    let _log_guard = init_logging(logging_config, Some(sender.clone())).map_err(|e| {
        GlimError::GeneralError(format!("Failed to initialize logging: {e}").into())
    })?;

    tracing::info!(version = env!("CARGO_PKG_VERSION"), "Glim TUI starting up");

    // tui backend
    let backend = CrosstermBackend::new(std::io::stdout());
    let terminal = Terminal::new(backend).map_err(|_| {
        GlimError::GeneralError("failed to initialize terminal".to_compact_string())
    })?;
    let mut tui = Tui::new(terminal, event_handler);
    tui.enter()?;

    let mut widget_states = StatefulWidgets::new(sender.clone());
    let config = run_config_ui_loop(
        &mut tui,
        &mut widget_states,
        sender.clone(),
        config_path.clone(),
        debug,
    )?;

    // app state and initial setup

    let mut app =
        GlimApp::new(sender.clone(), config_path, gitlab_client(sender.clone(), config, debug));
    app.dispatch(GlimEvent::RequestProjects);

    let mut effects = EffectRegistry::new(app.sender());
    effects.register_default_glitch_effect();

    // main loop
    while app.is_running() {
        widget_states.last_frame = app.process_timers();
        tui.receive_events(|event| {
            widget_states.apply(&app, &mut effects, &event);
            app.apply(event, &mut widget_states, &mut effects);
        });
        tui.draw(|f| render_widgets(f, &app, &mut widget_states, &mut effects))?;
    }

    tui.exit()
        .map_err(|_| GlimError::GeneralError("failed to exit TUI".to_compact_string()))?;
    Ok(())
}

fn render_widgets(
    f: &mut Frame,
    app: &GlimApp,
    widget_states: &mut StatefulWidgets,
    effects: &mut EffectRegistry,
) {
    effects.update_screen_area(f.area());

    let last_tick = widget_states.last_frame;
    let frame_area = f.area();
    let layout =
        Layout::new(Direction::Horizontal, [Constraint::Percentage(100)]).split(frame_area);

    // gitlab pipelines
    let config = app.load_config().unwrap_or_default();
    let effective_filter = widget_states.effective_filter(&config.search_filter);
    let (filtered_projects, filtered_indices) = app.filtered_projects(&effective_filter);
    widget_states.update_filtered_indices(filtered_indices);
    let projects = ProjectsTable::new(
        &filtered_projects,
        widget_states.filter_input_active,
        &widget_states.filter_input_text,
    );
    f.render_stateful_widget(projects, layout[0], &mut widget_states.project_table_state);

    // project details popup
    if let Some(project_details) = widget_states.project_details.as_mut() {
        let popup = ProjectDetailsPopup::new();
        let popup_area = layout[0].inner(Margin::new(6, 2));

        // f.render_effect(popup_area, &mut project_details.fade_in, last_frame_ms);
        f.render_stateful_widget(popup, popup_area, project_details);
    }

    // pipeline actions popup
    if let Some(pipeline_actions) = widget_states.pipeline_actions.as_mut() {
        let popup = PipelineActionsPopup::new();

        // render popup on top
        f.render_stateful_widget(popup, layout[0], pipeline_actions);
    }

    let last_tick = last_tick;

    // fade in table
    if let Some(shader) = &mut widget_states.table_fade_in {
        f.render_effect(shader, layout[0], last_tick);
        widget_states.table_fade_in.take_if(|s| s.done());
    }

    if let Some(config_popup) = &mut widget_states.config_popup_state {
        // f.render_effect(&mut config_popup.parent_fade, last_frame_ms);
        render_config_popup(f, config_popup, last_tick, layout[0]);
    }

    // notification
    if let Some(notification) = &mut widget_states.notice {
        f.render_stateful_widget(Notification::new(), layout[0], notification);
    }

    effects.process_effects(last_tick, f.buffer_mut(), frame_area);
}

fn render_config_popup(
    f: &mut Frame,
    config_popup: &mut ConfigPopupState,
    last_tick: Duration,
    layout: Rect,
) {
    // render widget
    let popup = ConfigPopup::new();
    f.render_stateful_widget(popup, layout, config_popup);

    // render cursor once UI has ~faded in
    // todo: refactor and fix
    if true {
        // config_popup.is_open_complete() {
        let cursor = config_popup.cursor_position;
        f.buffer_mut()
            .set_style(Rect::new(cursor.x, cursor.y, 1, 1), theme().input_selected);
        f.set_cursor_position(cursor);
    }
}

fn gitlab_client(sender: Sender<GlimEvent>, config: GlimConfig, debug: bool) -> GitlabClient {
    GitlabClient::new(sender, config.gitlab_url, config.gitlab_token, config.search_filter, debug)
}

fn default_config_path() -> PathBuf {
    if let Some(dirs) = BaseDirs::new() {
        dirs.config_dir().join("glim.toml")
    } else {
        PathBuf::from("glim.toml")
    }
}

pub fn save_config(config_file: &PathBuf, config: GlimConfig) -> Result<()> {
    confy::store_path(config_file, &config)
        .map_err(|e| GlimError::ConfigError(e.to_compact_string()))?;

    Ok(())
}

/// Run the configuration UI loop to create the configuration file.
/// If the configuration file already exists, it is loaded and returned.
pub fn run_config_ui_loop(
    tui: &mut Tui,
    ui: &mut StatefulWidgets,
    sender: Sender<GlimEvent>,
    config_file: PathBuf,
    debug: bool,
) -> Result<GlimConfig> {
    if config_file.exists() {
        let config: GlimConfig = confy::load_path(config_file)
            .map_err(|e| GlimError::ConfigError(e.to_compact_string()))?;

        Ok(config)
    } else {
        ui.config_popup_state =
            Some(ConfigPopupState::new(GlimConfig::default(), RefRect::default()));
        let sender = sender.clone();

        let mut last_tick = std::time::Instant::now();
        let mut valid_config: Option<GlimConfig> = None;
        while valid_config.is_none() && ui.config_popup_state.is_some() {
            let now = std::time::Instant::now();
            ui.last_frame = Duration::from_millis((now - last_tick).as_millis() as u32 / 2);
            last_tick = now;

            let mut input_processor = ConfigProcessor::new(sender.clone());

            tui.receive_events(|event| {
                input_processor.apply(&event, ui);
                match event {
                    // GlimEvent::CloseAlert => {}
                    GlimEvent::ApplyConfiguration => {
                        let config = ui
                            .config_popup_state
                            .as_ref()
                            .unwrap()
                            .to_config();
                        match config.validate() {
                            Ok(_) => {
                                let client =
                                    GitlabClient::new_from_config(sender.clone(), config, debug);
                                match client.validate_configuration() {
                                    Ok(_) => {
                                        let state = ui.config_popup_state.as_ref().unwrap();
                                        save_config(&config_file, state.to_config())
                                            .expect("failed to save configuration");

                                        valid_config = Some(state.to_config());
                                        ui.config_popup_state = None;
                                    },
                                    Err(error) => {
                                        ui.config_popup_state
                                            .as_mut()
                                            .unwrap()
                                            .error_message = Some(error.to_compact_string());
                                    },
                                }
                            },
                            Err(error) => {
                                ui.config_popup_state
                                    .as_mut()
                                    .unwrap()
                                    .error_message = Some(error.to_compact_string());
                            },
                        }
                    },
                    GlimEvent::CloseConfig => {
                        ui.config_popup_state = None;
                    },
                    GlimEvent::Error(error) => {
                        ui.config_popup_state
                            .as_mut()
                            .unwrap()
                            .error_message = Some(error.to_compact_string());
                    },
                    GlimEvent::Shutdown => {},
                    _ => {},
                }
            });

            if ui.config_popup_state.is_none() {
                break;
            }

            tui.draw(|f| {
                if let Some(config_popup) = ui.config_popup_state.as_mut() {
                    render_config_popup(f, config_popup, ui.last_frame, f.area())
                }
            })?;
        }

        if let Some(config) = valid_config {
            Ok(config)
        } else {
            tui.exit()?;
            exit(0)
        }
    }
}
