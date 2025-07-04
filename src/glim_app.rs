use std::path::PathBuf;
use std::sync::mpsc::Sender;

use compact_str::{format_compact, CompactString, ToCompactString};
use serde::{Deserialize, Serialize};
use tachyonfx::Duration;

use crate::client::GitlabClient;
use crate::dispatcher::Dispatcher;
use crate::domain::Project;
use crate::event::GlimEvent;
use crate::id::ProjectId;
use crate::input::processor::NormalModeProcessor;
use crate::input::InputMultiplexer;
use crate::notice_service::{Notice, NoticeLevel, NoticeService};
use crate::result::GlimError;
use crate::save_config;
use crate::stores::{log_event, ProjectStore};
use crate::ui::widget::NotificationState;
use crate::ui::StatefulWidgets;

pub struct GlimApp {
    running: bool,
    config_path: PathBuf,
    gitlab: GitlabClient,
    last_tick: std::time::Instant,
    pub sender: Sender<GlimEvent>,
    project_store: ProjectStore,
    notices: NoticeService,
    input: InputMultiplexer,
    clipboard: arboard::Clipboard,
    pub ui: UiState,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct GlimConfig {
    /// The URL of the GitLab instance
    pub gitlab_url: CompactString,
    /// The Personal Access Token to authenticate with GitLab
    pub gitlab_token: CompactString,
    /// Filter applied to the projects list
    pub search_filter: Option<CompactString>,
}

pub struct UiState {
    pub use_256_colors: bool,
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
    pub fn new(sender: Sender<GlimEvent>, config_path: PathBuf, gitlab: GitlabClient) -> Self {
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
            ui: UiState::new(),
        }
    }

    pub fn apply(&mut self, event: GlimEvent, ui: &mut StatefulWidgets) {
        self.input.apply(&event, ui);
        self.ui.apply(&event);
        log_event(&event);
        self.notices.apply(&event);
        self.project_store.apply(&event);

        match event {
            GlimEvent::Shutdown => self.running = false,

            // www
            GlimEvent::BrowseToProject(id) => {
                open::that(&self.project(id).url).expect("unable to open browser")
            }
            GlimEvent::BrowseToPipeline(project_id, pipeline_id) => {
                let project = self.project(project_id);
                let pipeline = project.pipeline(pipeline_id).expect("pipeline not found");

                open::that(&pipeline.url).expect("unable to open browser");
            }
            GlimEvent::BrowseToJob(project_id, pipeline_id, job_id) => {
                let project = self.project(project_id);
                let job_url = project
                    .pipeline(pipeline_id)
                    .and_then(|p| p.job(job_id))
                    .map(|job| &job.url)
                    .expect("job not found");

                open::that(job_url).expect("unable to open browser");
            }

            GlimEvent::DownloadErrorLog(project_id, pipeline_id) => {
                let project = self.project(project_id);
                let pipeline = project.pipeline(pipeline_id).expect("pipeline not found");

                let job = pipeline.failed_job().expect("no failed job found");

                self.gitlab.dispatch_download_job_log(project_id, job.id);
            }
            GlimEvent::JobLogDownloaded(_, _, trace) => {
                self.clipboard.set_text(trace).unwrap();
            }

            GlimEvent::RequestActiveJobs => {
                self.projects()
                    .iter()
                    .flat_map(|p| p.pipelines.iter())
                    .flatten()
                    .filter(|p| p.status.is_active() || p.has_active_jobs())
                    .for_each(|p| self.gitlab.dispatch_get_jobs(p.project_id, p.id));
            }
            GlimEvent::RequestPipelines(id) => self.gitlab.dispatch_get_pipelines(id, None),
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

                self.gitlab.dispatch_list_projects(updated_after)
            }
            GlimEvent::RequestJobs(project_id, pipeline_id) => {
                self.gitlab.dispatch_get_jobs(project_id, pipeline_id)
            }

            // configuration
            GlimEvent::UpdateConfig(config) => self.gitlab.update_config(config),
            GlimEvent::ApplyConfiguration => {
                if let Some(config_popup) = ui.config_popup_state.as_ref() {
                    let config = config_popup.to_config();
                    let client = GitlabClient::new_from_config(
                        self.sender.clone(),
                        config.clone(),
                        self.gitlab.debug(),
                    );
                    match client.validate_configuration() {
                        Ok(_) => {
                            save_config(&self.config_path, config.clone())
                                .expect("failed to save config");
                            self.dispatch(GlimEvent::UpdateConfig(config));
                            self.dispatch(GlimEvent::CloseConfig);
                        }
                        Err(e) => {
                            self.dispatch(GlimEvent::Error(e));
                        }
                    }
                }
            }

            GlimEvent::ShowLastNotification => {
                if let Some(notice) = self.notices.last_notification() {
                    ui.notice = Some(NotificationState::new(notice.clone(), &self.project_store));
                }
            }

            GlimEvent::ShowFilterMenu => {
                // Initialize filter input with the current temporary filter
                // The show_filter_input method will handle initialization
                ui.filter_input_active = true;
            }

            GlimEvent::ApplyFilter(filter_text) => {
                let mut config = self.load_config().unwrap_or_default();
                config.search_filter = if filter_text.is_empty() {
                    None
                } else {
                    Some(filter_text)
                };

                save_config(&self.config_path, config.clone()).expect("failed to save config");
                self.dispatch(GlimEvent::UpdateConfig(config));
                self.dispatch(GlimEvent::RequestProjects);
            }

            _ => {}
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
                ui.notice = Some(NotificationState::new(notice, &self.project_store));
            }
        }
    }

    pub fn load_config(&self) -> Result<GlimConfig, GlimError> {
        let config_file = &self.config_path;
        if config_file.exists() {
            let config: GlimConfig =
                confy::load_path(config_file).map_err(|e| GlimError::ConfigError(e.to_compact_string()))?;

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
        self.project_store.find(id).expect("project not found")
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
                    if project.path.to_lowercase().contains(filter_lower.as_str())
                        || project
                            .description
                            .as_ref()
                            .map_or(false, |d| d.to_lowercase().contains(filter_lower.as_str()))
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


    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn last_frame_time(&self) -> u32 {
        self.last_tick.elapsed().as_millis() as u32
    }

    pub fn pop_notice(&mut self) -> Option<Notice> {
        self.notices.pop_notice()
    }
}

impl UiState {
    pub fn new() -> Self {
        Self {
            use_256_colors: false,
        }
    }

    pub fn apply(&mut self, event: &GlimEvent) {
        match event {
            GlimEvent::ToggleColorDepth => self.use_256_colors = !self.use_256_colors,
            _ => (),
        }
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
