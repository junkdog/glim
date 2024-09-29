use serde_json::error::Category;
use thiserror::Error;
use crate::id::{PipelineId, ProjectId};

pub type Result<T> = std::result::Result<T, GlimError>;

#[derive(Debug, Clone,  Error)]
pub enum GlimError {
    #[error("The provided Gitlab token is invalid.")]
    InvalidGitlabToken,
    #[error("The provided Gitlab token has expired.")]
    ExpiredGitlabToken,
    #[error("{0}")]
    ConfigError(String),

    #[error("{0}")]
    GeneralError(String),

    #[error("{:0} - JSON: {1}")]
    JsonDeserializeError(Category, String),

    #[error("project_id={0}/pipeline_id={1}: {2}")]
    GitlabGetJobsError(ProjectId, PipelineId, String),
    #[error("project_id={0}/pipeline_id={1}: {2}")]
    GitlabGetTriggerJobsError(ProjectId, PipelineId, String),
    #[error("project_id={0}/pipeline_id={1}: {2}")]
    GitlabGetPipelinesError(ProjectId, PipelineId, String),
}

impl From<reqwest::Error> for GlimError {
    fn from(e: reqwest::Error) -> Self {
        match () {
            _ => GlimError::GeneralError(e.to_string()),
        }
    }
}