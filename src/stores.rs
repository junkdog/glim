use std::{collections::HashMap, sync::mpsc::Sender};

use chrono::{DateTime, Utc};
use itertools::Itertools;
use tracing::{debug, info, instrument, warn};

use crate::{
    dispatcher::Dispatcher,
    domain::{Job, Pipeline, Project},
    event::GlimEvent,
    id::ProjectId,
};

pub struct ProjectStore {
    sender: Sender<GlimEvent>,
    projects: Vec<Project>,
    project_id_lookup: HashMap<ProjectId, usize>,
    sorted: Vec<Project>, // todo: ref projects
}

impl ProjectStore {
    pub fn new(sender: Sender<GlimEvent>) -> Self {
        Self {
            sender,
            projects: Vec::new(),
            // pipelines: Vec::new(),
            project_id_lookup: HashMap::new(),
            sorted: Vec::new(),
        }
    }

    #[instrument(skip(self, event), fields(event_type = ?std::mem::discriminant(event)))]
    pub fn apply(&mut self, event: &GlimEvent) {
        match event {
            // requests jobs for pipelines that have not been loaded yet
            GlimEvent::ProjectDetailsOpen(id) => {
                debug!(project_id = %id, "Opening project details and requesting missing jobs");
                let project = self.find(*id).unwrap();
                project
                    .recent_pipelines()
                    .into_iter()
                    .filter(|p| p.jobs.is_none())
                    .for_each(|p| self.dispatch(GlimEvent::JobsFetch(project.id, p.id)));
            },

            // updates the projects in the store
            GlimEvent::ProjectsLoaded(projects) => {
                debug!(project_count = projects.len(), "Processing received projects");
                let first_projects = self.sorted.is_empty();
                projects
                    .iter()
                    .map(|p| Project::from(p.clone()))
                    .for_each(|p| {
                        let project = p.clone();
                        self.sync_project(p);
                        let sender = self.sender.clone();
                        sender.dispatch(GlimEvent::ProjectUpdated(Box::new(project)))
                    });

                self.sorted = self.projects_sorted_by_last_activity();
                if first_projects {
                    self.dispatch(GlimEvent::ProjectSelected(self.sorted.first().unwrap().id));
                }
            },

            // updates the pipelines for a project
            GlimEvent::PipelinesLoaded(pipelines) => {
                let project_id = pipelines[0].project_id;
                debug!(project_id = %project_id, pipeline_count = pipelines.len(), "Processing received pipelines");
                let sender = self.sender.clone();

                if let Some(project) = self.find_mut(project_id) {
                    let pipelines: Vec<Pipeline> = pipelines
                        .iter()
                        .map(|p| Pipeline::from(p.clone()))
                        .collect();

                    pipelines
                        .iter()
                        .filter(|&p| p.status.is_active() || p.has_active_jobs())
                        .for_each(|p| sender.dispatch(GlimEvent::JobsFetch(project_id, p.id)));

                    project.update_pipelines(pipelines);
                    sender.dispatch(GlimEvent::ProjectUpdated(Box::new(project.clone())))
                }

                self.sorted = self.projects_sorted_by_last_activity();
            },

            GlimEvent::JobsLoaded(project_id, pipeline_id, job_dtos) => {
                debug!(project_id = %project_id, pipeline_id = %pipeline_id, job_count = job_dtos.len(), "Processing received jobs");
                let jobs: Vec<Job> = job_dtos
                    .iter()
                    .map(|j| Job::from(j.clone()))
                    .collect();

                let sender = self.sender.clone();
                if let Some(project) = self.find_mut(*project_id) {
                    project.update_jobs(*pipeline_id, jobs);
                    // todo: ugly, fix
                    project.update_commit(
                        *pipeline_id,
                        job_dtos
                            .first()
                            .map(|j| j.commit.clone().into())
                            .unwrap(),
                    );
                    sender.dispatch(GlimEvent::ProjectUpdated(Box::new(project.clone())))
                }

                self.sorted = self.projects_sorted_by_last_activity();
            },

            // requests pipelines for a project if they are not already loaded
            GlimEvent::ProjectSelected(id) => {
                debug!(project_id = %id, "Project selected");
                let mut request_pipelines = false;
                if let Some(project) = self.find_mut(*id) {
                    if project.pipelines.is_none() {
                        project.pipelines = Some(Vec::new());
                        request_pipelines = true;
                    }
                };

                if request_pipelines {
                    self.dispatch(GlimEvent::PipelinesFetch(*id));
                };
            },
            _ => {},
        }
    }

    fn projects_sorted_by_last_activity(&mut self) -> Vec<Project> {
        self.projects
            .iter()
            .sorted_by(|a, b| b.last_activity().cmp(&a.last_activity()))
            .cloned()
            .collect()
    }

    pub fn find(&self, id: ProjectId) -> Option<&Project> {
        self.project_idx(id)
            .map(|idx| &self.projects[idx])
    }

    pub fn sorted_projects(&self) -> &[Project] {
        &self.sorted
    }

    fn find_mut(&mut self, id: ProjectId) -> Option<&mut Project> {
        self.project_idx(id)
            .map(|idx| &mut self.projects[idx])
    }

    fn project_idx(&self, id: ProjectId) -> Option<usize> {
        self.project_id_lookup.get(&id).copied()
    }

    #[instrument(skip(self, project), fields(project_id = %project.id, project_path = %project.path))]
    fn sync_project(&mut self, mut project: Project) {
        let sender = self.sender.clone();
        match self.find_mut(project.id) {
            Some(existing_entry) => {
                sender.dispatch(GlimEvent::PipelinesFetch(project.id));
                existing_entry.update_project(project.clone())
            },
            None => {
                self.project_id_lookup
                    .insert(project.id, self.projects.len());
                if !is_older_than_7d(project.last_activity()) {
                    sender.dispatch(GlimEvent::PipelinesFetch(project.id));
                    project.pipelines = Some(Vec::new());
                }
                self.projects.push(project);
            },
        }
    }
}

fn is_older_than_7d(date: DateTime<Utc>) -> bool {
    Utc::now().signed_duration_since(date).num_days() > 7
}

#[instrument(skip(event), fields(event_type = ?std::mem::discriminant(event)))]
pub fn log_event(event: &GlimEvent) {
    match event {
        GlimEvent::ProjectsFetch => info!("Requesting all projects from GitLab"),
        GlimEvent::ProjectsLoaded(projects) => {
            info!(count = projects.len(), "Received projects from GitLab API")
        },
        GlimEvent::ProjectFetch(id) => debug!(project_id = %id, "Refreshing project"),
        GlimEvent::JobsActiveFetch => debug!("Requesting active pipelines for all projects"),
        GlimEvent::PipelinesFetch(id) => {
            debug!(project_id = %id, "Requesting pipelines for project")
        },
        GlimEvent::JobsFetch(project_id, pipeline_id) => {
            debug!(project_id = %project_id, pipeline_id = %pipeline_id, "Requesting jobs")
        },
        GlimEvent::PipelinesLoaded(pipelines) => {
            let project_id = pipelines.first().map(|p| p.project_id);
            debug!(count = pipelines.len(), project_id = ?project_id, "Received pipelines from GitLab API")
        },
        GlimEvent::JobsLoaded(project_id, pipeline_id, jobs) => {
            debug!(project_id = %project_id, pipeline_id = %pipeline_id, count = jobs.len(), "Received jobs")
        },
        GlimEvent::ProjectDetailsOpen(id) => debug!(project_id = %id, "Opening project details"),
        GlimEvent::ProjectDetailsClose => debug!("Closing project details popup"),
        GlimEvent::PipelineActionsOpen(project_id, pipeline_id) => {
            debug!(project_id = %project_id, pipeline_id = %pipeline_id, "Opening pipeline actions")
        },
        GlimEvent::ProjectSelected(id) => debug!(project_id = %id, "Selected project"),
        GlimEvent::PipelineSelected(id) => debug!(pipeline_id = %id, "Selected pipeline"),
        GlimEvent::ProjectOpenUrl(id) => info!(project_id = %id, "Opening project in browser"),
        GlimEvent::PipelineOpenUrl(project_id, pipeline_id) => {
            info!(project_id = %project_id, pipeline_id = %pipeline_id, "Opening pipeline in browser")
        },
        GlimEvent::JobOpenUrl(project_id, pipeline_id, job_id) => {
            info!(project_id = %project_id, pipeline_id = %pipeline_id, job_id = %job_id, "Opening job in browser")
        },
        GlimEvent::JobLogFetch(project_id, pipeline_id) => {
            info!(project_id = %project_id, pipeline_id = %pipeline_id, "Downloading job error log")
        },
        GlimEvent::JobLogDownloaded(project_id, job_id, log_content) => {
            info!(
                project_id = %project_id,
                job_id = %job_id,
                content_length = log_content.len(),
                "Job log downloaded successfully"
            )
        },
        GlimEvent::ConfigOpen => debug!("Displaying configuration"),
        GlimEvent::ConfigApply => info!("Applying new configuration"),
        GlimEvent::ConfigUpdate(_) => debug!("Updating configuration"),
        GlimEvent::ApplyTemporaryFilter(filter) => {
            debug!(filter = ?filter, "Applying temporary filter")
        },
        GlimEvent::FilterClear => info!("Clearing project filter"),
        GlimEvent::FilterMenuClose => debug!("Closing filter input"),
        GlimEvent::AppExit => info!("Application shutting down"),
        GlimEvent::AppError(err) => {
            warn!(error = %err, error_type = ?std::mem::discriminant(err), "Application error occurred")
        },
        GlimEvent::LogEntry(_msg) => {}, // Don't log LogEntry events to prevent infinite loop
        _ => {},                         // Don't log every event
    }
}

impl Dispatcher for ProjectStore {
    fn dispatch(&self, event: GlimEvent) {
        self.sender.send(event).unwrap();
    }
}
