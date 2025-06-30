use crate::dispatcher::Dispatcher;
use crate::domain::{Job, Pipeline, Project};
use crate::event::GlimEvent;
use crate::id::ProjectId;
use chrono::{DateTime, Local, Utc};
use itertools::Itertools;
use std::collections::HashMap;
use std::sync::mpsc::Sender;

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

pub struct InternalLogsStore {
    logs: Vec<(DateTime<Local>, String)>,
}

impl InternalLogsStore {
    pub fn new() -> Self {
        Self { logs: Vec::new() }
    }

    pub fn apply(&mut self, event: &GlimEvent) {
        if let Some(log) = match event {
            GlimEvent::Log(s) => Some(s.to_owned()),
            GlimEvent::ToggleColorDepth => Some("toggling color depth".to_string()),
            GlimEvent::Shutdown => Some("shutting down...".to_string()),
            GlimEvent::RequestProject(id) => Some(format!("refresh project_id={id}")),
            GlimEvent::RequestProjects => {
                Some("request all projects since last update".to_string())
            }
            GlimEvent::RequestActiveJobs => {
                Some("request active pipelines for all projects".to_string())
            }
            GlimEvent::RequestPipelines(id) => {
                Some(format!("request pipelines for project_id={id}"))
            }
            GlimEvent::RequestJobs(project_id, pipeline_id) => Some(format!(
                "request jobs for project_id={project_id} pipeline_id={pipeline_id}"
            )),
            GlimEvent::ReceivedProjects(projects) => {
                Some(format!("received {:?} projects", projects.len()))
            }
            GlimEvent::ReceivedPipelines(pipelines) => {
                Some(format!("received {:?} pipelines", pipelines.len()))
            }
            GlimEvent::ReceivedJobs(project_id, _, jobs) => Some(format!(
                "received {:?} jobs for project_id={project_id}",
                jobs.len()
            )),
            GlimEvent::OpenProjectDetails(id) => Some(format!("showing project_id={id} details")),
            GlimEvent::CloseProjectDetails => Some("closing project details popup".to_string()),
            GlimEvent::OpenPipelineActions(id, pipeline_id) => Some(format!(
                "showing pipeline {pipeline_id}'s actions for project_id={id}"
            )),
            GlimEvent::Error(s) => Some(s.to_string()),
            GlimEvent::SelectedProject(id) => Some(format!("selected project_id={id}")),
            GlimEvent::SelectedPipeline(id) => Some(format!("selected pipeline_id={id}")),
            GlimEvent::BrowseToProject(id) => Some(format!("open project_id={id} in browser")),
            GlimEvent::BrowseToPipeline(_, id) => Some(format!("open pipeline_id={id} in browser")),
            GlimEvent::BrowseToJob(_, _, job_id) => {
                Some(format!("open job_id={job_id}  in browser"))
            }
            GlimEvent::DownloadErrorLog(_, id) => {
                Some(format!("download job log for failed pipeline_id={id}"))
            }
            GlimEvent::ShowFilterMenu => Some("showing filter menu".to_string()),
            GlimEvent::ShowSortMenu => Some("showing sort menu".to_string()),
            GlimEvent::JobLogDownloaded(_, id, _) => {
                Some(format!("downloaded log for job_id={id}"))
            }
            GlimEvent::DisplayConfig => Some("display config".to_string()),
            GlimEvent::ApplyConfiguration => Some("applying new configuration".to_string()),
            GlimEvent::UpdateConfig(_) => Some("updating configuration".to_string()),
            GlimEvent::CloseConfig => None,
            GlimEvent::ClosePipelineActions => None,
            GlimEvent::GlitchOverride(_) => None,
            GlimEvent::Tick => None,
            GlimEvent::ProjectUpdated(_) => None,
            GlimEvent::Key(_) => None,
            GlimEvent::SelectNextProject => None,
            GlimEvent::ShowLastNotification => None,
            GlimEvent::SelectPreviousProject => None,
            GlimEvent::ToggleInternalLogs => None,
            GlimEvent::CloseFilter => Some("closing filter input".to_string()),
            GlimEvent::FilterInputChar(_) => None,
            GlimEvent::FilterInputBackspace => None,
            GlimEvent::ApplyFilter(filter) => Some(format!("applying filter: '{}'", filter)),
            GlimEvent::ApplyTemporaryFilter(filter) => {
                Some(format!("applying temporary filter: '{:?}'", filter))
            }
            GlimEvent::ClearFilter => Some("clearing filter".to_string()),
        } {
            self.logs.push((Local::now(), log));
        }

        if self.logs.len() > 200 {
            self.logs = self.logs.iter().dropping(150).cloned().collect();
        }
    }

    pub fn logs(&self) -> Vec<(DateTime<Local>, &str)> {
        self.logs.iter().map(|(dt, s)| (*dt, s.as_str())).collect()
    }
}

impl Dispatcher for ProjectStore {
    fn dispatch(&self, event: GlimEvent) {
        self.sender.send(event).unwrap();
    }
}
