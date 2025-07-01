use std::fmt::Debug;
use std::sync::mpsc;
use std::thread;

use compact_str::CompactString;
use crate::dispatcher::Dispatcher;
use crate::domain::{JobDto, PipelineDto, Project, ProjectDto};
use crate::glim_app::GlimConfig;
use crate::id::{JobId, PipelineId, ProjectId};
use crate::result;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind};

#[derive(Debug, Clone)]
pub enum GlimEvent {
    Tick,
    Shutdown,
    Key(KeyEvent),
    Log(CompactString),
    GlitchOverride(GlitchState),
    CloseProjectDetails,
    OpenProjectDetails(ProjectId),
    OpenPipelineActions(ProjectId, PipelineId),
    ClosePipelineActions,
    RequestProject(ProjectId),
    RequestProjects,
    RequestJobs(ProjectId, PipelineId),
    RequestActiveJobs,
    RequestPipelines(ProjectId),
    ReceivedProjects(Vec<ProjectDto>),
    ReceivedPipelines(Vec<PipelineDto>),
    ReceivedJobs(ProjectId, PipelineId, Vec<JobDto>),
    SelectedProject(ProjectId),
    SelectedPipeline(PipelineId),
    Error(result::GlimError),
    SelectNextProject,
    SelectPreviousProject,
    ApplyConfiguration,
    UpdateConfig(GlimConfig),
    DisplayConfig,
    CloseConfig,
    BrowseToJob(ProjectId, PipelineId, JobId),
    BrowseToPipeline(ProjectId, PipelineId),
    BrowseToProject(ProjectId),
    DownloadErrorLog(ProjectId, PipelineId),
    JobLogDownloaded(ProjectId, JobId, CompactString),
    ProjectUpdated(Box<Project>),
    ShowLastNotification,
    ShowFilterMenu,
    CloseFilter,
    FilterInputChar(CompactString),
    FilterInputBackspace,
    ApplyFilter(CompactString),
    ApplyTemporaryFilter(Option<CompactString>),
    ClearFilter,
    ShowSortMenu,
    ToggleColorDepth,
}

#[derive(Debug, Clone, Copy)]
pub enum GlitchState {
    Active,
    Inactive,
}

#[derive(Debug)]
pub struct EventHandler {
    sender: mpsc::Sender<GlimEvent>,
    receiver: mpsc::Receiver<GlimEvent>,
    _handler: thread::JoinHandle<()>,
}

pub trait IntoGlimEvent {
    fn into_glim_event(self) -> GlimEvent;
}

impl EventHandler {
    pub fn new(tick_rate: std::time::Duration) -> Self {
        let (sender, receiver) = mpsc::channel();

        let handler = {
            let sender = sender.clone();
            thread::spawn(move || {
                let mut last_tick = std::time::Instant::now();
                loop {
                    let timeout = tick_rate
                        .checked_sub(last_tick.elapsed())
                        .unwrap_or(tick_rate);

                    if event::poll(timeout).expect("unable to poll for events") {
                        Self::apply_event(&sender);
                    }

                    if last_tick.elapsed() >= tick_rate {
                        sender.dispatch(GlimEvent::Tick);
                        last_tick = std::time::Instant::now();
                    }
                }
            })
        };

        Self {
            sender,
            receiver,
            _handler: handler,
        }
    }

    pub fn sender(&self) -> mpsc::Sender<GlimEvent> {
        self.sender.clone()
    }

    pub fn next(&self) -> Result<GlimEvent, mpsc::RecvError> {
        self.receiver.recv()
    }

    pub fn try_next(&self) -> Option<GlimEvent> {
        match self.receiver.try_recv() {
            Ok(e) => Some(e),
            Err(_) => None,
        }
    }

    fn apply_event(sender: &mpsc::Sender<GlimEvent>) {
        match event::read().expect("unable to read event") {
            CrosstermEvent::Key(e) if e.kind == KeyEventKind::Press => {
                sender.send(GlimEvent::Key(e))
            }

            _ => Ok(()),
        }
        .expect("failed to send event")
    }
}

impl From<Vec<ProjectDto>> for GlimEvent {
    fn from(projects: Vec<ProjectDto>) -> Self {
        GlimEvent::ReceivedProjects(projects)
    }
}

impl From<Vec<PipelineDto>> for GlimEvent {
    fn from(pipelines: Vec<PipelineDto>) -> Self {
        GlimEvent::ReceivedPipelines(pipelines)
    }
}

impl From<(ProjectId, PipelineId, Vec<JobDto>)> for GlimEvent {
    fn from(value: (ProjectId, PipelineId, Vec<JobDto>)) -> Self {
        let (project_id, pipeline_id, jobs) = value;
        GlimEvent::ReceivedJobs(project_id, pipeline_id, jobs)
    }
}

impl IntoGlimEvent for Vec<ProjectDto> {
    fn into_glim_event(self) -> GlimEvent {
        GlimEvent::ReceivedProjects(self)
    }
}

impl IntoGlimEvent for Vec<PipelineDto> {
    fn into_glim_event(self) -> GlimEvent {
        GlimEvent::ReceivedPipelines(self)
    }
}

impl IntoGlimEvent for (ProjectId, PipelineId, Vec<JobDto>) {
    fn into_glim_event(self) -> GlimEvent {
        let (project_id, pipeline_id, jobs) = self;
        GlimEvent::ReceivedJobs(project_id, pipeline_id, jobs)
    }
}
