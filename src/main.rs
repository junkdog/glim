use std::process::exit;
use std::sync::mpsc::Sender;

use directories::BaseDirs;
use ratatui::{Frame, Terminal};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::prelude::Direction;
use tachyonfx::{Duration, EffectRenderer, Shader};
use tachyonfx::fx::term256_colors;

use crate::client::GitlabClient;
use crate::event::{EventHandler, GlimEvent};
use crate::glim_app::{GlimApp, GlimConfig, StatefulWidgets};
use crate::input::InputProcessor;
use crate::input::processor::ConfigProcessor;
use crate::result::{GlimError, Result};
use crate::theme::theme;
use crate::tui::Tui;
use crate::ui::popup::{ConfigPopup, ConfigPopupState, PipelineActionsPopup, ProjectDetailsPopup};
use crate::ui::widget::{LogsWidget, ProjectsTable};

mod tui;
mod event;
mod domain;
mod client;
mod result;
mod gruvbox;
mod stores;
mod ui;
mod glim_app;
mod theme;
mod id;
mod dispatcher;
mod input;

fn main() -> Result<()> {
    let debug = std::env::var("GLIM_DEBUG").is_ok();

    // event handler
    let event_handler = EventHandler::new(std::time::Duration::from_millis(33));
    let sender = event_handler.sender();

    // tui backend
    let backend = CrosstermBackend::new(std::io::stdout());
    let terminal = Terminal::new(backend)
        .map_err(|_| GlimError::GeneralError("failed to initialize terminal".to_string()))?;
    let mut tui = Tui::new(terminal, event_handler);
    tui.enter()?;

    let mut widget_states = StatefulWidgets::new(sender.clone());
    let config = run_config_ui_loop(&mut tui, &mut widget_states, sender.clone(), debug)?;

    // app state and initial setup
    let mut app = GlimApp::new(sender.clone(), gitlab_client(sender.clone(), config, debug));
    app.apply(GlimEvent::RequestProjects, &mut widget_states);

    // main loop
    while app.is_running() {
        widget_states.last_frame = app.process_timers();
        tui.receive_events(|event| {
            widget_states.apply(&app, &event);
            app.apply(event, &mut widget_states);
        });
        tui.draw(|f| render_widgets(f, &app, &mut widget_states))?;
    }

    tui.exit().map_err(|_| GlimError::GeneralError("failed to exit TUI".to_string()))?;
    Ok(())
}

fn render_widgets(
    f: &mut Frame,
    app: &GlimApp,
    widget_states: &mut StatefulWidgets
) {
    let last_frame_ms = widget_states.last_frame;
    let layout = if app.ui.show_internal_logs {
        Layout::new(Direction::Horizontal, [
            Constraint::Percentage(65),
            Constraint::Percentage(35),
        ]).split(f.area())
    } else {
        Layout::new(Direction::Horizontal, [
            Constraint::Percentage(100),
        ]).split(f.area())
    };

    // gitlab pipelines
    let projects = ProjectsTable::new(app.projects());
    f.render_stateful_widget(projects, layout[0], &mut widget_states.project_table_state);

    // internal logs
    if app.ui.show_internal_logs {
        let raw_logs = app.logs();
        let logs = LogsWidget::from(&raw_logs);
        *widget_states.logs_state.selected_mut() = Some(raw_logs.len());
        f.render_stateful_widget(logs, layout[1], &mut widget_states.logs_state);
    }

    // project details popup
    if let Some(project_details) = widget_states.project_details.as_mut() {
        let popup = ProjectDetailsPopup::new(last_frame_ms);
        let popup_area = layout[0].inner(Margin::new(6, 2));

        // f.render_effect(popup_area, &mut project_details.fade_in, last_frame_ms);
        f.render_stateful_widget(popup, popup_area, project_details);
    }
    
    // pipeline actions popup
    if let Some(pipeline_actions) = widget_states.pipeline_actions.as_mut() {
        let project = app.project(pipeline_actions.project_id);
        let popup = PipelineActionsPopup::from(last_frame_ms, project);

        // render popup on top
        f.render_stateful_widget(popup, layout[0], pipeline_actions);
    }

    let last_tick = last_frame_ms;;
    // glitch shader
    f.render_effect(widget_states.glitch(), f.area(), last_tick);

    // fade in table
    if let Some(shader) = &mut widget_states.table_fade_in {
        f.render_effect(shader, layout[0], last_frame_ms);
        if shader.done() {
            widget_states.table_fade_in = None;
        }
        
    }

    if let Some(config_popup) = &mut widget_states.config_popup_state {
        // f.render_effect(&mut config_popup.parent_fade, last_frame_ms);
        render_config_popup(f, config_popup, last_frame_ms, layout[0]);
    }

    // modal alert.rs message
    if let Some(alert) = widget_states.alert() {
        f.render_widget(alert.clone(), layout[0])
    }
    
    // shader experiment
    if let Some(shader) = widget_states.shader_pipeline.as_mut() {
        f.render_effect(shader, f.area(), last_tick);
        if shader.done() {
            widget_states.shader_pipeline = None;
        }
    }
    
    if app.ui.use_256_colors {
        f.render_effect(&mut term256_colors(), f.area(), last_tick);
    }
}

fn render_config_popup(
    f: &mut Frame,
    config_popup: &mut ConfigPopupState,
    last_tick: Duration,
    layout: Rect
) {
    // render widget
    let popup = ConfigPopup::new(last_tick);
    f.render_stateful_widget(popup, layout, config_popup);

    // render cursor once UI has ~faded in
    if config_popup.is_open_complete() {
        let cursor = config_popup.cursor_position;
        f.buffer_mut().set_style(Rect::new(cursor.x, cursor.y, 1, 1), theme().input_selected);
        f.set_cursor_position(cursor);
    }
}

fn gitlab_client(
    sender: Sender<GlimEvent>,
    config: GlimConfig,
    debug: bool,
) -> GitlabClient {
    GitlabClient::new(
        sender,
        config.gitlab_url,
        config.gitlab_token,
        config.search_filter,
        debug,
    )
}

/// Read the configuration file from $HOME/.config/glim.toml
pub fn read_config() -> Result<GlimConfig> {
    return if let Some(dirs) = BaseDirs::new() {
        let config_file = dirs.config_dir().join("glim.toml");
        if config_file.exists() {
            let config: GlimConfig = confy::load_path(config_file)
                .map_err(GlimError::ConfigError)?;
            
            Ok(config)
        } else {
            eprintln!("Unable to find configuration file at {:?}", config_file);
            exit(3)
        }
    } else {
        eprintln!("Unable to determine home directory");
        exit(2)
    };
}

pub fn save_config(config: GlimConfig) -> Result<()> {
    if let Some(dirs) = BaseDirs::new() {
        let config_file = dirs.config_dir().join("glim.toml");
        confy::store_path(config_file, &config)
            .map_err(GlimError::ConfigError)?;
        Ok(())
    } else {
        eprintln!("Unable to determine home directory");
        exit(2)
    }
}


/// Run the configuration UI loop to create the configuration file.
/// If the configuration file already exists, it is loaded and returned.
pub fn run_config_ui_loop(
    tui: &mut Tui,
    ui: &mut StatefulWidgets,
    sender: Sender<GlimEvent>,
    debug: bool,
) -> Result<GlimConfig> {
    if let Some(dirs) = BaseDirs::new() {
        let config_file = dirs.config_dir().join("glim.toml");
        if config_file.exists() {
            let config: GlimConfig = confy::load_path(config_file)
                .map_err(GlimError::ConfigError)?;

            Ok(config)
        } else {
            ui.config_popup_state = Some(ConfigPopupState::new(&GlimConfig::default()));
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
                            let config = ui.config_popup_state.as_ref().unwrap().to_config();
                            match config.validate() {
                                Ok(_) => {
                                    let client = GitlabClient::new_from_config(sender.clone(), config, debug);
                                    match client.validate_configuration() {
                                        Ok(_) => {
                                            let state = ui.config_popup_state.as_ref().unwrap();
                                            save_config(state.to_config())
                                                .expect("failed to save configuration");

                                            valid_config = Some(state.to_config());
                                            ui.config_popup_state = None;
                                        }
                                        Err(error) => {
                                            ui.config_popup_state.as_mut().unwrap().error_message = Some(error.to_string());
                                        }
                                    }
                                }
                                Err(error) => {
                                    ui.config_popup_state.as_mut().unwrap().error_message = Some(error.to_string());
                                }
                            }
                        }
                        GlimEvent::CloseConfig => {
                            ui.config_popup_state = None;
                        }
                        GlimEvent::Error(error) => {
                            ui.config_popup_state.as_mut().unwrap().error_message = Some(error.to_string());
                        }
                        GlimEvent::Shutdown => {}
                        _ => {}
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
            };

            if let Some(config) = valid_config {
                Ok(config)
            } else {
                tui.exit()?;
                exit(0)
            }
        }
    } else {
        eprintln!("Unable to determine home directory");
        exit(2)
    }
}
