use std::{path::PathBuf, sync::mpsc::Sender};

use compact_str::{format_compact, CompactString, ToCompactString};
use ratatui::layout::Rect;
use serde::{Deserialize, Serialize};
use tachyonfx::{Duration, EffectManager};
use tracing::{debug, info, instrument, warn};

use crate::{
    client::{ClientConfig, GitlabService},
    config::save_config,
    dispatcher::Dispatcher,
    domain::Project,
    effect_registry::{EffectRegistry, FxId},
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
    effect_manager: EffectManager<FxId>,
    config_path: PathBuf,
    gitlab: GitlabService,
    last_tick: std::time::Instant,
    sender: Sender<GlimEvent>,
    project_store: ProjectStore,
    notices: NoticeService,
    input: InputMultiplexer,
    clipboard: arboard::Clipboard,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
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

impl GlimConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.gitlab_url.trim().is_empty() {
            return Err("gitlab_url is required".to_string());
        }
        if self.gitlab_token.trim().is_empty() {
            return Err("gitlab_token is required".to_string());
        }
        Ok(())
    }
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
            effect_manager: EffectManager::<FxId>::default(),
        }
    }

    #[instrument(skip(self, event, ui, effects), fields(event_type = ?std::mem::discriminant(&event)))]
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
            GlimEvent::Shutdown => self.running = false,

            // www
            GlimEvent::BrowseToProject(id) => {
                debug!(project_id = %id, "Opening project in browser");
                open::that(&self.project(id).url).expect("unable to open browser")
            },
            GlimEvent::BrowseToPipeline(project_id, pipeline_id) => {
                debug!(project_id = %project_id, pipeline_id = %pipeline_id, "Opening pipeline in browser");
                let project = self.project(project_id);
                let pipeline = project
                    .pipeline(pipeline_id)
                    .expect("pipeline not found");

                open::that(&pipeline.url).expect("unable to open browser");
            },
            GlimEvent::BrowseToJob(project_id, pipeline_id, job_id) => {
                debug!(project_id = %project_id, pipeline_id = %pipeline_id, job_id = %job_id, "Opening job in browser");
                let project = self.project(project_id);
                let job_url = project
                    .pipeline(pipeline_id)
                    .and_then(|p| p.job(job_id))
                    .map(|job| &job.url)
                    .expect("job not found");

                open::that(job_url).expect("unable to open browser");
            },

            GlimEvent::DownloadErrorLog(project_id, pipeline_id) => {
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

            GlimEvent::RequestActiveJobs => {
                debug!("Requesting active jobs for all projects");
                self.projects()
                    .iter()
                    .flat_map(|p| p.pipelines.iter())
                    .flatten()
                    .filter(|p| p.status.is_active() || p.has_active_jobs())
                    .for_each(|p| self.gitlab.spawn_fetch_jobs(p.project_id, p.id));
            },
            GlimEvent::RequestPipelines(id) => {
                debug!(project_id = %id, "Requesting pipelines for project");
                self.gitlab.spawn_fetch_pipelines(id, None)
            },
            GlimEvent::RequestProjects => {
                let latest_activity = self
                    .projects()
                    .iter()
                    .max_by_key(|p| p.last_activity_at)
                    .map(|p| p.last_activity_at);

                let updated_after = self
                    .projects()
                    .iter()
                    .filter(|p| p.has_active_pipelines())
                    .min_by_key(|p| p.last_activity_at)
                    .map(|p| p.last_activity_at)
                    .map_or_else(|| latest_activity, Some);

                self.gitlab.spawn_fetch_projects(updated_after)
            },
            GlimEvent::RequestJobs(project_id, pipeline_id) => {
                debug!(project_id = %project_id, pipeline_id = %pipeline_id, "Requesting jobs for pipeline");
                self.gitlab
                    .spawn_fetch_jobs(project_id, pipeline_id)
            },

            // configuration
            GlimEvent::UpdateConfig(config) => {
                let client_config = ClientConfig::from(config)
                    .with_debug_logging(self.gitlab.config().debug.log_responses);
                let _ = self.gitlab.update_config(client_config);
            },
            GlimEvent::ApplyConfiguration => {
                if let Some(config_popup) = ui.config_popup_state.as_ref() {
                    let config = config_popup.to_config();
                    let client_config = ClientConfig::from(config.clone())
                        .with_debug_logging(self.gitlab.config().debug.log_responses);

                    // Create a temporary service for validation
                    match GitlabService::new(client_config, self.sender.clone()) {
                        Ok(_service) => {
                            // Use async validation in blocking context - for now, skip validation in apply
                            // as validation is already done in config.rs
                            save_config(&self.config_path, config.clone())
                                .expect("failed to save config");
                            self.dispatch(GlimEvent::UpdateConfig(config));
                            self.dispatch(GlimEvent::CloseConfig);
                        },
                        Err(e) => {
                            let glim_error = GlimError::GeneralError(e.to_string().into());
                            self.dispatch(GlimEvent::Error(glim_error));
                        },
                    }
                }
            },

            GlimEvent::ShowLastNotification => {
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

            GlimEvent::CloseNotification => {
                ui.notice = None;
            },

            GlimEvent::ShowFilterMenu => {
                // Initialize filter input with the current temporary filter
                // The show_filter_input method will handle initialization
                ui.filter_input_active = true;
            },

            GlimEvent::ApplyFilter(filter_text) => {
                let mut config = self.load_config().unwrap_or_default();
                config.search_filter =
                    if filter_text.is_empty() { None } else { Some(filter_text) };

                save_config(&self.config_path, config.clone()).expect("failed to save config");
                self.dispatch(GlimEvent::UpdateConfig(config));
                self.dispatch(GlimEvent::RequestProjects);
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
        self.project_store.projects()
    }

    pub fn filtered_projects(
        &self,
        temporary_filter: &Option<CompactString>,
    ) -> (Vec<Project>, Vec<usize>) {
        let all_projects = self.project_store.projects();

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

    pub fn effect_manager_mut(&mut self) -> &mut EffectManager<FxId> {
        &mut self.effect_manager
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
