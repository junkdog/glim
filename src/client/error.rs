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

    /// Network timeout
    #[error("Request timeout")]
    Timeout,

    /// Invalid URL format
    #[error("Invalid URL: {url}")]
    InvalidUrl { url: String },

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

    /// Create an invalid URL error
    pub fn invalid_url(url: impl Into<String>) -> Self {
        Self::InvalidUrl { url: url.into() }
    }

    /// Create a not found error
    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::NotFound { resource: resource.into() }
    }

    /// Create a rate limit error
    pub fn rate_limit(retry_after: Option<std::time::Duration>) -> Self {
        Self::RateLimit { retry_after }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            ClientError::Http(e) => e.is_timeout() || e.is_connect(),
            ClientError::Timeout => true,
            ClientError::RateLimit { .. } => true,
            _ => false,
        }
    }

    /// Check if this error indicates a temporary network issue
    pub fn is_network_error(&self) -> bool {
        match self {
            ClientError::Http(e) => e.is_timeout() || e.is_connect() || e.is_request(),
            ClientError::Timeout => true,
            _ => false,
        }
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

    #[test]
    fn test_retryable_errors() {
        assert!(ClientError::Timeout.is_retryable());
        assert!(ClientError::rate_limit(None).is_retryable());
        assert!(!ClientError::Authentication.is_retryable());
        assert!(!ClientError::config("test").is_retryable());
    }

    #[test]
    fn test_network_errors() {
        assert!(ClientError::Timeout.is_network_error());
        assert!(!ClientError::Authentication.is_network_error());
        assert!(!ClientError::config("test").is_network_error());
    }
}
