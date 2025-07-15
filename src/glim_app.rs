use std::{path::PathBuf, sync::mpsc::Sender};

use compact_str::{format_compact, CompactString, ToCompactString};
use ratatui::layout::Rect;
use serde::{Deserialize, Serialize};
use tachyonfx::Duration;
use tracing::{debug, info, instrument, warn};

use crate::{
    client::{ClientConfig, GitlabService},
    config::save_config,
    dispatcher::Dispatcher,
    domain::Project,
    effect_registry::EffectRegistry,
    event::GlimEvent,
    id::ProjectId,
    input::{processor::NormalModeProcessor, InputMultiplexer},
    notice_service::{Notice, NoticeLevel, NoticeService},
    result::GlimError,
    stores::{log_event, ProjectStore},
    ui::{
        widget::{NotificationState, RefRect},
        StatefulWidgets,
    },
};

pub struct GlimApp {
    running: bool,
    config_path: PathBuf,
    gitlab: GitlabService,
    last_tick: std::time::Instant,
    sender: Sender<GlimEvent>,
    project_store: ProjectStore,
    notices: NoticeService,
    input: InputMultiplexer,
    clipboard: arboard::Clipboard,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct GlimConfig {
    /// The URL of the GitLab instance
    pub gitlab_url: CompactString,
    /// The Personal Access Token to authenticate with GitLab
    pub gitlab_token: CompactString,
    /// Filter applied to the projects list
    pub search_filter: Option<CompactString>,
    /// Logging level: Off, Error, Warn, Info, Debug, Trace
    pub log_level: Option<CompactString>,
}

impl GlimApp {
    pub fn new(sender: Sender<GlimEvent>, config_path: PathBuf, gitlab: GitlabService) -> Self {
        let mut input = InputMultiplexer::new(sender.clone());
        input.push(Box::new(NormalModeProcessor::new(sender.clone())));

        Self {
            running: true,
            config_path,
            gitlab,
            last_tick: std::time::Instant::now(),
            sender: sender.clone(),
            project_store: ProjectStore::new(sender),
            notices: NoticeService::new(),
            input,
            clipboard: arboard::Clipboard::new().expect("failed to create clipboard"),
        }
    }

    #[instrument(skip(self, event, ui, effects), fields(event_type = %event.variant_name()))]
    pub fn apply(
        &mut self,
        event: GlimEvent,
        ui: &mut StatefulWidgets,
        effects: &mut EffectRegistry,
    ) {
        self.input.apply(&event, ui);
        log_event(&event);
        effects.apply(&event);
        self.notices.apply(&event);
        self.project_store.apply(&event);

        match event {
            GlimEvent::AppExit => self.running = false,

            // www
            GlimEvent::ProjectOpenUrl(id) => {
                debug!(project_id = %id, "Opening project in browser");
                open::that(&self.project(id).url).expect("unable to open browser")
            },
            GlimEvent::PipelineOpenUrl(project_id, pipeline_id) => {
                debug!(project_id = %project_id, pipeline_id = %pipeline_id, "Opening pipeline in browser");
                let project = self.project(project_id);
                let pipeline = project
                    .pipeline(pipeline_id)
                    .expect("pipeline not found");

                open::that(&pipeline.url).expect("unable to open browser");
            },
            GlimEvent::JobOpenUrl(project_id, pipeline_id, job_id) => {
                debug!(project_id = %project_id, pipeline_id = %pipeline_id, job_id = %job_id, "Opening job in browser");
                let project = self.project(project_id);
                let job_url = project
                    .pipeline(pipeline_id)
                    .and_then(|p| p.job(job_id))
                    .map(|job| &job.url)
                    .expect("job not found");

                open::that(job_url).expect("unable to open browser");
            },

            GlimEvent::JobLogFetch(project_id, pipeline_id) => {
                debug!(project_id = %project_id, pipeline_id = %pipeline_id, "Downloading error log");
                let project = self.project(project_id);
                let pipeline = project
                    .pipeline(pipeline_id)
                    .expect("pipeline not found");

                let job = pipeline
                    .failed_job()
                    .expect("no failed job found");

                self.gitlab
                    .spawn_download_job_log(project_id, job.id);
            },
            GlimEvent::JobLogDownloaded(project_id, job_id, trace) => {
                info!(project_id = %project_id, job_id = %job_id, trace_length = trace.len(), "Job log downloaded and copied to clipboard");
                self.clipboard.set_text(trace).unwrap();
            },

            GlimEvent::JobsActiveFetch => {
                debug!("Requesting active jobs for all projects");
                self.project_store
                    .sorted_projects()
                    .iter()
                    .flat_map(|p| p.pipelines.iter())
                    .flatten()
                    .filter(|p| p.status.is_active() || p.has_active_jobs())
                    .for_each(|p| self.gitlab.spawn_fetch_jobs(p.project_id, p.id));
            },
            GlimEvent::PipelinesFetch(id) => {
                debug!(project_id = %id, "Requesting pipelines for project");
                self.gitlab.spawn_fetch_pipelines(id, None)
            },
            GlimEvent::ProjectsFetch => {
                let latest_activity = self
                    .project_store
                    .sorted_projects()
                    .iter()
                    .max_by_key(|p| p.last_activity_at)
                    .map(|p| p.last_activity_at);

                let updated_after = self
                    .project_store
                    .sorted_projects()
                    .iter()
                    .filter(|p| p.has_active_pipelines())
                    .min_by_key(|p| p.last_activity_at)
                    .map(|p| p.last_activity_at)
                    .map_or_else(|| latest_activity, Some);

                self.gitlab.spawn_fetch_projects(updated_after)
            },
            GlimEvent::JobsFetch(project_id, pipeline_id) => {
                debug!(project_id = %project_id, pipeline_id = %pipeline_id, "Requesting jobs for pipeline");
                self.gitlab
                    .spawn_fetch_jobs(project_id, pipeline_id)
            },

            // configuration
            GlimEvent::ConfigUpdate(config) => {
                let client_config = ClientConfig::from(config)
                    .with_debug_logging(self.gitlab.config().debug.log_responses);
                let _ = self.gitlab.update_config(client_config);
            },
            GlimEvent::ConfigApply => {
                if let Some(config_popup) = ui.config_popup_state.as_ref() {
                    let config = config_popup.to_config();
                    let client_config = ClientConfig::from(config.clone())
                        .with_debug_logging(self.gitlab.config().debug.log_responses);

                    // Create a temporary service for validation
                    match self.gitlab.update_config(client_config) {
                        Ok(_) => {
                            save_config(&self.config_path, config.clone())
                                .expect("failed to save config");
                            self.dispatch(GlimEvent::ConfigUpdate(config));
                            self.dispatch(GlimEvent::ConfigClose);
                        },
                        Err(e) => {
                            let glim_error = GlimError::GeneralError(e.to_string().into());
                            self.dispatch(GlimEvent::AppError(glim_error));
                        },
                    }
                }
            },

            GlimEvent::NotificationLast => {
                if let Some(notice) = self.notices.last_notification() {
                    let content_area = RefRect::new(Rect::default());
                    effects.register_notification_effect(content_area.clone());
                    ui.notice = Some(NotificationState::new(
                        notice.clone(),
                        &self.project_store,
                        content_area,
                    ));
                }
            },

            GlimEvent::NotificationDismiss => {
                ui.notice = None;
            },

            GlimEvent::FilterMenuShow => {
                // Initialize filter input with the current temporary filter
                // The show_filter_input method will handle initialization
                ui.filter_input_active = true;
            },

            GlimEvent::ScreenCapture => {
                debug!("Screen capture requested");
                // The actual screen capture will be handled in the rendering loop
                // where we have access to the frame buffer
                ui.capture_screen_requested = true;
            },

            GlimEvent::ScreenCaptureToClipboard(ansi_string) => {
                debug!("Copying screen capture to clipboard");
                match self.clipboard.set_text(ansi_string) {
                    Ok(_) => {
                        info!("Screen buffer captured and copied to clipboard");
                    },
                    Err(e) => {
                        warn!(error = %e, "Failed to copy screen capture to clipboard");
                    },
                }
            },

            _ => {},
        }

        // if there are any error notifications, and the current notification is an info notice, dismiss it
        if self.notices.has_error()
            && ui
                .notice
                .as_ref()
                .map(|n| n.notice.level == NoticeLevel::Info)
                .unwrap_or(false)
        {
            ui.notice = None;
        }

        if ui.notice.is_none() {
            // if there's a notice waiting, update fetch it
            if let Some(notice) = self.pop_notice() {
                let content_area = RefRect::new(Rect::default());
                effects.register_notification_effect(content_area.clone());
                ui.notice = Some(NotificationState::new(notice, &self.project_store, content_area));
            }
        }
    }

    pub fn load_config(&self) -> Result<GlimConfig, GlimError> {
        let config_file = &self.config_path;
        if config_file.exists() {
            let config: GlimConfig = confy::load_path(config_file)
                .map_err(|e| GlimError::ConfigError(e.to_compact_string()))?;

            Ok(config)
        } else {
            Err(GlimError::ConfigError(format_compact!(
                "Unable to find configuration file at {:?}",
                config_file
            )))
        }
    }

    pub fn process_timers(&mut self) -> Duration {
        let now = std::time::Instant::now();
        let elapsed = now - self.last_tick;
        self.last_tick = now;

        // do nothing with elapsed time for now;
        // and consider moving to UiState

        Duration::from_millis(elapsed.as_millis() as u32)
    }

    pub fn project(&self, id: ProjectId) -> &Project {
        self.project_store
            .find(id)
            .expect("project not found")
    }

    pub fn projects(&self) -> &[Project] {
        self.project_store.sorted_projects()
    }

    pub fn filtered_projects(
        &self,
        temporary_filter: &Option<CompactString>,
    ) -> (Vec<Project>, Vec<usize>) {
        let all_projects = self.project_store.sorted_projects();

        if let Some(filter) = temporary_filter {
            if !filter.trim().is_empty() {
                let filter_lower = filter.to_lowercase();
                let mut filtered_projects = Vec::new();
                let mut filtered_indices = Vec::new();

                for (index, project) in all_projects.iter().enumerate() {
                    if project
                        .path
                        .to_lowercase()
                        .contains(filter_lower.as_str())
                        || project
                            .description
                            .as_ref()
                            .is_some_and(|d| d.to_lowercase().contains(filter_lower.as_str()))
                    {
                        filtered_projects.push(project.clone());
                        filtered_indices.push(index);
                    }
                }

                return (filtered_projects, filtered_indices);
            }
        }

        (all_projects.to_vec(), (0..all_projects.len()).collect())
    }

    pub fn sender(&self) -> Sender<GlimEvent> {
        self.sender.clone()
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn pop_notice(&mut self) -> Option<Notice> {
        self.notices.pop_notice()
    }
}

impl Dispatcher for GlimApp {
    fn dispatch(&self, event: GlimEvent) {
        self.sender.send(event).unwrap_or(());
    }
}

#[allow(unused)]
pub fn modulo(a: u32, b: u32) -> u32 {
    if b == 0 {
        return 0;
    }

    let a = a as i32;
    let b = b as i32;
    ((a % b) + b) as u32 % b as u32
}

pub trait Modulo {
    fn modulo(self, b: Self) -> Self;
}

impl Modulo for i32 {
    fn modulo(self, b: i32) -> i32 {
        if b == 0 {
            return 0;
        }

        ((self % b) + b) % b
    }
}

impl Modulo for u32 {
    fn modulo(self, b: u32) -> u32 {
        if b == 0 {
            return 0;
        }

        (self as i32).modulo(b as i32) as u32
    }
}

impl Modulo for isize {
    fn modulo(self, b: isize) -> isize {
        if b == 0 {
            return 0;
        }

        ((self % b) + b) % b
    }
}

impl Modulo for usize {
    fn modulo(self, b: usize) -> usize {
        if b == 0 {
            return 0;
        }

        (self as isize).modulo(b as isize) as usize
    }
}
