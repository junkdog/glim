use std::sync::mpsc::Sender;

use compact_str::CompactString;
use ratatui::widgets::TableState;
use tachyonfx::{Duration, Effect};

use crate::{
    dispatcher::Dispatcher,
    domain::Project,
    effect_registry::{fade_in_projects_table, EffectRegistry},
    event::GlimEvent,
    glim_app::{GlimApp, GlimConfig, Modulo},
    id::PipelineId,
    ui::{
        popup::{ConfigPopupState, PipelineActionsPopupState, ProjectDetailsPopupState},
        widget::{NotificationState, RefRect},
    },
};

pub struct StatefulWidgets {
    pub last_frame: Duration,
    pub sender: Sender<GlimEvent>,
    pub project_table_state: TableState,
    pub config_popup_state: Option<ConfigPopupState>,
    pub table_fade_in: Option<Effect>,
    pub project_details: Option<ProjectDetailsPopupState>,
    pub pipeline_actions: Option<PipelineActionsPopupState>,
    pub notice: Option<NotificationState>,
    pub filter_input_active: bool,
    pub filter_input_text: CompactString,
    pub temporary_filter: Option<CompactString>,
    pub capture_screen_requested: bool,
    current_filtered_indices: Vec<usize>,
}

impl StatefulWidgets {
    pub fn new(sender: Sender<GlimEvent>) -> Self {
        Self {
            last_frame: Duration::default(),
            sender,
            project_table_state: TableState::default().with_selected(0),
            table_fade_in: None,
            config_popup_state: None,
            project_details: None,
            pipeline_actions: None,
            notice: None,
            filter_input_active: false,
            filter_input_text: CompactString::default(),
            temporary_filter: None,
            capture_screen_requested: false,
            current_filtered_indices: Vec::new(),
        }
    }

    pub fn apply(&mut self, app: &GlimApp, effects: &mut EffectRegistry, event: &GlimEvent) {
        match event {
            GlimEvent::ProjectNext => self.handle_project_selection(1, app),
            GlimEvent::ProjectPrevious => self.handle_project_selection(-1, app),

            GlimEvent::ProjectsLoaded(_) => self.fade_in_projects_table(),

            GlimEvent::ProjectDetailsOpen(id) => {
                let popup_area = RefRect::default();
                effects.register_project_details(popup_area.clone());
                self.open_project_details(app.project(*id).clone(), popup_area, app.sender())
            },
            GlimEvent::ProjectDetailsClose => self.project_details = None,
            GlimEvent::ProjectUpdated(p) => self.refresh_project_details(p),

            GlimEvent::PipelineActionsClose => self.close_pipeline_actions(),
            GlimEvent::PipelineActionsOpen(project_id, pipeline_id) => {
                let popup_area = RefRect::default();
                effects.register_pipeline_actions(popup_area.clone());
                let project = app.project(*project_id);
                self.open_pipeline_actions(project, *pipeline_id, popup_area);
            },

            GlimEvent::ConfigOpen => {
                let popup_area = RefRect::default();
                effects.register_config_popup(popup_area.clone());
                self.open_config(app.load_config().unwrap_or_default(), popup_area);
            },
            GlimEvent::ConfigClose => self.config_popup_state = None,

            GlimEvent::FilterMenuShow => self.show_filter_input(),
            GlimEvent::FilterMenuClose => self.close_filter_input(),
            GlimEvent::FilterInputChar(c) => self.add_filter_char(c),
            GlimEvent::FilterInputBackspace => self.remove_filter_char(),
            GlimEvent::FilterClear => self.clear_filter(),
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
            .is_some_and(|pd| pd.project.id == project.id);

        if requires_refresh {
            let existing = self.project_details.take().unwrap();
            self.project_details = Some(existing.with_project(project.clone()));
        }
    }

    fn open_project_details(
        &mut self,
        project: Project,
        area_tracker: RefRect,
        sender: Sender<GlimEvent>,
    ) {
        project
            .recent_pipelines()
            .first()
            .map(|p| sender.dispatch(GlimEvent::PipelineSelected(p.id)))
            .unwrap_or(());

        self.project_details = Some(ProjectDetailsPopupState::new(project, area_tracker));
    }

    fn open_config(&mut self, config: GlimConfig, popup_area: RefRect) {
        self.config_popup_state = Some(ConfigPopupState::new(config, popup_area));
    }

    fn open_pipeline_actions(
        &mut self,
        project: &Project,
        pipeline_id: PipelineId,
        popup_area: RefRect,
    ) {
        let failed_job = project
            .pipeline(pipeline_id)
            .and_then(|p| p.failed_job());

        let actions = if let Some(job) = failed_job {
            vec![
                GlimEvent::JobOpenUrl(project.id, pipeline_id, job.id),
                GlimEvent::PipelineOpenUrl(project.id, pipeline_id),
                GlimEvent::ProjectOpenUrl(project.id),
                GlimEvent::JobLogFetch(project.id, pipeline_id),
            ]
        } else {
            vec![
                GlimEvent::PipelineOpenUrl(project.id, pipeline_id),
                GlimEvent::ProjectOpenUrl(project.id),
            ]
        };

        self.pipeline_actions = Some(PipelineActionsPopupState::new(actions, popup_area));
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
            app.dispatch(GlimEvent::ProjectSelected(project.id));
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
                    .dispatch(GlimEvent::PipelineSelected(pipeline.id));
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

            pipelines
                .list_state
                .select(Some(new_index as usize));
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
        self.sender
            .dispatch(GlimEvent::ApplyTemporaryFilter(None));
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
}
