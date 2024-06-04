use std::sync::mpsc::Sender;

use chrono::{DateTime, Local};
use rand::prelude::{SeedableRng, SmallRng};
use ratatui::widgets::{ListState, TableState};
use serde::{Deserialize, Serialize};

use crate::{save_config, shader};
use crate::client::GitlabClient;
use crate::dispatcher::Dispatcher;
use crate::domain::Project;
use crate::event::GlimEvent;
use crate::gruvbox::Gruvbox::{Dark0Hard, Dark3};
use crate::id::{PipelineId, ProjectId};
use crate::input::InputMultiplexer;
use crate::input::processor::NormalModeProcessor;
use crate::interpolation::Interpolation;
use crate::shader::{Effect, IntoEffect, parallel};
use crate::shader::fx::Glitch;
use crate::stores::{InternalLogsStore, ProjectStore};
use crate::ui::popup::{AlertPopup, ConfigPopupState, PipelineActionsPopupState, ProjectDetailsPopupState};

pub struct GlimApp {
    running: bool,
    gitlab: GitlabClient,
    last_tick: std::time::Instant,
    sender: Sender<GlimEvent>,
    project_store: ProjectStore,
    logs_store: InternalLogsStore,
    input: InputMultiplexer,
    clipboard: arboard::Clipboard,
    pub ui: UiState,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct GlimConfig {
    /// The URL of the GitLab instance
    pub gitlab_url: String,
    /// The Personal Access Token to authenticate with GitLab
    pub gitlab_token: String,
    /// Filter applied to the projects list
    pub search_filter: Option<String>
}

pub struct UiState {
    pub show_internal_logs: bool,
    pub use_256_colors: bool,
}

pub struct StatefulWidgets {
    pub last_frame_ms: u32,
    pub sender: Sender<GlimEvent>,
    pub table_state: TableState,
    pub logs_state: ListState,
    pub config_popup_state: Option<ConfigPopupState>,
    pub table_fade_in: Option<Effect>,
    pub project_details: Option<ProjectDetailsPopupState>,
    pub pipeline_actions: Option<PipelineActionsPopupState>,
    pub shader_pipeline: Option<Effect>,
    glitch_override: Option<Effect>,
    glitch: Effect,
    alerts: Vec<AlertPopup>,
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

impl StatefulWidgets {
    pub fn new(sender: Sender<GlimEvent>) -> Self {
        Self {
            last_frame_ms: 0,
            sender,
            table_state: TableState::default().with_selected(0),
            logs_state: ListState::default().with_selected(Some(0)),
            table_fade_in: None,
            config_popup_state: None,
            project_details: None,
            pipeline_actions: None,
            alerts: Vec::new(),
            shader_pipeline: None,
            glitch_override: None,
            glitch: Glitch::builder()
                .action_ms(100..500)
                .action_start_delay_ms(0..2000)
                .cell_glitch_ratio(0.0015)
                .rng(SmallRng::from_entropy())
                .into()
        }
    }

    pub fn apply(
        &mut self,
        app: &GlimApp,
        event: &GlimEvent
    ) {
        match event {
            GlimEvent::GlitchOverride(g)            => self.glitch_override = g.clone().map(|g| g.into_effect()),

            GlimEvent::SelectNextProject            => self.handle_project_selection(1, app),
            GlimEvent::SelectPreviousProject        => self.handle_project_selection(-1, app),

            GlimEvent::ReceivedProjects(_)          => self.fade_in_projects_table(),

            GlimEvent::OpenProjectDetails(id)       => self.open_project_details(app.project(*id).clone(), app.sender.clone()),
            GlimEvent::CloseProjectDetails          => self.project_details = {
                let fade_in = shader::fade_from(Dark3, Dark0Hard, 300, Interpolation::PowIn(2));
                self.shader_pipeline = Some(fade_in);

                None
            },
            GlimEvent::ProjectUpdated(p)            => self.refresh_project_details(p),

            GlimEvent::DisplayAlert(msg)            => self.alerts.push(AlertPopup::new(msg.clone())),
            GlimEvent::CloseAlert                   => { self.alerts.pop(); },
            
            GlimEvent::ClosePipelineActions         => self.close_pipeline_actions(),
            GlimEvent::OpenPipelineActions(project_id, pipeline_id) => {
                let project = app.project(*project_id);
                self.open_pipeline_actions(project, *pipeline_id);
            },

            GlimEvent::DisplayConfig(config)        => self.open_config(config),
            GlimEvent::CloseConfig                  => self.config_popup_state = None,

            _ => (),
        }
    }

    fn fade_in_projects_table(&mut self) {
        let effect = parallel(vec![
            shader::coalesce(550, 2_000, Interpolation::Linear),
            shader::sweep_in(50, Dark0Hard, 450, Interpolation::PowIn(2))
        ]);
        self.table_fade_in = Some(effect);
    }

    fn refresh_project_details(&mut self, project: &Project) {
        let requires_refresh = self.project_details.as_ref()
            .map_or(false, |pd| pd.project.id == project.id);

        if requires_refresh {
            let existing = self.project_details.take().unwrap();
            self.project_details = Some(existing.with_project(project.clone()));
        }
    }

    fn open_project_details(&mut self, project: Project, sender: Sender<GlimEvent>) {
        project.recent_pipelines().first()
            .map(|p| sender.dispatch(GlimEvent::SelectedPipeline(p.id)))
            .unwrap_or(());

        self.project_details = Some(ProjectDetailsPopupState::new(project));
    }

    fn open_config(&mut self, config: &GlimConfig) {
        self.config_popup_state = Some(ConfigPopupState::new(config));
    }

    fn open_pipeline_actions(
        &mut self,
        project: &Project,
        pipeline_id: PipelineId
    ) {
        let failed_job = project
            .pipeline(pipeline_id)
            .and_then(|p| p.failed_job());
        
        let actions = if let Some(job) = failed_job {
            vec![
                GlimEvent::BrowseToJob(project.id, pipeline_id, job.id),
                GlimEvent::BrowseToPipeline(project.id, pipeline_id),
                GlimEvent::BrowseToProject(project.id),
                GlimEvent::DownloadErrorLog(project.id, pipeline_id),
            ]
        } else {
            vec![
                GlimEvent::BrowseToPipeline(project.id, pipeline_id),
                GlimEvent::BrowseToProject(project.id),
            ]
        };
        
        self.pipeline_actions = Some(PipelineActionsPopupState::new(actions, project.id, pipeline_id));
    }
    
    pub fn alert(&self) -> Option<&AlertPopup> {
        self.alerts.last()
    }

    fn close_pipeline_actions(&mut self) {
        self.pipeline_actions = None;
    }

    fn handle_project_selection(&mut self, direction: i32, app: &GlimApp) {
        let projects = app.projects();
        if let Some(current) = self.table_state.selected() {
            let new_index = match direction {
                1  => current.saturating_add(1),
                -1 => current.saturating_sub(1),
                n  => panic!("invalid direction: {n}")
            }.min(projects.len().saturating_sub(1));

            self.table_state.select(Some(new_index));
            let project = &projects[new_index];
            app.dispatch(GlimEvent::SelectedProject(project.id));
        }
    }

    pub fn handle_pipeline_selection(&mut self, direction: i32) {
        if self.project_details.is_none() { return; }
        let pd = self.project_details.as_mut().unwrap();

        if let Some(current) = pd.pipelines_table_state.selected() {
            let pipelines = pd.project.recent_pipelines();
            
            let new_index = (current as i32 + direction)
                .modulo(pipelines.len() as i32) as usize;
            
            if pipelines.is_empty() {
                pd.pipelines_table_state.select(None);
            } else {
                pd.pipelines_table_state.select(Some(new_index));
                let pipeline = &pipelines[new_index];
                self.sender.dispatch(GlimEvent::SelectedPipeline(pipeline.id));
            }
        }
    }
    
    pub fn handle_pipeline_action_selection(&mut self, direction: i32) {
        if self.pipeline_actions.is_none() { return; }

        let pipelines = self.pipeline_actions.as_mut().unwrap();
        if let Some(current) = pipelines.list_state.selected() {
            let new_index = (current as i32 + direction)
                .modulo(pipelines.actions.len() as i32);

            pipelines.list_state.select(Some(new_index as usize));
        }
    }

    pub fn glitch(&mut self) -> &mut Effect {
        match self.glitch_override.as_mut() {
            Some(g) => g,
            None => &mut self.glitch
        }
    }
}

impl GlimApp {
    pub fn new(
        sender: Sender<GlimEvent>,
        gitlab: GitlabClient,
    ) -> Self {
        let mut input = InputMultiplexer::new(sender.clone());
        input.push(Box::new(NormalModeProcessor::new(sender.clone())));

        Self {
            running: true,
            gitlab,
            last_tick: std::time::Instant::now(),
            sender: sender.clone(),
            project_store: ProjectStore::new(sender),
            logs_store: InternalLogsStore::new(),
            input,
            clipboard: arboard::Clipboard::new().expect("failed to create clipboard"),
            ui: UiState::new(),
        }
    }

    pub fn apply(&mut self, event: GlimEvent, ui: &mut StatefulWidgets) {
        self.input.apply(&event, ui);
        self.ui.apply(&event);
        self.logs_store.apply(&event);
        self.project_store.apply(&event);

        match event {
            GlimEvent::Shutdown                 => self.running = false,
            
            // www
            GlimEvent::BrowseToProject(id) => open::that(&self.project(id).url)
                .expect("unable to open browser"),
            GlimEvent::BrowseToPipeline(project_id, pipeline_id) => {
                let project = self.project(project_id);
                let pipeline = project.pipeline(pipeline_id)
                    .expect("pipeline not found");

                open::that(&pipeline.url)
                    .expect("unable to open browser");
            },
            GlimEvent::BrowseToJob(project_id, pipeline_id, job_id) => {
                let project = self.project(project_id);
                let job_url = project.pipeline(pipeline_id)
                    .and_then(|p| p.job(job_id))
                    .map(|job| &job.url)
                    .expect("job not found");

                open::that(job_url)
                    .expect("unable to open browser");
            },

            GlimEvent::DownloadErrorLog(project_id, pipeline_id) => {
                let project = self.project(project_id);
                let pipeline = project.pipeline(pipeline_id)
                    .expect("pipeline not found");

                let job = pipeline.failed_job()
                    .expect("no failed job found");

                self.gitlab.dispatch_download_job_log(project_id, job.id);
            },
            GlimEvent::JobLogDownloaded(_, _, trace) => {
                self.clipboard.set_text(trace).unwrap();
                self.dispatch(GlimEvent::DisplayAlert("Job log copied to clipboard".to_string()));
            },
            GlimEvent::Error(e) => {
                self.dispatch(GlimEvent::DisplayAlert(e.to_string()));
            },

            GlimEvent::RequestActiveJobs => {
                self.projects().iter()
                    .flat_map(|p| p.pipelines.iter())
                    .flatten()
                    .filter(|p| p.status.is_active() || p.has_active_jobs())
                    .for_each(|p| self.gitlab.dispatch_get_jobs(p.project_id, p.id));
            }
            GlimEvent::RequestPipelines(id)     =>
                self.gitlab.dispatch_get_pipelines(id, None),
            GlimEvent::RequestProjects          => {
                let latest_activity = self.projects().iter()
                    .max_by_key(|p| p.last_activity_at)
                    .map(|p| p.last_activity_at);

                let updated_after = self.projects().iter()
                    .filter(|p| p.has_active_pipelines())
                    .min_by_key(|p| p.last_activity_at)
                    .map(|p| p.last_activity_at)
                    .map_or_else(|| latest_activity, Some);

                self.gitlab.dispatch_list_projects(updated_after)
            },
            GlimEvent::RequestJobs(project_id, pipeline_id) =>
                self.gitlab.dispatch_get_jobs(project_id, pipeline_id),
            
            // configuration 
            GlimEvent::UpdateConfig(config) => self.gitlab.update_config(config),
            GlimEvent::ApplyConfiguration => {
                if let Some(config_popup) = ui.config_popup_state.as_ref() {
                    let config = config_popup.to_config();
                    let client = GitlabClient::new_from_config(self.sender.clone(), config.clone());
                    match client.validate_configuration() {
                        Ok(_) => {
                            save_config(config.clone()).expect("failed to save config");
                            self.dispatch(GlimEvent::UpdateConfig(config));
                            self.dispatch(GlimEvent::CloseConfig);
                        }
                        Err(e) => {
                            self.dispatch(GlimEvent::DisplayAlert(e.to_string()));
                        }
                    }
                }
            },

            _ => {}
        }
    }

    pub fn process_timers(&mut self) -> u32 {
        let now = std::time::Instant::now();
        let elapsed = now - self.last_tick;
        self.last_tick = now;

        // do nothing with elapsed time for now;
        // and consider moving to UiState

        elapsed.as_millis() as u32
    }

    pub fn project(&self, id: ProjectId) -> &Project {
        self.project_store.find(id).expect("project not found")
    }

    pub fn projects(&self) -> &[Project] {
        self.project_store.projects()
    }

    pub fn logs(&self) -> Vec<(DateTime<Local>, &str)> {
        self.logs_store.logs()
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn last_frame_time(&self) -> u32 {
        self.last_tick.elapsed().as_millis() as u32
    }
}

impl UiState {
    pub fn new() -> Self {
        Self {
            show_internal_logs: false,
            use_256_colors: false,
        }
    }

    pub fn apply(&mut self, event: &GlimEvent) {
        match event {
            GlimEvent::ToggleInternalLogs => self.show_internal_logs = !self.show_internal_logs,
            GlimEvent::ToggleColorDepth   => self.use_256_colors = !self.use_256_colors,
            _ => ()
        }
    }
}

impl Dispatcher for GlimApp {
    fn dispatch(&self, event: GlimEvent) {
        self.sender.send(event).unwrap_or(());
    }
}

pub fn modulo(a: u32, b: u32) -> u32 {
    if b == 0 { return 0; }

    let a = a as i32;
    let b = b as i32;
    ((a % b) + b) as u32 % b as u32
}

pub trait Modulo {
    fn modulo(self, b: Self) -> Self;
}

impl Modulo for i32 {
    fn modulo(self, b: i32) -> i32 {
        if b == 0 { return 0; }

        ((self % b) + b) % b
    }
}

impl Modulo for u32 {
    fn modulo(self, b: u32) -> u32 {
        if b == 0 { return 0; }

        (self as i32).modulo(b as i32) as u32
    }
}

impl Modulo for isize {
    fn modulo(self, b: isize) -> isize {
        if b == 0 { return 0; }

        ((self % b) + b) % b
    }
}

impl Modulo for usize {
    fn modulo(self, b: usize) -> usize {
        if b == 0 { return 0; }

        (self as isize).modulo(b as isize) as usize
    }
}