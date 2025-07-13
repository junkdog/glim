// GitLab API Documentation: https://docs.gitlab.com/ee/api/api_resources.html
use chrono::{DateTime, Duration, Local, Utc};
use compact_str::{CompactString, ToCompactString};
use itertools::Itertools;
use ratatui::{
    text::{Line, Span, Text},
    widgets::Row,
};
use serde::Deserialize;

use crate::{
    id::{JobId, PipelineId, ProjectId},
    theme::theme,
    ui::{format_duration, widget::text_from},
};

#[derive(Clone, Debug)]
pub struct Project {
    pub id: ProjectId,
    pub path: CompactString,
    pub description: Option<CompactString>,
    pub default_branch: CompactString,
    pub ssh_git_url: CompactString,
    pub url: CompactString,
    pub last_activity_at: DateTime<Utc>,
    pub pipelines: Option<Vec<Pipeline>>,
    pub commit_count: u32,
    pub repo_size_kb: u64,
    pub artifacts_size_kb: u64,
}

#[derive(Clone, Debug)]
pub struct Pipeline {
    pub id: PipelineId,
    pub project_id: ProjectId,
    pub status: PipelineStatus,
    pub source: PipelineSource,
    pub branch: CompactString,
    pub url: CompactString,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub jobs: Option<Vec<Job>>,
    pub commit: Option<Commit>,
}

#[derive(Clone, Debug)]
pub struct Commit {
    pub title: CompactString,
    #[allow(dead_code)]
    pub author_name: CompactString,
}

#[derive(Clone, Debug)]
pub struct Job {
    pub id: JobId,
    pub name: CompactString,
    pub status: PipelineStatus,
    #[allow(dead_code)]
    pub stage: CompactString,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub url: CompactString,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProjectDto {
    pub id: ProjectId,
    pub path_with_namespace: CompactString,
    pub description: Option<CompactString>,
    pub default_branch: CompactString,
    pub ssh_url_to_repo: CompactString,
    pub web_url: CompactString,
    pub last_activity_at: DateTime<Utc>,
    pub statistics: StatisticsDto,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct StatisticsDto {
    pub commit_count: u32,
    pub job_artifacts_size: u64,
    pub repository_size: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CommitDto {
    pub title: CompactString,
    pub author_name: CompactString,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JobDto {
    pub id: JobId,
    pub name: CompactString,
    pub stage: CompactString,
    pub commit: CommitDto,
    pub status: PipelineStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub web_url: CompactString,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PipelineDto {
    pub id: PipelineId,
    pub project_id: ProjectId,
    pub status: PipelineStatus,
    pub source: PipelineSource,
    #[serde(rename = "ref")]
    pub branch: CompactString,
    pub web_url: CompactString,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStatus {
    #[default]
    Created,
    WaitingForResource,
    Preparing,
    Pending,
    Running,
    Success,
    Failed,
    Canceling,
    Canceled,
    Skipped,
    Manual,
    Scheduled,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PipelineSource {
    #[default]
    Api,
    Chat,
    External,
    ExternalPullRequestEvent,
    MergeRequestEvent,
    OndemandDastScan,
    OndemandDastValidation,
    ParentPipeline,
    Pipeline,
    Push,
    Schedule,
    SecurityOrchestrationPolicy,
    Trigger,
    Web,
    Webide,
}

impl PipelineSource {
    pub fn to_string(&self) -> CompactString {
        match self {
            PipelineSource::Api => "api",
            PipelineSource::Chat => "chat",
            PipelineSource::External => "external",
            PipelineSource::ExternalPullRequestEvent => "pull request",
            PipelineSource::MergeRequestEvent => "merge request",
            PipelineSource::OndemandDastScan => "dast scan",
            PipelineSource::OndemandDastValidation => "dast validation",
            PipelineSource::ParentPipeline => "parent pipeline",
            PipelineSource::Pipeline => "pipeline",
            PipelineSource::Push => "push",
            PipelineSource::Schedule => "schedule",
            PipelineSource::SecurityOrchestrationPolicy => "sec policy",
            PipelineSource::Trigger => "trigger",
            PipelineSource::Web => "web",
            PipelineSource::Webide => "web ide",
        }
        .into()
    }
}

impl PipelineStatus {
    pub(crate) fn is_active(&self) -> bool {
        self < &PipelineStatus::Success
    }
}

impl PipelineSource {
    pub(crate) fn is_interesting(&self) -> bool {
        matches!(
            self,
            PipelineSource::Api
                | PipelineSource::Chat
                | PipelineSource::ParentPipeline
                | PipelineSource::Push
                | PipelineSource::Schedule
                | PipelineSource::Trigger
                | PipelineSource::Web
                | PipelineSource::Webide
        )
    }
}

impl Project {
    #[allow(dead_code)]
    pub fn row(&self) -> Row<'_> {
        Row::new(vec![
            Span::from(self.last_activity_at.to_compact_string()),
            Span::from(self.path.as_str()),
            Span::from(self.default_branch.as_str()),
        ])
    }

    pub fn last_activity(&self) -> DateTime<Utc> {
        self.last_activity_at
    }

    pub fn title(&self) -> CompactString {
        match self.path.rfind('/') {
            Some(i) => self.path[i + 1..].into(),
            None => self.path.clone(),
        }
    }

    pub fn first_pipeline_per_branch(
        &self,
        count: usize,
        predicate: impl Fn(&Pipeline) -> bool,
    ) -> Vec<&Pipeline> {
        if let Some(pipelines) = self.pipelines.as_ref() {
            pipelines
                .iter()
                .filter(|p| p.source.is_interesting() || predicate(p))
                .unique_by(|p| &p.branch)
                .take(count)
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn recent_pipelines(&self) -> Vec<&Pipeline> {
        if let Some(pipelines) = self.pipelines.as_ref() {
            pipelines
                .iter()
                .filter(|p| p.source.is_interesting())
                .take(8)
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn has_active_pipelines(&self) -> bool {
        self.pipelines.as_ref().is_some_and(|ps| {
            ps.iter()
                .any(|p| p.status.is_active() || p.has_active_jobs())
        })
    }

    pub fn path_and_name(&self) -> (&str, &str) {
        match self.path.rfind('/') {
            Some(i) => (&self.path[0..=i], &self.path[i + 1..]),
            None => ("", self.path.as_str()),
        }
    }

    pub fn pipeline(&self, id: PipelineId) -> Option<&Pipeline> {
        self.pipelines
            .as_ref()
            .and_then(|ps| ps.iter().find(|p| p.id == id))
    }
}

impl From<ProjectDto> for Project {
    fn from(p: ProjectDto) -> Self {
        Self {
            id: p.id,
            description: p.description,
            path: p.path_with_namespace,
            default_branch: p.default_branch,
            ssh_git_url: p.ssh_url_to_repo,
            url: p.web_url,
            last_activity_at: p.last_activity_at,
            pipelines: None,
            commit_count: p.statistics.commit_count,
            repo_size_kb: p.statistics.repository_size / 1024,
            artifacts_size_kb: p.statistics.job_artifacts_size / 1024,
        }
    }
}

impl Job {
    pub fn duration(&self) -> Duration {
        match (&self.started_at, &self.finished_at) {
            (Some(begin), Some(end)) => end.signed_duration_since(begin),
            (Some(begin), None) => Utc::now().signed_duration_since(begin),
            _ => Duration::zero(),
        }
    }
}

impl Project {
    pub fn update_pipelines(&mut self, pipelines: Vec<Pipeline>) {
        self.pipelines = Some(
            pipelines
                .iter()
                .map(|p| {
                    if let Some(existing) = self
                        .pipelines
                        .as_ref()
                        .and_then(|ps| ps.iter().find(|ep| ep.id == p.id))
                    {
                        let mut new = p.clone();
                        new.jobs.clone_from(&existing.jobs);
                        new.commit.clone_from(&existing.commit);
                        new
                    } else {
                        p.clone()
                    }
                })
                .sorted_by(|a, b| b.updated_at.cmp(&a.updated_at))
                .collect(),
        );
    }

    pub fn update_project(&mut self, project: Project) {
        self.id = project.id;
        self.path = project.path;
        self.default_branch = project.default_branch;
        self.ssh_git_url = project.ssh_git_url;
        self.url = project.url;
        self.last_activity_at = project.last_activity_at;
    }

    pub fn update_jobs(&mut self, pipeline_id: PipelineId, jobs: Vec<Job>) {
        if let Some(pipelines) = self.pipelines.as_mut() {
            if let Some(pipeline) = pipelines.iter_mut().find(|p| p.id == pipeline_id) {
                pipeline.jobs = Some(jobs);
            }
        }
    }

    pub fn update_commit(&mut self, pipeline_id: PipelineId, commit: Commit) {
        if let Some(pipelines) = self.pipelines.as_mut() {
            if let Some(pipeline) = pipelines.iter_mut().find(|p| p.id == pipeline_id) {
                pipeline.commit = Some(commit);
            }
        }
    }
}

impl From<PipelineDto> for Pipeline {
    fn from(p: PipelineDto) -> Self {
        Self {
            id: p.id,
            project_id: p.project_id,
            status: p.status,
            source: p.source,
            branch: p.branch,
            url: p.web_url,
            created_at: p.created_at,
            updated_at: p.updated_at,
            jobs: None,
            commit: None,
        }
    }
}

impl From<JobDto> for Job {
    fn from(j: JobDto) -> Self {
        Self {
            id: j.id,
            name: j.name,
            stage: j.stage,
            status: j.status,
            created_at: j.created_at,
            started_at: j.started_at,
            finished_at: j.finished_at,
            url: j.web_url,
        }
    }
}

impl From<CommitDto> for Commit {
    fn from(c: CommitDto) -> Self {
        Self {
            title: c.title,
            author_name: c.author_name,
        }
    }
}

impl Pipeline {
    pub fn has_active_jobs(&self) -> bool {
        self.jobs
            .as_ref()
            .is_some_and(|jobs| jobs.iter().any(|j| j.status.is_active()))
    }

    pub fn active_job(&self) -> Option<&Job> {
        self.jobs
            .as_ref()
            .and_then(|jobs| jobs.iter().find(|j| j.status.is_active()))
    }

    pub fn failed_job(&self) -> Option<&Job> {
        self.jobs.as_ref().and_then(|jobs| {
            jobs.iter()
                .find(|j| j.status == PipelineStatus::Failed)
        })
    }

    pub fn active_job_name(&self) -> CompactString {
        self.active_job()
            .map_or("".into(), |j| j.name.clone())
    }

    #[allow(dead_code)]
    pub fn has_failed_jobs(&self) -> bool {
        self.failed_job().is_some()
    }

    pub fn failing_job_name(&self) -> Option<CompactString> {
        self.failed_job().map(|j| j.name.clone())
    }

    pub fn job(&self, id: JobId) -> Option<&Job> {
        self.jobs
            .as_ref()
            .and_then(|jobs| jobs.iter().find(|j| j.id == id))
    }

    /// Returns the duration of the pipeline, measured from the time it was started
    /// to the time it was finished. If the pipeline is still running, the duration
    /// is measured from the time it was started to the current time.
    pub fn duration(&self) -> Duration {
        match (&self.created_at, &self.finished_at()) {
            (begin, Some(end)) => end.signed_duration_since(begin),
            (begin, None) => Utc::now().signed_duration_since(begin),
        }
    }

    fn finished_at(&self) -> Option<DateTime<Utc>> {
        match () {
            _ if self.status.is_active() => None,
            _ => self
                .jobs
                .as_ref()
                .and_then(|jobs| jobs.iter().map(|j| j.finished_at).max().unwrap()),
        }
    }
}

pub fn parse_row<'a>(project: &'a Project) -> Row<'a> {
    let distinct_by_branch = project.first_pipeline_per_branch(3, |p| p.status.is_active());

    let pipeline_to_span = |p: &'a Pipeline| -> Line<'a> {
        let icon = p.status.icon();
        let branch = p.branch.as_str();

        let updated_at = p.updated_at.with_timezone(&Local);
        match () {
            _ if p.has_active_jobs() => Line::from(vec![
                Span::from(updated_at.format("%a, %d %b").to_compact_string()).style(theme().date),
                Span::from(" "),
                Span::from(updated_at.format("%H:%M:%S").to_compact_string()).style(theme().time),
                Span::from(" "),
                Span::from(p.jobs.as_ref().unwrap().icon()),
                Span::from(" "),
                Span::from(branch).style(theme().pipeline_branch),
                Span::from(" "),
                Span::from(p.active_job_name()).style(theme().pipeline_job),
                Span::from(" "),
                Span::from(format_duration(p.duration())).style(theme().time),
            ]),
            _ if p.status.is_active() => Line::from(vec![
                Span::from(updated_at.format("%a, %d %b").to_compact_string()).style(theme().date),
                Span::from(" "),
                Span::from(updated_at.format("%H:%M:%S").to_compact_string()).style(theme().time),
                Span::from(" "),
                Span::from(icon),
                Span::from(" "),
                Span::from(branch).style(theme().pipeline_branch),
                Span::from(" "),
                Span::from(format_duration(p.duration())).style(theme().time),
            ]),
            _ => Line::from(vec![
                Span::from(updated_at.format("%a, %d %b").to_compact_string()).style(theme().date),
                Span::from(" "),
                Span::from(updated_at.format("%H:%M:%S").to_compact_string()).style(theme().time),
                Span::from(" "),
                Span::from(icon),
                Span::from(" "),
                Span::from(branch).style(theme().pipeline_branch),
            ]),
        }
    };

    let pipeline_spans: Vec<Line<'a>> = distinct_by_branch
        .iter()
        .map(|p| pipeline_to_span(p))
        .collect();

    let last_activity = project.last_activity_at.with_timezone(&Local);

    let project_path = match project.path.rfind('/') {
        Some(i) => Text::from(vec![
            Line::from(&project.path[i + 1..]).style(theme().project_name),
            Line::from(&project.path[0..=i]).style(theme().project_parents),
        ]),
        None => Text::from(Span::from(&project.path)).style(theme().project_name),
    };

    Row::new(vec![text_from(last_activity), project_path, Text::from(pipeline_spans)]).height(3)
}

/// Represents types that can be associated with an icon.
///
/// The icon returned is expected to be a string that may contain
/// special characters or emojis
pub trait IconRepresentable {
    fn icon(&self) -> CompactString;
}

impl IconRepresentable for PipelineStatus {
    fn icon(&self) -> CompactString {
        match self {
            PipelineStatus::Created => "⚪",
            PipelineStatus::WaitingForResource => "⏳",
            PipelineStatus::Preparing => "🟡",
            PipelineStatus::Pending => "🕒",
            PipelineStatus::Running => "🔵",
            PipelineStatus::Success => "🟢",
            PipelineStatus::Failed => "🔴",
            PipelineStatus::Canceled => "🚫",
            PipelineStatus::Canceling => "🚫",
            PipelineStatus::Skipped => "⚫",
            PipelineStatus::Manual => "🟣",
            PipelineStatus::Scheduled => "📅",
            PipelineStatus::Unknown => "❓",
        }
        .into()
    }
}

impl IconRepresentable for &Vec<Job> {
    fn icon(&self) -> CompactString {
        self.iter()
            .map(|j| j.status.icon())
            .collect::<CompactString>()
    }
}

impl IconRepresentable for Pipeline {
    fn icon(&self) -> CompactString {
        self.jobs
            .as_ref()
            .map(|jobs| jobs.icon())
            .unwrap_or(self.status.icon())
    }
}
