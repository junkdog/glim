//! Core HTTP client for GitLab API

use std::sync::RwLock;

use chrono::Local;
use compact_str::{format_compact, CompactString};
use reqwest::{Client, RequestBuilder, Response};
use serde::Deserialize;
use tracing::{debug, instrument, warn};

use super::{
    config::{ClientConfig, PipelineQuery, ProjectQuery},
    error::{ClientError, Result},
};
use crate::{
    domain::{JobDto, PipelineDto, ProjectDto},
    id::{JobId, PipelineId, ProjectId},
};

/// Pure HTTP client for GitLab API
#[derive(Debug)]
pub struct GitlabApi {
    client: RwLock<Client>,
    config: RwLock<ClientConfig>,
}

/// GitLab API error response formats
#[derive(Debug, Deserialize)]
struct GitlabApiError {
    error: CompactString,
    error_description: Option<CompactString>,
}

#[derive(Debug, Deserialize)]
struct GitlabApiError2 {
    message: CompactString,
}

impl GitlabApi {
    pub fn force_new(config: ClientConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(config.request.timeout)
            .build()
            .map_err(ClientError::Http)?;

        Ok(Self {
            client: RwLock::new(client),
            config: RwLock::new(config),
        })
    }

    /// Get projects from GitLab API
    #[instrument(skip(self), fields(per_page = %query.per_page))]
    pub async fn get_projects(&self, query: &ProjectQuery) -> Result<Vec<ProjectDto>> {
        let url = self.build_projects_url(query);
        self.get_json(&url).await
    }

    /// Get pipelines for a project
    #[instrument(skip(self), fields(project_id = %project_id, per_page = %query.per_page))]
    pub async fn get_pipelines(
        &self,
        project_id: ProjectId,
        query: &PipelineQuery,
    ) -> Result<Vec<PipelineDto>> {
        let url = self.build_pipelines_url(project_id, query);
        self.get_json(&url).await
    }

    /// Get jobs for a pipeline
    #[instrument(skip(self), fields(project_id = %project_id, pipeline_id = %pipeline_id))]
    pub async fn get_jobs(
        &self,
        project_id: ProjectId,
        pipeline_id: PipelineId,
    ) -> Result<Vec<JobDto>> {
        let base_url = {
            let config = self.config.read().unwrap();
            format_compact!(
                "{}/projects/{}/pipelines/{}",
                config.base_url,
                project_id,
                pipeline_id
            )
        };

        // Fetch both regular jobs and trigger jobs concurrently
        let jobs_url = format_compact!("{}/jobs", base_url);
        let bridges_url = format_compact!("{}/bridges", base_url);

        let (jobs_result, bridges_result) = tokio::try_join!(
            self.get_json::<Vec<JobDto>>(&jobs_url),
            self.get_json::<Vec<JobDto>>(&bridges_url)
        )?;

        // Combine and sort by ID
        let mut all_jobs = jobs_result;
        all_jobs.extend(bridges_result);
        all_jobs.sort_by_key(|job| job.id);

        debug!(job_count = all_jobs.len(), "Successfully fetched jobs");
        Ok(all_jobs)
    }

    /// Get job trace/log
    #[instrument(skip(self), fields(project_id = %project_id, job_id = %job_id))]
    pub async fn get_job_trace(
        &self,
        project_id: ProjectId,
        job_id: JobId,
    ) -> Result<CompactString> {
        let url = {
            let config = self.config.read().unwrap();
            format_compact!(
                "{}/projects/{}/jobs/{}/trace",
                config.base_url,
                project_id,
                job_id
            )
        };

        let response = self.authenticated_request(&url).send().await?;
        let body = response.text().await?;
        Ok(body.into())
    }

    /// Update configuration
    pub fn update_config(&self, config: ClientConfig) -> Result<()> {
        config.validate()?;

        // Create new client with updated timeout
        let client = Client::builder()
            .timeout(config.request.timeout)
            .build()
            .map_err(ClientError::Http)?;

        // Update both config and client atomically
        *self.config.write().unwrap() = config;
        *self.client.write().unwrap() = client;

        Ok(())
    }

    /// Get current configuration
    pub fn config(&self) -> ClientConfig {
        self.config.read().unwrap().clone()
    }

    pub fn is_configured(&self) -> bool {
        self.config
            .read()
            .map(|c| c.validate().is_ok())
            .unwrap_or(false)
    }

    // Private helper methods

    /// Perform authenticated GET request and deserialize JSON response
    async fn get_json<T>(&self, url: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let response = self.authenticated_request(url).send().await?;
        self.handle_response(response).await
    }

    /// Create authenticated request builder
    fn authenticated_request(&self, url: &str) -> RequestBuilder {
        let client = self.client.read().unwrap();
        let private_token = self.config.read().unwrap().private_token.clone();
        client
            .get(url)
            .header("PRIVATE-TOKEN", private_token.as_str())
    }

    /// Handle HTTP response and deserialize JSON
    async fn handle_response<T>(&self, response: Response) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url_path = response.url().path().to_string();
        let status = response.status();
        let body = response.text().await?;

        // Log response if debug is enabled
        {
            let config = self.config.read().unwrap();
            if config.debug.log_responses {
                self.log_response_to_file(&url_path, &body, &config);
            }
        }

        if status.is_success() {
            serde_json::from_str(&body)
                .map_err(|e| ClientError::json_parse(url_path, "Failed to parse response", e))
        } else {
            self.handle_error_response(status.as_u16(), &body)
        }
    }

    /// Handle error responses from GitLab API
    fn handle_error_response<T>(&self, status: u16, body: &str) -> Result<T> {
        match status {
            401 => {
                // Try to parse GitLab API error to distinguish between invalid and expired tokens
                if let Ok(api_error) = serde_json::from_str::<GitlabApiError>(body) {
                    match api_error.error.as_str() {
                        "invalid_token" => Err(ClientError::InvalidToken),
                        "expired_token" => Err(ClientError::ExpiredToken),
                        _ => {
                            // Check error description for expiration indicators
                            if let Some(description) = &api_error.error_description {
                                if description.contains("expired") || description.contains("expiry")
                                {
                                    return Err(ClientError::ExpiredToken);
                                }
                            }
                            Err(ClientError::Authentication)
                        },
                    }
                } else {
                    // Fallback to generic authentication error
                    Err(ClientError::Authentication)
                }
            },
            404 => Err(ClientError::not_found("Resource")),
            429 => {
                // Try to extract retry-after header info if available
                Err(ClientError::rate_limit(None))
            },
            _ => {
                // Try to parse GitLab API error formats
                if let Ok(api_error) = serde_json::from_str::<GitlabApiError>(body) {
                    Err(ClientError::gitlab_api(format_compact!(
                        "HTTP {}: {} {}",
                        status,
                        api_error.error,
                        api_error.error_description.unwrap_or_default()
                    )))
                } else if let Ok(api_error2) = serde_json::from_str::<GitlabApiError2>(body) {
                    Err(ClientError::gitlab_api(format_compact!(
                        "HTTP {}: {}",
                        status,
                        api_error2.message
                    )))
                } else {
                    Err(ClientError::gitlab_api(format_compact!(
                        "HTTP {}: {}",
                        status,
                        body
                    )))
                }
            },
        }
    }

    /// Build URL for projects endpoint
    fn build_projects_url(&self, query: &ProjectQuery) -> CompactString {
        let config = self.config.read().unwrap();
        let mut url = format_compact!("{}/projects?", config.base_url);

        // Add query parameters
        url.push_str("search_namespaces=true");

        if let Some(filter) = &query.search_filter {
            url.push_str(&format_compact!("&search={}", filter));
        }

        if let Some(updated_after) = query.updated_after {
            url.push_str(&format_compact!(
                "&last_activity_after={}",
                updated_after.to_rfc3339()
            ));
        }

        if query.include_statistics {
            url.push_str("&statistics=true");
        }

        if !query.archived {
            url.push_str("&archived=false");
        }

        if query.membership {
            url.push_str("&membership=true");
        }

        url.push_str(&format_compact!("&per_page={}", query.per_page));

        url
    }

    /// Build URL for pipelines endpoint
    fn build_pipelines_url(&self, project_id: ProjectId, query: &PipelineQuery) -> CompactString {
        let config = self.config.read().unwrap();
        let mut url = format_compact!(
            "{}/projects/{}/pipelines?per_page={}",
            config.base_url,
            project_id,
            query.per_page
        );

        if let Some(updated_after) = query.updated_after {
            url.push_str(&format_compact!(
                "&updated_after={}",
                updated_after.to_rfc3339()
            ));
        }

        url
    }

    /// Log HTTP response to file for debugging
    fn log_response_to_file(&self, path: &str, body: &str, config: &ClientConfig) {
        if let Some(log_dir) = &config.debug.log_directory {
            if !log_dir.exists() {
                if let Err(e) = std::fs::create_dir_all(log_dir) {
                    warn!("Failed to create log directory: {}", e);
                    return;
                }
            }

            let filename = format!(
                "{}_{}.json",
                Local::now().format("%Y-%m-%d_%H-%M-%S"),
                path.replace('/', "_")
            );

            let log_path = log_dir.join(filename);

            if let Err(e) = std::fs::write(&log_path, body) {
                warn!("Failed to write response log to {:?}: {}", log_path, e);
            } else {
                debug!("Response logged to {:?}", log_path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;

    impl GitlabApi {
        /// Create a new GitLab API client
        pub fn new(config: ClientConfig) -> Result<Self> {
            config.validate()?;

            let client = Client::builder()
                .timeout(config.request.timeout)
                .build()
                .map_err(ClientError::Http)?;

            Ok(Self {
                client: RwLock::new(client),
                config: RwLock::new(config),
            })
        }
    }

    fn test_config() -> ClientConfig {
        ClientConfig::new("https://gitlab.example.com", "test-token")
    }

    #[test]
    fn test_api_creation() {
        let config = test_config();
        let api = GitlabApi::new(config.clone());
        assert!(api.is_ok());

        let api = api.unwrap();
        assert_eq!(api.config().base_url, config.base_url);
        assert_eq!(api.config().private_token, config.private_token);
    }

    #[test]
    fn test_api_creation_invalid_config() {
        let config = ClientConfig::new("", "test-token");
        let api = GitlabApi::new(config);
        assert!(api.is_err());
    }

    #[test]
    fn test_config_update() {
        let config = test_config();
        let api = GitlabApi::new(config).unwrap();

        let new_config = ClientConfig::new("https://gitlab.new.com", "new-token");
        assert!(api.update_config(new_config.clone()).is_ok());

        let updated_config = api.config();
        assert_eq!(updated_config.base_url, new_config.base_url);
        assert_eq!(updated_config.private_token, new_config.private_token);
    }

    #[test]
    fn test_build_projects_url() {
        let config = test_config().with_search_filter(Some("frontend".into()));
        let api = GitlabApi::new(config).unwrap();

        let mut query = ProjectQuery::default()
            .with_search_filter(Some("frontend".into()))
            .with_per_page(50);
        query.include_statistics = true;
        query.membership = true;
        query.search_namespaces = true;

        let url = api.build_projects_url(&query);

        assert!(url.contains("https://gitlab.example.com/projects?"));
        assert!(url.contains("search_namespaces=true"));
        assert!(url.contains("search=frontend"));
        assert!(url.contains("per_page=50"));
        assert!(url.contains("statistics=true"));
        assert!(url.contains("archived=false"));
        assert!(url.contains("membership=true"));
    }

    #[test]
    fn test_build_pipelines_url() {
        let config = test_config();
        let api = GitlabApi::new(config).unwrap();

        let project_id = ProjectId::new(123);
        let query = PipelineQuery::new().with_per_page(60);

        let url = api.build_pipelines_url(project_id, &query);

        assert_eq!(
            url,
            "https://gitlab.example.com/projects/123/pipelines?per_page=60"
        );
    }

    #[test]
    fn test_build_pipelines_url_with_date() {
        let config = test_config();
        let api = GitlabApi::new(config).unwrap();

        let project_id = ProjectId::new(123);
        let updated_after = DateTime::parse_from_rfc3339("2023-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let query = PipelineQuery::new()
            .with_per_page(60)
            .with_updated_after(Some(updated_after));

        let url = api.build_pipelines_url(project_id, &query);

        assert!(url.contains("updated_after=2023-01-01T00:00:00"));
    }

    #[test]
    fn test_error_handling() {
        let api = GitlabApi::new(test_config()).unwrap();

        // Test authentication error
        let error = api.handle_error_response::<()>(401, "");
        assert!(matches!(error, Err(ClientError::Authentication)));

        // Test not found error
        let error = api.handle_error_response::<()>(404, "");
        assert!(matches!(error, Err(ClientError::NotFound { .. })));

        // Test rate limit error
        let error = api.handle_error_response::<()>(429, "");
        assert!(matches!(error, Err(ClientError::RateLimit { .. })));
    }
}
