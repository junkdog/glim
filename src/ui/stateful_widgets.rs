use crate::dispatcher::Dispatcher;
use crate::domain::Project;
use crate::effects::{
    default_glitch_effect, fade_in_projects_table, make_glitch_effect, project_details_close_effect,
};
use crate::event::GlimEvent;
use crate::glim_app::{GlimApp, GlimConfig, Modulo};
use crate::id::PipelineId;
use crate::ui::popup::{ConfigPopupState, PipelineActionsPopupState, ProjectDetailsPopupState};
use crate::ui::widget::NotificationState;
use compact_str::CompactString;
use ratatui::widgets::{ListState, TableState};
use std::sync::mpsc::Sender;
use tachyonfx::{Duration, Effect};

pub struct StatefulWidgets {
    pub last_frame: Duration,
    pub sender: Sender<GlimEvent>,
    pub project_table_state: TableState,
    pub logs_state: ListState,
    pub config_popup_state: Option<ConfigPopupState>,
    pub table_fade_in: Option<Effect>,
    pub project_details: Option<ProjectDetailsPopupState>,
    pub pipeline_actions: Option<PipelineActionsPopupState>,
    pub shader_pipeline: Option<Effect>,
    pub notice: Option<NotificationState>,
    pub filter_input_active: bool,
    pub filter_input_text: CompactString,
    pub temporary_filter: Option<CompactString>,
    current_filtered_indices: Vec<usize>,
    glitch_override: Option<Effect>,
    glitch: Effect,
}

impl StatefulWidgets {
    pub fn new(sender: Sender<GlimEvent>) -> Self {
        Self {
            last_frame: Duration::default(),
            sender,
            project_table_state: TableState::default().with_selected(0),
            logs_state: ListState::default().with_selected(Some(0)),
            table_fade_in: None,
            config_popup_state: None,
            project_details: None,
            pipeline_actions: None,
            shader_pipeline: None,
            glitch_override: None,
            notice: None,
            filter_input_active: false,
            filter_input_text: CompactString::default(),
            temporary_filter: None,
            current_filtered_indices: Vec::new(),
            glitch: default_glitch_effect(),
        }
    }

    pub fn apply(&mut self, app: &GlimApp, event: &GlimEvent) {
        match event {
            GlimEvent::GlitchOverride(g) => self.glitch_override = make_glitch_effect(*g),

            GlimEvent::SelectNextProject => self.handle_project_selection(1, app),
            GlimEvent::SelectPreviousProject => self.handle_project_selection(-1, app),

            GlimEvent::ReceivedProjects(_) => self.fade_in_projects_table(),

            GlimEvent::OpenProjectDetails(id) => {
                self.open_project_details(app.project(*id).clone(), app.sender.clone())
            }
            GlimEvent::CloseProjectDetails => {
                self.project_details = {
                    self.shader_pipeline = Some(project_details_close_effect());
                    None
                }
            }
            GlimEvent::ProjectUpdated(p) => self.refresh_project_details(p),

            GlimEvent::ClosePipelineActions => self.close_pipeline_actions(),
            GlimEvent::OpenPipelineActions(project_id, pipeline_id) => {
                let project = app.project(*project_id);
                self.open_pipeline_actions(project, *pipeline_id);
            }

            GlimEvent::DisplayConfig => self.open_config(app.load_config().unwrap_or_default()),
            GlimEvent::CloseConfig => self.config_popup_state = None,

            GlimEvent::ShowFilterMenu => self.show_filter_input(),
            GlimEvent::CloseFilter => self.close_filter_input(),
            GlimEvent::FilterInputChar(c) => self.add_filter_char(c),
            GlimEvent::FilterInputBackspace => self.remove_filter_char(),
            GlimEvent::ClearFilter => self.clear_filter(),
            GlimEvent::ApplyTemporaryFilter(filter) => self.apply_temporary_filter(filter.clone()),

            _ => (),
        }
    }

    fn fade_in_projects_table(&mut self) {
        self.table_fade_in = Some(fade_in_projects_table());
    }

    fn refresh_project_details(&mut self, project: &Project) {
        let requires_refresh = self
            .project_details
            .as_ref()
            .map_or(false, |pd| pd.project.id == project.id);

        if requires_refresh {
            let existing = self.project_details.take().unwrap();
            self.project_details = Some(existing.with_project(project.clone()));
        }
    }

    fn open_project_details(&mut self, project: Project, sender: Sender<GlimEvent>) {
        project
            .recent_pipelines()
            .first()
            .map(|p| sender.dispatch(GlimEvent::SelectedPipeline(p.id)))
            .unwrap_or(());

        self.project_details = Some(ProjectDetailsPopupState::new(project));
    }

    fn open_config(&mut self, config: GlimConfig) {
        self.config_popup_state = Some(ConfigPopupState::new(config));
    }

    fn open_pipeline_actions(&mut self, project: &Project, pipeline_id: PipelineId) {
        let failed_job = project.pipeline(pipeline_id).and_then(|p| p.failed_job());

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

        self.pipeline_actions = Some(PipelineActionsPopupState::new(
            actions,
            project.id,
            pipeline_id,
        ));
    }

    fn close_pipeline_actions(&mut self) {
        self.pipeline_actions = None;
    }

    fn handle_project_selection(&mut self, direction: i32, app: &GlimApp) {
        let all_projects = app.projects();
        let filtered_count = if self.current_filtered_indices.is_empty() {
            all_projects.len()
        } else {
            self.current_filtered_indices.len()
        };

        if filtered_count == 0 {
            return;
        }

        if let Some(current) = self.project_table_state.selected() {
            let new_index = match direction {
                1 => current.saturating_add(1),
                -1 => current.saturating_sub(1),
                n => panic!("invalid direction: {n}"),
            }
            .min(filtered_count.saturating_sub(1));

            self.project_table_state.select(Some(new_index));

            // Get the actual project from the filtered list
            let project_index = if self.current_filtered_indices.is_empty() {
                new_index
            } else {
                self.current_filtered_indices[new_index]
            };

            let project = &all_projects[project_index];
            app.dispatch(GlimEvent::SelectedProject(project.id));
        } else {
            self.project_table_state.select(Some(0));
        }
    }

    pub fn handle_pipeline_selection(&mut self, direction: i32) {
        if self.project_details.is_none() {
            return;
        }
        let pd = self.project_details.as_mut().unwrap();

        if let Some(current) = pd.pipelines_table_state.selected() {
            let pipelines = pd.project.recent_pipelines();

            let new_index = (current as i32 + direction).modulo(pipelines.len() as i32) as usize;

            if pipelines.is_empty() {
                pd.pipelines_table_state.select(None);
            } else {
                pd.pipelines_table_state.select(Some(new_index));
                let pipeline = &pipelines[new_index];
                self.sender
                    .dispatch(GlimEvent::SelectedPipeline(pipeline.id));
            }
        }
    }

    pub fn handle_pipeline_action_selection(&mut self, direction: i32) {
        if self.pipeline_actions.is_none() {
            return;
        }

        let pipelines = self.pipeline_actions.as_mut().unwrap();
        if let Some(current) = pipelines.list_state.selected() {
            let new_index = (current as i32 + direction).modulo(pipelines.actions.len() as i32);

            pipelines.list_state.select(Some(new_index as usize));
        }
    }

    fn show_filter_input(&mut self) {
        self.filter_input_active = true;
        // Start with the current temporary filter or empty string
        self.filter_input_text = self.temporary_filter.clone().unwrap_or_default();
    }

    fn close_filter_input(&mut self) {
        self.filter_input_active = false;
    }

    fn add_filter_char(&mut self, c: &str) {
        if self.filter_input_active {
            self.filter_input_text.push_str(c);
            // Apply filter immediately as user types
            let filter = if self.filter_input_text.is_empty() {
                None
            } else {
                Some(self.filter_input_text.clone())
            };
            self.temporary_filter = filter;
            self.project_table_state.select(Some(0)); // Reset selection to first item
        }
    }

    fn remove_filter_char(&mut self) {
        if self.filter_input_active {
            self.filter_input_text.pop();
            // Apply filter immediately as user deletes characters
            let filter = if self.filter_input_text.is_empty() {
                None
            } else {
                Some(self.filter_input_text.clone())
            };
            self.temporary_filter = filter;
            self.project_table_state.select(Some(0)); // Reset selection to first item
        }
    }

    fn apply_temporary_filter(&mut self, filter: Option<CompactString>) {
        self.temporary_filter = filter;
        self.filter_input_active = false;
        // Reset table selection when filter changes
        self.project_table_state.select(Some(0));
    }

    fn clear_filter(&mut self) {
        self.filter_input_text.clear();
        self.filter_input_active = false;
        self.temporary_filter = None;
        self.sender.dispatch(GlimEvent::ApplyTemporaryFilter(None));
    }

    pub fn effective_filter(&self, config_filter: &Option<CompactString>) -> Option<CompactString> {
        // Temporary filter takes precedence over config filter
        self.temporary_filter
            .clone()
            .or_else(|| config_filter.clone())
    }

    pub fn update_filtered_indices(&mut self, indices: Vec<usize>) {
        self.current_filtered_indices = indices;
    }

    pub fn glitch(&mut self) -> &mut Effect {
        match self.glitch_override.as_mut() {
            Some(g) => g,
            None => &mut self.glitch,
        }
    }
}
