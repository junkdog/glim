use std::collections::VecDeque;

use compact_str::CompactString;
use serde_json::error::Category;

use crate::{
    event::GlimEvent,
    id::{JobId, PipelineId, ProjectId},
    result::GlimError,
};

#[derive(Debug)]
pub struct NoticeService {
    info_notices: VecDeque<Notice>,
    error_notices: VecDeque<Notice>,
    most_recent: Option<Notice>,
}

#[derive(Debug, Clone)]
pub struct Notice {
    pub level: NoticeLevel,
    pub message: NoticeMessage,
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub enum NoticeLevel {
    Info,
    Error,
}

#[derive(Debug, Clone)]
pub enum NoticeMessage {
    GeneralMessage(CompactString),
    JobLogDownloaded(ProjectId, PipelineId, JobId),
    ScreenCaptured,
    // InvalidGitlabToken,
    // ExpiredGitlabToken,
    ConfigError(CompactString),
    JsonDeserializeError(Category, CompactString),
    GitlabGetJobsError(ProjectId, PipelineId, CompactString),
    GitlabGetTriggerJobsError(ProjectId, PipelineId, CompactString),
    GitlabGetPipelinesError(ProjectId, PipelineId, CompactString),
}

impl NoticeService {
    pub fn new() -> Self {
        Self {
            info_notices: VecDeque::new(),
            error_notices: VecDeque::new(),
            most_recent: None,
        }
    }

    pub fn apply(&mut self, event: &GlimEvent) {
        match event {
            GlimEvent::AppError(e) => match e.clone() {
                // GlimError::InvalidGitlabToken => {}
                // GlimError::ExpiredGitlabToken => {}
                GlimError::ConfigError(s) => Some(NoticeMessage::ConfigError(s)),
                GlimError::GeneralError(s) => Some(NoticeMessage::GeneralMessage(s)),
                GlimError::JsonDeserializeError(cat, json) => {
                    Some(NoticeMessage::JsonDeserializeError(cat, json))
                },
                GlimError::GitlabGetJobsError(project_id, pipeline_id, s) => {
                    Some(NoticeMessage::GitlabGetJobsError(project_id, pipeline_id, s))
                },
                GlimError::GitlabGetTriggerJobsError(project_id, pipeline_id, s) => {
                    Some(NoticeMessage::GitlabGetTriggerJobsError(project_id, pipeline_id, s))
                },
                GlimError::GitlabGetPipelinesError(project_id, pipeline_id, s) => {
                    Some(NoticeMessage::GitlabGetPipelinesError(project_id, pipeline_id, s))
                },
                _ => None,
            }
            .map(|m| self.push_notice(NoticeLevel::Error, m))
            .unwrap_or(()),
            GlimEvent::JobLogDownloaded(_project_id, _job_id, _) => self.push_notice(
                NoticeLevel::Info,
                NoticeMessage::GeneralMessage("Job log downloaded".into()),
            ),
            GlimEvent::ScreenCaptureToClipboard(_) => self.push_notice(
                NoticeLevel::Info,
                NoticeMessage::ScreenCaptured,
            ),
            _ => {},
        }
    }

    pub fn has_error(&self) -> bool {
        !self.error_notices.is_empty()
    }

    pub fn last_notification(&self) -> Option<&Notice> {
        self.most_recent.as_ref()
    }

    pub fn pop_notice(&mut self) -> Option<Notice> {
        let notice = self
            .error_notices
            .pop_front()
            .or_else(|| self.info_notices.pop_front());

        if notice.is_some() {
            self.most_recent = notice.clone();
        }

        notice
    }

    pub fn push_notice(&mut self, level: NoticeLevel, message: NoticeMessage) {
        let notice = Notice { level, message };

        match level {
            NoticeLevel::Info => self.info_notices.push_back(notice),
            NoticeLevel::Error => self.error_notices.push_back(notice),
        }
    }
}
