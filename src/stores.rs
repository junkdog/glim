use crate::dispatcher::Dispatcher;
use crate::domain::{Job, Pipeline, Project};
use crate::event::GlimEvent;
use crate::id::ProjectId;
use chrono::{DateTime, Utc};
use itertools::Itertools;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use tracing::{debug, info, warn};

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

    pub fn apply(&mut self, event: &GlimEvent) {
        match event {
            // requests jobs for pipelines that have not been loaded yet
            GlimEvent::OpenProjectDetails(id) => {
                let project = self.find(*id).unwrap();
                project
                    .recent_pipelines()
                    .into_iter()
                    .filter(|p| p.jobs.is_none())
                    .for_each(|p| self.dispatch(GlimEvent::RequestJobs(project.id, p.id)));
            }

            // updates the projects in the store
            GlimEvent::ReceivedProjects(projects) => {
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
                    self.dispatch(GlimEvent::SelectedProject(self.sorted.first().unwrap().id));
                }
            }

            // updates the pipelines for a project
            GlimEvent::ReceivedPipelines(pipelines) => {
                let project_id = pipelines[0].project_id;
                let sender = self.sender.clone();

                if let Some(project) = self.find_mut(project_id) {
                    let pipelines: Vec<Pipeline> = pipelines
                        .iter()
                        .map(|p| Pipeline::from(p.clone()))
                        .collect();

                    pipelines
                        .iter()
                        .filter(|&p| p.status.is_active() || p.has_active_jobs())
                        .for_each(|p| sender.dispatch(GlimEvent::RequestJobs(project_id, p.id)));

                    project.update_pipelines(pipelines);
                    sender.dispatch(GlimEvent::ProjectUpdated(Box::new(project.clone())))
                }

                self.sorted = self.projects_sorted_by_last_activity();
            }

            GlimEvent::ReceivedJobs(project_id, pipeline_id, job_dtos) => {
                let jobs: Vec<Job> = job_dtos.iter().map(|j| Job::from(j.clone())).collect();

                let sender = self.sender.clone();
                if let Some(project) = self.find_mut(*project_id) {
                    project.update_jobs(*pipeline_id, jobs);
                    // todo: ugly, fix
                    project.update_commit(
                        *pipeline_id,
                        job_dtos.first().map(|j| j.commit.clone().into()).unwrap(),
                    );
                    sender.dispatch(GlimEvent::ProjectUpdated(Box::new(project.clone())))
                }

                self.sorted = self.projects_sorted_by_last_activity();
            }

            // requests pipelines for a project if they are not already loaded
            GlimEvent::SelectedProject(id) => {
                let mut request_pipelines = false;
                if let Some(project) = self.find_mut(*id) {
                    if project.pipelines.is_none() {
                        project.pipelines = Some(Vec::new());
                        request_pipelines = true;
                    }
                };

                if request_pipelines {
                    self.dispatch(GlimEvent::RequestPipelines(*id));
                };
            }
            _ => {}
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
        self.project_idx(id).map(|idx| &self.projects[idx])
    }

    pub fn projects(&self) -> &[Project] {
        &self.sorted
    }

    fn find_mut(&mut self, id: ProjectId) -> Option<&mut Project> {
        self.project_idx(id).map(|idx| &mut self.projects[idx])
    }

    fn project_idx(&self, id: ProjectId) -> Option<usize> {
        self.project_id_lookup.get(&id).copied()
    }

    fn sync_project(&mut self, mut project: Project) {
        let sender = self.sender.clone();
        match self.find_mut(project.id) {
            Some(existing_entry) => {
                sender.dispatch(GlimEvent::RequestPipelines(project.id));
                existing_entry.update_project(project.clone())
            }
            None => {
                self.project_id_lookup
                    .insert(project.id, self.projects.len());
                if !is_older_than_7d(project.last_activity()) {
                    sender.dispatch(GlimEvent::RequestPipelines(project.id));
                    project.pipelines = Some(Vec::new());
                }
                self.projects.push(project);
            }
        }
    }
}

fn is_older_than_7d(date: DateTime<Utc>) -> bool {
    Utc::now().signed_duration_since(date).num_days() > 7
}

pub fn log_event(event: &GlimEvent) {
    match event {
        GlimEvent::RequestProjects => info!("Requesting all projects from GitLab"),
        GlimEvent::ReceivedProjects(projects) => {
            info!(count = projects.len(), "Received projects from GitLab API")
        }
        GlimEvent::RequestProject(id) => debug!(project_id = %id, "Refreshing project"),
        GlimEvent::RequestActiveJobs => debug!("Requesting active pipelines for all projects"),
        GlimEvent::RequestPipelines(id) => {
            debug!(project_id = %id, "Requesting pipelines for project")
        }
        GlimEvent::RequestJobs(project_id, pipeline_id) => {
            debug!(project_id = %project_id, pipeline_id = %pipeline_id, "Requesting jobs")
        }
        GlimEvent::ReceivedPipelines(pipelines) => {
            debug!(count = pipelines.len(), "Received pipelines from GitLab API")
        }
        GlimEvent::ReceivedJobs(project_id, pipeline_id, jobs) => {
            debug!(project_id = %project_id, pipeline_id = %pipeline_id, count = jobs.len(), "Received jobs")
        }
        GlimEvent::OpenProjectDetails(id) => debug!(project_id = %id, "Opening project details"),
        GlimEvent::CloseProjectDetails => debug!("Closing project details popup"),
        GlimEvent::OpenPipelineActions(project_id, pipeline_id) => {
            debug!(project_id = %project_id, pipeline_id = %pipeline_id, "Opening pipeline actions")
        }
        GlimEvent::SelectedProject(id) => debug!(project_id = %id, "Selected project"),
        GlimEvent::SelectedPipeline(id) => debug!(pipeline_id = %id, "Selected pipeline"),
        GlimEvent::BrowseToProject(id) => info!(project_id = %id, "Opening project in browser"),
        GlimEvent::BrowseToPipeline(project_id, pipeline_id) => {
            info!(project_id = %project_id, pipeline_id = %pipeline_id, "Opening pipeline in browser")
        }
        GlimEvent::BrowseToJob(project_id, pipeline_id, job_id) => {
            info!(project_id = %project_id, pipeline_id = %pipeline_id, job_id = %job_id, "Opening job in browser")
        }
        GlimEvent::DownloadErrorLog(project_id, pipeline_id) => {
            info!(project_id = %project_id, pipeline_id = %pipeline_id, "Downloading job error log")
        }
        GlimEvent::JobLogDownloaded(project_id, job_id, log_content) => {
            info!(
                project_id = %project_id, 
                job_id = %job_id, 
                content_length = log_content.len(),
                "Job log downloaded successfully"
            )
        }
        GlimEvent::DisplayConfig => debug!("Displaying configuration"),
        GlimEvent::ApplyConfiguration => info!("Applying new configuration"),
        GlimEvent::UpdateConfig(_) => debug!("Updating configuration"),
        GlimEvent::ApplyFilter(filter) => info!(filter = %filter, "Applying project filter"),
        GlimEvent::ApplyTemporaryFilter(filter) => debug!(filter = ?filter, "Applying temporary filter"),
        GlimEvent::ClearFilter => info!("Clearing project filter"),
        GlimEvent::CloseFilter => debug!("Closing filter input"),
        GlimEvent::ToggleColorDepth => debug!("Toggling color depth"),
        GlimEvent::Shutdown => info!("Application shutting down"),
        GlimEvent::Error(err) => warn!(error = %err, "Application error occurred"),
        GlimEvent::Log(msg) => info!(message = %msg, "Application log message"),
        _ => {} // Don't log every event
    }
}

impl Dispatcher for ProjectStore {
    fn dispatch(&self, event: GlimEvent) {
        self.sender.send(event).unwrap();
    }
}
