//! Configuration management for GitLab client

use std::{path::PathBuf, time::Duration};

use chrono::{DateTime, Utc};
use compact_str::CompactString;

use super::error::{ClientError, Result};
use crate::glim_app::GlimConfig;

/// Main configuration for GitLab client
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// GitLab instance base URL
    pub base_url: CompactString,
    /// Private access token
    pub private_token: CompactString,
    /// Optional search filter for projects
    pub search_filter: Option<CompactString>,
    /// Polling configuration
    pub polling: PollingConfig,
    /// Request configuration
    pub request: RequestConfig,
    /// Debug configuration
    pub debug: DebugConfig,
}

/// Polling intervals configuration
#[derive(Debug, Clone)]
pub struct PollingConfig {
    /// Interval for fetching projects
    pub projects_interval: Duration,
    /// Interval for fetching active jobs
    pub jobs_interval: Duration,
}

/// HTTP request configuration
#[derive(Debug, Clone)]
pub struct RequestConfig {
    /// Number of items per page for paginated requests
    pub per_page: u32,
    /// Request timeout
    pub timeout: Duration,
    /// Maximum number of retries for failed requests
    pub max_retries: u32,
}

/// Debug and logging configuration
#[derive(Debug, Clone)]
pub struct DebugConfig {
    /// Enable debug logging of HTTP responses
    pub log_responses: bool,
    /// Directory for storing debug logs
    pub log_directory: Option<PathBuf>,
}

/// Query parameters for fetching projects
#[derive(Debug, Clone, Default)]
pub struct ProjectQuery {
    /// Search filter for project names
    pub search_filter: Option<CompactString>,
    /// Only fetch projects updated after this time
    pub updated_after: Option<DateTime<Utc>>,
    /// Number of results per page
    pub per_page: u32,
    /// Include project statistics
    pub include_statistics: bool,
    /// Include archived projects
    pub archived: bool,
    /// Only include projects where user is a member
    pub membership: bool,
    /// Search in namespaces
    pub search_namespaces: bool,
}

/// Query parameters for fetching pipelines
#[derive(Debug, Clone, Default)]
pub struct PipelineQuery {
    /// Only fetch pipelines updated after this time
    pub updated_after: Option<DateTime<Utc>>,
    /// Number of results per page
    pub per_page: u32,
    /// Pipeline scope (running, pending, finished, etc.)
    pub scope: Option<PipelineScope>,
    /// Pipeline status filter
    pub status: Option<PipelineStatus>,
}

/// Pipeline scope for filtering
#[derive(Debug, Clone)]
pub enum PipelineScope {
    Running,
    Pending,
    Finished,
    Branches,
    Tags,
}

/// Pipeline status for filtering
#[derive(Debug, Clone)]
pub enum PipelineStatus {
    Created,
    WaitingForResource,
    Preparing,
    Pending,
    Running,
    Success,
    Failed,
    Canceled,
    Skipped,
    Manual,
    Scheduled,
}

impl Default for PollingConfig {
    fn default() -> Self {
        Self {
            projects_interval: Duration::from_secs(60),
            jobs_interval: Duration::from_secs(30),
        }
    }
}

impl Default for RequestConfig {
    fn default() -> Self {
        Self {
            per_page: 100,
            timeout: Duration::from_secs(30),
            max_retries: 3,
        }
    }
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            log_responses: false,
            log_directory: Some(PathBuf::from("glim-logs")),
        }
    }
}

impl ClientConfig {
    /// Create a new client configuration
    pub fn new(
        base_url: impl Into<CompactString>,
        private_token: impl Into<CompactString>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            private_token: private_token.into(),
            search_filter: None,
            polling: PollingConfig::default(),
            request: RequestConfig::default(),
            debug: DebugConfig::default(),
        }
    }

    /// Create a builder for fluent configuration
    pub fn builder() -> ClientConfigBuilder {
        ClientConfigBuilder::default()
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.base_url.is_empty() {
            return Err(ClientError::config("Base URL cannot be empty"));
        }

        if self.private_token.is_empty() {
            return Err(ClientError::config("Private token cannot be empty"));
        }

        if !self.base_url.starts_with("http://") && !self.base_url.starts_with("https://") {
            return Err(ClientError::config("Base URL must start with http:// or https://"));
        }

        if self.request.per_page == 0 || self.request.per_page > 100 {
            return Err(ClientError::config("per_page must be between 1 and 100"));
        }

        if self.request.timeout.is_zero() {
            return Err(ClientError::config("Timeout must be greater than zero"));
        }

        Ok(())
    }

    /// Create default project query with config values
    pub fn default_project_query(&self) -> ProjectQuery {
        ProjectQuery {
            search_filter: self.search_filter.clone(),
            per_page: self.request.per_page,
            include_statistics: true,
            archived: false,
            membership: true,
            search_namespaces: true,
            ..Default::default()
        }
    }

    /// Create default pipeline query with config values
    pub fn default_pipeline_query(&self) -> PipelineQuery {
        PipelineQuery {
            per_page: self.request.per_page.min(60), // GitLab API limit for pipelines
            ..Default::default()
        }
    }
}

impl From<GlimConfig> for ClientConfig {
    fn from(config: GlimConfig) -> Self {
        Self::new(config.gitlab_url, config.gitlab_token).with_search_filter(config.search_filter)
    }
}

impl ClientConfig {
    /// Set search filter
    pub fn with_search_filter(mut self, filter: Option<CompactString>) -> Self {
        self.search_filter = filter;
        self
    }

    /// Set polling configuration
    pub fn with_polling(mut self, polling: PollingConfig) -> Self {
        self.polling = polling;
        self
    }

    /// Set request configuration
    pub fn with_request(mut self, request: RequestConfig) -> Self {
        self.request = request;
        self
    }

    /// Set debug configuration
    pub fn with_debug(mut self, debug: DebugConfig) -> Self {
        self.debug = debug;
        self
    }

    /// Enable debug logging
    pub fn with_debug_logging(mut self, enabled: bool) -> Self {
        self.debug.log_responses = enabled;
        self
    }
}

/// Builder for ClientConfig
#[derive(Debug, Default)]
pub struct ClientConfigBuilder {
    base_url: Option<CompactString>,
    private_token: Option<CompactString>,
    search_filter: Option<CompactString>,
    polling: Option<PollingConfig>,
    request: Option<RequestConfig>,
    debug: Option<DebugConfig>,
}

impl ClientConfigBuilder {
    /// Set base URL
    pub fn base_url(mut self, url: impl Into<CompactString>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Set private token
    pub fn private_token(mut self, token: impl Into<CompactString>) -> Self {
        self.private_token = Some(token.into());
        self
    }

    /// Set search filter
    pub fn search_filter(mut self, filter: impl Into<CompactString>) -> Self {
        self.search_filter = Some(filter.into());
        self
    }

    /// Set polling configuration
    pub fn polling(mut self, polling: PollingConfig) -> Self {
        self.polling = Some(polling);
        self
    }

    /// Set request configuration
    pub fn request(mut self, request: RequestConfig) -> Self {
        self.request = Some(request);
        self
    }

    /// Set debug configuration
    pub fn debug(mut self, debug: DebugConfig) -> Self {
        self.debug = Some(debug);
        self
    }

    /// Enable debug logging
    pub fn debug_logging(mut self, enabled: bool) -> Self {
        let mut debug = self.debug.unwrap_or_default();
        debug.log_responses = enabled;
        self.debug = Some(debug);
        self
    }

    /// Set polling interval for projects
    pub fn projects_interval(mut self, interval: Duration) -> Self {
        let mut polling = self.polling.unwrap_or_default();
        polling.projects_interval = interval;
        self.polling = Some(polling);
        self
    }

    /// Set polling interval for jobs
    pub fn jobs_interval(mut self, interval: Duration) -> Self {
        let mut polling = self.polling.unwrap_or_default();
        polling.jobs_interval = interval;
        self.polling = Some(polling);
        self
    }

    /// Set items per page
    pub fn per_page(mut self, per_page: u32) -> Self {
        let mut request = self.request.unwrap_or_default();
        request.per_page = per_page;
        self.request = Some(request);
        self
    }

    /// Set request timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        let mut request = self.request.unwrap_or_default();
        request.timeout = timeout;
        self.request = Some(request);
        self
    }

    /// Build the configuration
    pub fn build(self) -> Result<ClientConfig> {
        let base_url = self
            .base_url
            .ok_or_else(|| ClientError::config("Base URL is required"))?;
        let private_token = self
            .private_token
            .ok_or_else(|| ClientError::config("Private token is required"))?;

        let config = ClientConfig {
            base_url,
            private_token,
            search_filter: self.search_filter,
            polling: self.polling.unwrap_or_default(),
            request: self.request.unwrap_or_default(),
            debug: self.debug.unwrap_or_default(),
        };

        config.validate()?;
        Ok(config)
    }
}

impl ProjectQuery {
    /// Create a new project query
    pub fn new() -> Self {
        Self::default()
    }

    /// Set search filter
    pub fn with_search_filter(mut self, filter: Option<CompactString>) -> Self {
        self.search_filter = filter;
        self
    }

    /// Set updated after filter
    pub fn with_updated_after(mut self, updated_after: Option<DateTime<Utc>>) -> Self {
        self.updated_after = updated_after;
        self
    }

    /// Set per page limit
    pub fn with_per_page(mut self, per_page: u32) -> Self {
        self.per_page = per_page;
        self
    }
}

impl PipelineQuery {
    /// Create a new pipeline query
    pub fn new() -> Self {
        Self::default()
    }

    /// Set updated after filter
    pub fn with_updated_after(mut self, updated_after: Option<DateTime<Utc>>) -> Self {
        self.updated_after = updated_after;
        self
    }

    /// Set per page limit
    pub fn with_per_page(mut self, per_page: u32) -> Self {
        self.per_page = per_page;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = ClientConfig::builder()
            .base_url("https://gitlab.example.com")
            .private_token("test-token")
            .search_filter("frontend")
            .debug_logging(true)
            .build()
            .unwrap();

        assert_eq!(config.base_url, "https://gitlab.example.com");
        assert_eq!(config.private_token, "test-token");
        assert_eq!(config.search_filter, Some("frontend".into()));
        assert!(config.debug.log_responses);
    }

    #[test]
    fn test_config_validation() {
        // Valid config
        let config = ClientConfig::new("https://gitlab.com", "token");
        assert!(config.validate().is_ok());

        // Empty base URL
        let config = ClientConfig::new("", "token");
        assert!(config.validate().is_err());

        // Empty token
        let config = ClientConfig::new("https://gitlab.com", "");
        assert!(config.validate().is_err());

        // Invalid URL
        let config = ClientConfig::new("not-a-url", "token");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_from_glim_config() {
        let glim_config = GlimConfig {
            gitlab_url: "https://gitlab.example.com".into(),
            gitlab_token: "test-token".into(),
            search_filter: Some("test".into()),
            log_level: Some("Off".into()),
        };

        let client_config = ClientConfig::from(glim_config);
        assert_eq!(client_config.base_url, "https://gitlab.example.com");
        assert_eq!(client_config.private_token, "test-token");
        assert_eq!(client_config.search_filter, Some("test".into()));
    }

    #[test]
    fn test_default_queries() {
        let config = ClientConfig::new("https://gitlab.com", "token")
            .with_search_filter(Some("test".into()));

        let project_query = config.default_project_query();
        assert_eq!(project_query.search_filter, Some("test".into()));
        assert!(project_query.include_statistics);
        assert!(!project_query.archived);

        let pipeline_query = config.default_pipeline_query();
        assert_eq!(pipeline_query.per_page, 60); // Limited by GitLab API
    }
}
