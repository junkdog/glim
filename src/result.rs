use compact_str::{CompactString, ToCompactString};
use serde_json::error::Category;
use thiserror::Error;

use crate::id::{PipelineId, ProjectId};

pub type Result<T> = std::result::Result<T, GlimError>;

#[derive(Debug, Clone, Error)]
pub enum GlimError {
    #[error("The provided Gitlab token is invalid.")]
    InvalidGitlabToken,
    #[error("The provided Gitlab token has expired.")]
    ExpiredGitlabToken,
    #[error("{0}")]
    ConfigError(CompactString),

    #[error("{0}")]
    GeneralError(CompactString),

    #[error("{:0} - JSON: {1}")]
    JsonDeserializeError(Category, CompactString),

    #[error("project_id={0}/pipeline_id={1}: {2}")]
    GitlabGetJobsError(ProjectId, PipelineId, CompactString),
    #[error("project_id={0}/pipeline_id={1}: {2}")]
    GitlabGetTriggerJobsError(ProjectId, PipelineId, CompactString),
    #[error("project_id={0}/pipeline_id={1}: {2}")]
    GitlabGetPipelinesError(ProjectId, PipelineId, CompactString),
}

impl From<reqwest::Error> for GlimError {
    fn from(e: reqwest::Error) -> Self {
        GlimError::GeneralError(e.to_compact_string())
    }
}

impl From<crate::client::ClientError> for GlimError {
    fn from(e: crate::client::ClientError) -> Self {
        GlimError::GeneralError(e.to_string().into())
    }
}
