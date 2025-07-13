//! Error types for GitLab client operations

use compact_str::CompactString;
use thiserror::Error;

/// Structured error types for GitLab client operations
#[derive(Debug, Error)]
pub enum ClientError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON deserialization failed
    #[error("JSON deserialization failed: {message}")]
    JsonParse {
        message: String,
        #[source]
        source: serde_json::Error,
    },

    /// GitLab API returned an error response
    #[error("GitLab API error: {message}")]
    GitlabApi { message: CompactString },

    /// Configuration is invalid
    #[error("Configuration error: {0}")]
    Config(String),

    /// Authentication failed
    #[error("Authentication failed")]
    Authentication,

    /// Resource not found
    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    /// Rate limit exceeded
    #[error("Rate limit exceeded, retry after {retry_after:?}")]
    RateLimit { retry_after: Option<std::time::Duration> },
}

impl ClientError {
    /// Create a JSON parsing error with context
    pub fn json_parse(message: impl Into<String>, source: serde_json::Error) -> Self {
        Self::JsonParse { message: message.into(), source }
    }

    /// Create a GitLab API error
    pub fn gitlab_api(message: impl Into<CompactString>) -> Self {
        Self::GitlabApi { message: message.into() }
    }

    /// Create a configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config(message.into())
    }

    /// Create a not found error
    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::NotFound { resource: resource.into() }
    }

    /// Create a rate limit error
    pub fn rate_limit(retry_after: Option<std::time::Duration>) -> Self {
        Self::RateLimit { retry_after }
    }
}

/// Result type alias for client operations
pub type Result<T> = std::result::Result<T, ClientError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = ClientError::config("Invalid token");
        assert!(matches!(err, ClientError::Config(_)));
        assert_eq!(err.to_string(), "Configuration error: Invalid token");
    }

    #[test]
    fn test_gitlab_api_error() {
        let err = ClientError::gitlab_api("Project not found");
        assert!(matches!(err, ClientError::GitlabApi { .. }));
        assert_eq!(err.to_string(), "GitLab API error: Project not found");
    }
}
