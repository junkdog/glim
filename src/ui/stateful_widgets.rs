use std::sync::mpsc::Sender;
use ratatui::widgets::{ListState, TableState};
use tachyonfx::{fx, Duration, Effect, Interpolation, IntoEffect};
use tachyonfx::fx::{parallel, Direction, Glitch};
use crate::dispatcher::Dispatcher;
use crate::domain::Project;
use crate::event::{GlimEvent, GlitchState};
use crate::glim_app::{GlimApp, GlimConfig, Modulo};
use crate::gruvbox::Gruvbox::{Dark0Hard, Dark3};
use crate::id::PipelineId;
use crate::ui::popup::{ConfigPopupState, PipelineActionsPopupState, ProjectDetailsPopupState};
use crate::ui::widget::NotificationState;

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
            glitch: Glitch::builder()
                .action_ms(100..500)
                .action_start_delay_ms(0..2000)
                .cell_glitch_ratio(0.0015)
                .build()
                .into_effect()
        }
    }

    pub fn apply(
        &mut self,
        app: &GlimApp,
        event: &GlimEvent
    ) {
        match event {
            GlimEvent::GlitchOverride(g)            => self.glitch_override = make_glitch_effect(*g),

            GlimEvent::SelectNextProject            => self.handle_project_selection(1, app),
            GlimEvent::SelectPreviousProject        => self.handle_project_selection(-1, app),

            GlimEvent::ReceivedProjects(_)          => self.fade_in_projects_table(),

            GlimEvent::OpenProjectDetails(id)       => self.open_project_details(app.project(*id).clone(), app.sender.clone()),
            GlimEvent::CloseProjectDetails          => self.project_details = {
                let fade_in = fx::fade_from(Dark3, Dark0Hard, (300, Interpolation::CircIn));
                self.shader_pipeline = Some(fade_in);

                None
            },
            GlimEvent::ProjectUpdated(p)            => self.refresh_project_details(p),

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
        let effect = parallel(&[
            fx::coalesce(550),
            fx::sweep_in(Direction::LeftToRight, 50, 0, Dark0Hard, (450, Interpolation::QuadIn))
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

    fn close_pipeline_actions(&mut self) {
        self.pipeline_actions = None;
    }

    fn handle_project_selection(&mut self, direction: i32, app: &GlimApp) {
        let projects = app.projects();
        if projects.is_empty() { return; }

        if let Some(current) = self.project_table_state.selected() {
            let new_index = match direction {
                1  => current.saturating_add(1),
                -1 => current.saturating_sub(1),
                n  => panic!("invalid direction: {n}")
            }.min(projects.len().saturating_sub(1));

            self.project_table_state.select(Some(new_index));
            let project = &projects[new_index];
            app.dispatch(GlimEvent::SelectedProject(project.id));
        } else {
            self.project_table_state.select(Some(0));
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


fn make_glitch_effect(glitch_state: GlitchState) -> Option<Effect> {
    match glitch_state {
        GlitchState::Inactive => None,
        GlitchState::Active => Some(Glitch::builder()
            .action_ms(100..200)
            .action_start_delay_ms(0..500)
            .cell_glitch_ratio(0.05)
            .build()
            .into_effect())
    }
}
