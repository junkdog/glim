use std::{fmt::Debug, sync::mpsc, thread};

use compact_str::CompactString;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind};
use tracing::Level;

use crate::{
    dispatcher::Dispatcher,
    domain::{JobDto, PipelineDto, Project, ProjectDto},
    glim_app::GlimConfig,
    id::{JobId, PipelineId, ProjectId},
    result,
};

#[derive(Debug, Clone)]
pub enum GlimEvent {
    AppError(result::GlimError),
    AppExit,
    AppTick,
    ApplyTemporaryFilter(Option<CompactString>),
    ConfigApply,
    ConfigClose,
    ConfigOpen,
    ConfigUpdate(GlimConfig),
    FilterClear,
    FilterInputBackspace,
    FilterInputChar(CompactString),
    FilterMenuClose,
    FilterMenuShow,
    #[allow(dead_code)]
    GlitchOverride(GlitchState),
    InputKey(KeyEvent),
    JobLogDownloaded(ProjectId, JobId, CompactString),
    JobLogFetch(ProjectId, PipelineId),
    JobOpenUrl(ProjectId, PipelineId, JobId),
    JobsActiveFetch,
    JobsFetch(ProjectId, PipelineId),
    JobsLoaded(ProjectId, PipelineId, Vec<JobDto>),
    LogEntry(CompactString),
    LogLevelChanged(Level),
    NotificationDismiss,
    NotificationLast,
    PipelineActionsClose,
    PipelineActionsOpen(ProjectId, PipelineId),
    PipelineOpenUrl(ProjectId, PipelineId),
    PipelineSelected(PipelineId),
    PipelinesFetch(ProjectId),
    PipelinesLoaded(Vec<PipelineDto>),
    ProjectDetailsClose,
    ProjectDetailsOpen(ProjectId),
    #[allow(dead_code)]
    ProjectFetch(ProjectId),
    ProjectNext,
    ProjectOpenUrl(ProjectId),
    ProjectPrevious,
    ProjectSelected(ProjectId),
    ProjectUpdated(Box<Project>),
    ProjectsFetch,
    ProjectsLoaded(Vec<ProjectDto>),
    ScreenCapture,
    ScreenCaptureToClipboard(String),
}

impl GlimEvent {
    /// Get the variant name as a string slice (without "GlimEvent::" prefix)
    pub fn variant_name(&self) -> &'static str {
        match self {
            GlimEvent::AppError(_) => "AppError",
            GlimEvent::AppExit => "AppExit",
            GlimEvent::AppTick => "AppTick",
            GlimEvent::ApplyTemporaryFilter(_) => "ApplyTemporaryFilter",
            GlimEvent::ConfigApply => "ConfigApply",
            GlimEvent::ConfigClose => "ConfigClose",
            GlimEvent::ConfigOpen => "ConfigOpen",
            GlimEvent::ConfigUpdate(_) => "ConfigUpdate",
            GlimEvent::FilterClear => "FilterClear",
            GlimEvent::FilterInputBackspace => "FilterInputBackspace",
            GlimEvent::FilterInputChar(_) => "FilterInputChar",
            GlimEvent::FilterMenuClose => "FilterMenuClose",
            GlimEvent::FilterMenuShow => "FilterMenuShow",
            GlimEvent::GlitchOverride(_) => "GlitchOverride",
            GlimEvent::InputKey(_) => "InputKey",
            GlimEvent::JobLogDownloaded(_, _, _) => "JobLogDownloaded",
            GlimEvent::JobLogFetch(_, _) => "JobLogFetch",
            GlimEvent::JobOpenUrl(_, _, _) => "JobOpenUrl",
            GlimEvent::JobsActiveFetch => "JobsActiveFetch",
            GlimEvent::JobsFetch(_, _) => "JobsFetch",
            GlimEvent::JobsLoaded(_, _, _) => "JobsLoaded",
            GlimEvent::LogEntry(_) => "LogEntry",
            GlimEvent::LogLevelChanged(_) => "LogLevelChanged",
            GlimEvent::NotificationDismiss => "NotificationDismiss",
            GlimEvent::NotificationLast => "NotificationLast",
            GlimEvent::PipelineActionsClose => "PipelineActionsClose",
            GlimEvent::PipelineActionsOpen(_, _) => "PipelineActionsOpen",
            GlimEvent::PipelineOpenUrl(_, _) => "PipelineOpenUrl",
            GlimEvent::PipelineSelected(_) => "PipelineSelected",
            GlimEvent::PipelinesFetch(_) => "PipelinesFetch",
            GlimEvent::PipelinesLoaded(_) => "PipelinesLoaded",
            GlimEvent::ProjectDetailsClose => "ProjectDetailsClose",
            GlimEvent::ProjectDetailsOpen(_) => "ProjectDetailsOpen",
            GlimEvent::ProjectFetch(_) => "ProjectFetch",
            GlimEvent::ProjectNext => "ProjectNext",
            GlimEvent::ProjectOpenUrl(_) => "ProjectOpenUrl",
            GlimEvent::ProjectPrevious => "ProjectPrevious",
            GlimEvent::ProjectSelected(_) => "ProjectSelected",
            GlimEvent::ProjectUpdated(_) => "ProjectUpdated",
            GlimEvent::ProjectsFetch => "ProjectsFetch",
            GlimEvent::ProjectsLoaded(_) => "ProjectsLoaded",
            GlimEvent::ScreenCapture => "ScreenCapture",
            GlimEvent::ScreenCaptureToClipboard(_) => "ScreenCaptureToClipboard",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GlitchState {
    #[allow(dead_code)]
    RampedUp,
    #[allow(dead_code)]
    Normal,
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
                        sender.dispatch(GlimEvent::AppTick);
                        last_tick = std::time::Instant::now();
                    }
                }
            })
        };

        Self { sender, receiver, _handler: handler }
    }

    pub fn sender(&self) -> mpsc::Sender<GlimEvent> {
        self.sender.clone()
    }

    pub fn next(&self) -> Result<GlimEvent, mpsc::RecvError> {
        self.receiver.recv()
    }

    pub fn try_next(&self) -> Option<GlimEvent> {
        self.receiver.try_recv().ok()
    }

    fn apply_event(sender: &mpsc::Sender<GlimEvent>) {
        match event::read().expect("unable to read event") {
            CrosstermEvent::Key(e) if e.kind == KeyEventKind::Press => {
                sender.send(GlimEvent::InputKey(e))
            },

            _ => Ok(()),
        }
        .expect("failed to send event")
    }
}

impl From<Vec<ProjectDto>> for GlimEvent {
    fn from(projects: Vec<ProjectDto>) -> Self {
        GlimEvent::ProjectsLoaded(projects)
    }
}

impl From<Vec<PipelineDto>> for GlimEvent {
    fn from(pipelines: Vec<PipelineDto>) -> Self {
        GlimEvent::PipelinesLoaded(pipelines)
    }
}

impl From<(ProjectId, PipelineId, Vec<JobDto>)> for GlimEvent {
    fn from(value: (ProjectId, PipelineId, Vec<JobDto>)) -> Self {
        let (project_id, pipeline_id, jobs) = value;
        GlimEvent::JobsLoaded(project_id, pipeline_id, jobs)
    }
}

impl IntoGlimEvent for Vec<ProjectDto> {
    fn into_glim_event(self) -> GlimEvent {
        GlimEvent::ProjectsLoaded(self)
    }
}

impl IntoGlimEvent for Vec<PipelineDto> {
    fn into_glim_event(self) -> GlimEvent {
        GlimEvent::PipelinesLoaded(self)
    }
}

impl IntoGlimEvent for (ProjectId, PipelineId, Vec<JobDto>) {
    fn into_glim_event(self) -> GlimEvent {
        let (project_id, pipeline_id, jobs) = self;
        GlimEvent::JobsLoaded(project_id, pipeline_id, jobs)
    }
}
