use std::{path::PathBuf, process::exit, sync::mpsc::Sender};

use compact_str::ToCompactString;
use directories::BaseDirs;
use ratatui::Frame;
use tachyonfx::Duration;

use crate::{
    client::GitlabClient,
    event::GlimEvent,
    glim_app::GlimConfig,
    input::{processor::ConfigProcessor, InputProcessor},
    result::{GlimError, Result},
    tui::Tui,
    ui::{popup::ConfigPopupState, widget::RefRect, StatefulWidgets},
};

pub fn default_config_path() -> PathBuf {
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

fn render_config_popup(
    f: &mut Frame,
    config_popup: &mut ConfigPopupState,
    _last_tick: Duration,
    layout: ratatui::prelude::Rect,
) {
    use ratatui::prelude::Rect;

    use crate::{theme::theme, ui::popup::ConfigPopup};

    let popup = ConfigPopup::new();
    f.render_stateful_widget(popup, layout, config_popup);

    if true {
        let cursor = config_popup.cursor_position;
        f.buffer_mut()
            .set_style(Rect::new(cursor.x, cursor.y, 1, 1), theme().input_selected);
        f.set_cursor_position(cursor);
    }
}
