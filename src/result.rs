use std::path::PathBuf;

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

    #[error("Configuration file not found: {path}")]
    ConfigFileNotFound { path: PathBuf },

    #[error("Failed to load configuration from: {path}")]
    ConfigLoadError { path: PathBuf, message: String },

    #[error("Failed to save configuration to: {path}")]
    ConfigSaveError { path: PathBuf, message: String },

    #[error("Invalid configuration: {field}")]
    ConfigValidationError { field: String, message: String },

    #[error("Configuration connection test failed: {message}")]
    ConfigConnectionError { message: String },

    #[error("{0}")]
    GeneralError(CompactString),

    #[error("{0:?} - JSON: {1}")]
    #[allow(dead_code)]
    JsonDeserializeError(Category, CompactString),

    #[error("project_id={0}/pipeline_id={1}: {2}")]
    #[allow(dead_code)]
    GitlabGetJobsError(ProjectId, PipelineId, CompactString),
    #[error("project_id={0}/pipeline_id={1}: {2}")]
    #[allow(dead_code)]
    GitlabGetTriggerJobsError(ProjectId, PipelineId, CompactString),
    #[error("project_id={0}/pipeline_id={1}: {2}")]
    #[allow(dead_code)]
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

impl GlimError {
    /// Create a configuration file not found error
    pub fn config_file_not_found(path: PathBuf) -> Self {
        Self::ConfigFileNotFound { path }
    }

    /// Create a configuration load error
    pub fn config_load_error(path: PathBuf, source: impl std::fmt::Display) -> Self {
        Self::ConfigLoadError { path, message: source.to_string() }
    }

    /// Create a configuration save error
    pub fn config_save_error(path: PathBuf, source: impl std::fmt::Display) -> Self {
        Self::ConfigSaveError { path, message: source.to_string() }
    }

    /// Create a configuration validation error
    pub fn config_validation_error(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ConfigValidationError { field: field.into(), message: message.into() }
    }

    /// Create a configuration connection error
    pub fn config_connection_error(message: impl Into<String>) -> Self {
        Self::ConfigConnectionError { message: message.into() }
    }
}
