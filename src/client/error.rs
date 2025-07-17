//! Error types for GitLab client operations

use compact_str::CompactString;
use thiserror::Error;

/// Structured error types for GitLab client operations
#[derive(Debug, Error)]
pub enum ClientError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON parsing error with endpoint context
    #[error("Failed to parse JSON response from {endpoint}: {message}")]
    JsonParse {
        endpoint: String,
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

    /// Configuration field validation failed
    #[error("Invalid {field}: {message}")]
    ConfigValidation { field: String, message: String },

    /// Authentication failed
    #[error("Authentication failed")]
    Authentication,

    /// GitLab token is invalid
    #[error("GitLab token is invalid")]
    InvalidToken,

    /// GitLab token has expired
    #[error("GitLab token has expired")]
    ExpiredToken,

    /// Network timeout
    #[error("Request timeout")]
    #[allow(dead_code)]
    Timeout,

    /// Invalid URL format
    #[error("Invalid URL: {url}")]
    #[allow(dead_code)]
    InvalidUrl { url: String },

    /// Resource not found
    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    /// Rate limit exceeded
    #[error("Rate limit exceeded, retry after {retry_after:?}")]
    RateLimit { retry_after: Option<std::time::Duration> },
}

impl ClientError {
    /// Create a JSON parsing error with endpoint context
    pub fn json_parse(
        endpoint: impl Into<String>,
        message: impl Into<String>,
        source: serde_json::Error,
    ) -> Self {
        Self::JsonParse {
            endpoint: endpoint.into(),
            message: message.into(),
            source,
        }
    }

    /// Create a GitLab API error
    pub fn gitlab_api(message: impl Into<CompactString>) -> Self {
        Self::GitlabApi { message: message.into() }
    }

    /// Create a configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config(message.into())
    }

    /// Create a configuration field validation error
    pub fn config_validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ConfigValidation { field: field.into(), message: message.into() }
    }

    /// Create an invalid URL error
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn is_retryable(&self) -> bool {
        match self {
            ClientError::Http(e) => e.is_timeout() || e.is_connect(),
            ClientError::Timeout => true,
            ClientError::RateLimit { .. } => true,
            _ => false,
        }
    }

    /// Check if this error indicates a temporary network issue
    #[allow(dead_code)]
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
