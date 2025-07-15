//! High-level GitLab service operations

use std::sync::{mpsc::Sender, Arc};

use chrono::{DateTime, Utc};
use tokio::runtime::Handle;
use tracing::{debug, error, info, instrument, warn};

use super::{
    api::GitlabApi,
    config::ClientConfig,
    error::{ClientError, Result},
};
use crate::{
    dispatcher::Dispatcher,
    event::{GlimEvent, IntoGlimEvent},
    id::{JobId, PipelineId, ProjectId},
    result::GlimError::GeneralError,
};

/// High-level service for GitLab operations
///
/// Orchestrates API calls and handles event dispatching to the application
#[derive(Debug)]
pub struct GitlabService {
    api: Arc<GitlabApi>,
    sender: Sender<GlimEvent>,
    handle: Handle,
}

impl GitlabService {
    /// Create a new GitLab service
    pub fn new(config: ClientConfig, sender: Sender<GlimEvent>) -> Result<Self> {
        let api = Arc::new(GitlabApi::new(config)?);
        let handle = Handle::try_current().map_err(|_| {
            ClientError::config("GitlabService must be created within a Tokio runtime context")
        })?;
        Ok(Self { api, sender, handle })
    }

    /// Create service from existing API client
    pub fn from_api(api: Arc<GitlabApi>, sender: Sender<GlimEvent>) -> Result<Self> {
        let handle = Handle::try_current().map_err(|_| {
            ClientError::config("GitlabService must be created within a Tokio runtime context")
        })?;
        Ok(Self { api, sender, handle })
    }

    /// Fetch projects and dispatch results as events
    #[instrument(skip(self), fields(updated_after = ?updated_after))]
    pub async fn fetch_projects(&self, updated_after: Option<DateTime<Utc>>) -> Result<()> {
        if !self.api.is_configured() {
            return Ok(());
        }

        info!("Fetching projects from GitLab");

        let query = self
            .api
            .config()
            .default_project_query()
            .with_updated_after(updated_after);

        match self.api.get_projects(&query).await {
            Ok(projects) => {
                debug!(project_count = projects.len(), "Successfully fetched projects");
                self.sender.dispatch(projects.into_glim_event());
                Ok(())
            },
            Err(e) => {
                error!(error = %e, "Failed to fetch projects");
                let glim_error = crate::result::GlimError::from(&e);
                self.sender
                    .dispatch(GlimEvent::AppError(glim_error));
                Err(e)
            },
        }
    }

    /// Fetch pipelines for a project and dispatch results as events
    #[instrument(skip(self), fields(project_id = %project_id, updated_after = ?updated_after))]
    pub async fn fetch_pipelines(
        &self,
        project_id: ProjectId,
        updated_after: Option<DateTime<Utc>>,
    ) -> Result<()> {
        if !self.api.is_configured() {
            return Ok(());
        }

        let query = self
            .api
            .config()
            .default_pipeline_query()
            .with_updated_after(updated_after);

        match self.api.get_pipelines(project_id, &query).await {
            Ok(pipelines) => {
                debug!(
                    pipeline_count = pipelines.len(),
                    project_id = %project_id,
                    "Successfully fetched pipelines"
                );
                self.sender.dispatch(pipelines.into_glim_event());
                Ok(())
            },
            Err(e) => {
                error!(
                    error = %e,
                    project_id = %project_id,
                    "Failed to fetch pipelines"
                );
                let glim_error = crate::result::GlimError::from(&e);
                self.sender
                    .dispatch(GlimEvent::AppError(glim_error));
                Err(e)
            },
        }
    }

    /// Fetch all jobs (regular + trigger jobs) for a pipeline and dispatch results
    #[instrument(skip(self), fields(project_id = %project_id, pipeline_id = %pipeline_id))]
    pub async fn fetch_all_jobs(
        &self,
        project_id: ProjectId,
        pipeline_id: PipelineId,
    ) -> Result<()> {
        if !self.api.is_configured() {
            return Ok(());
        }

        match self.api.get_jobs(project_id, pipeline_id).await {
            Ok(jobs) => {
                debug!(
                    job_count = jobs.len(),
                    project_id = %project_id,
                    pipeline_id = %pipeline_id,
                    "Successfully fetched jobs"
                );
                self.sender
                    .dispatch((project_id, pipeline_id, jobs).into_glim_event());
                Ok(())
            },
            Err(e) => {
                error!(
                    error = %e,
                    project_id = %project_id,
                    pipeline_id = %pipeline_id,
                    "Failed to fetch jobs"
                );
                let glim_error = crate::result::GlimError::from(&e);
                self.sender
                    .dispatch(GlimEvent::AppError(glim_error));
                Err(e)
            },
        }
    }

    /// Download job log and dispatch results
    #[instrument(skip(self), fields(project_id = %project_id, job_id = %job_id))]
    pub async fn download_job_log(&self, project_id: ProjectId, job_id: JobId) -> Result<()> {
        if !self.api.is_configured() {
            return Ok(());
        }

        info!("Downloading job log from GitLab");

        match self.api.get_job_trace(project_id, job_id).await {
            Ok(trace) => {
                info!(
                    project_id = %project_id,
                    job_id = %job_id,
                    trace_length = trace.len(),
                    "Successfully downloaded job log"
                );
                self.sender
                    .dispatch(GlimEvent::JobLogDownloaded(project_id, job_id, trace));
                Ok(())
            },
            Err(e) => {
                error!(
                    error = %e,
                    project_id = %project_id,
                    job_id = %job_id,
                    "Failed to download job log"
                );
                let glim_error = crate::result::GlimError::from(&e);
                self.sender
                    .dispatch(GlimEvent::AppError(glim_error));
                Err(e)
            },
        }
    }

    /// Validate GitLab connection and credentials
    #[instrument(skip(self))]
    pub async fn validate_connection(&self) -> Result<()> {
        info!("Validating GitLab connection");

        match self.api.validate_connection().await {
            Ok(()) => {
                info!("GitLab connection validation successful");
                Ok(())
            },
            Err(e) => {
                error!(error = %e, "GitLab connection validation failed");
                Err(e)
            },
        }
    }

    /// Update service configuration
    pub fn update_config(&self, config: ClientConfig) -> Result<()> {
        self.api.update_config(config)
    }

    /// Get current configuration
    pub fn config(&self) -> ClientConfig {
        self.api.config()
    }

    /// Get reference to the underlying API client
    #[allow(dead_code)]
    pub fn api(&self) -> &GitlabApi {
        &self.api
    }

    /// Get reference to the event sender
    pub fn sender(&self) -> &Sender<GlimEvent> {
        &self.sender
    }

    /// Spawn an async task to fetch projects
    ///
    /// This is a convenience method for fire-and-forget project fetching
    pub fn spawn_fetch_projects(&self, updated_after: Option<DateTime<Utc>>) {
        let api = self.api.clone();
        let sender = self.sender.clone();
        self.handle.spawn(async move {
            let temp_service = Self::from_api(api, sender).unwrap();
            if let Err(e) = temp_service.fetch_projects(updated_after).await {
                warn!("Background project fetch failed: {}", e);
            }
        });
    }

    /// Spawn an async task to fetch pipelines
    pub fn spawn_fetch_pipelines(
        &self,
        project_id: ProjectId,
        updated_after: Option<DateTime<Utc>>,
    ) {
        let api = self.api.clone();
        let sender = self.sender.clone();
        self.handle.spawn(async move {
            let temp_service = Self::from_api(api, sender).unwrap();
            if let Err(e) = temp_service
                .fetch_pipelines(project_id, updated_after)
                .await
            {
                warn!("Background pipeline fetch failed: {}", e);
            }
        });
    }

    /// Spawn an async task to fetch jobs
    pub fn spawn_fetch_jobs(&self, project_id: ProjectId, pipeline_id: PipelineId) {
        let api = self.api.clone();
        let sender = self.sender.clone();
        self.handle.spawn(async move {
            let temp_service = Self::from_api(api, sender).unwrap();
            if let Err(e) = temp_service
                .fetch_all_jobs(project_id, pipeline_id)
                .await
            {
                warn!("Background job fetch failed: {}", e);
            }
        });
    }

    /// Spawn an async task to download job log
    pub fn spawn_download_job_log(&self, project_id: ProjectId, job_id: JobId) {
        let api = self.api.clone();
        let sender = self.sender.clone();
        self.handle.spawn(async move {
            let temp_service = Self::from_api(api, sender).unwrap();
            if let Err(e) = temp_service
                .download_job_log(project_id, job_id)
                .await
            {
                warn!("Background job log download failed: {}", e);
            }
        });
    }
}

// Convert ClientError to the application's GlimError type
impl From<&ClientError> for crate::result::GlimError {
    fn from(err: &ClientError) -> Self {
        match err {
            ClientError::Http(e) => GeneralError(format!("HTTP error: {e}").into()),
            ClientError::JsonParse { message, .. } => GeneralError(message.clone().into()),
            ClientError::GitlabApi { message } => GeneralError(message.clone()),
            ClientError::Config(msg) => GeneralError(msg.into()),
            ClientError::Authentication => GeneralError("Authentication failed".into()),
            ClientError::Timeout => GeneralError("Request timeout".into()),
            ClientError::InvalidUrl { url } => GeneralError(format!("Invalid URL: {url}").into()),
            ClientError::NotFound { resource } => {
                GeneralError(format!("Not found: {resource}").into())
            },
            ClientError::RateLimit { .. } => GeneralError("Rate limit exceeded".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;
    use crate::client::config::ClientConfig;

    fn test_config() -> ClientConfig {
        ClientConfig::new("https://gitlab.example.com", "test-token")
    }

    #[tokio::test]
    async fn test_service_creation() {
        let config = test_config();
        let (sender, _receiver) = mpsc::channel();
        let service = GitlabService::new(config, sender);
        assert!(service.is_ok());
    }

    #[tokio::test]
    async fn test_service_creation_invalid_config() {
        let config = ClientConfig::new("", "test-token");
        let (sender, _receiver) = mpsc::channel();
        let service = GitlabService::new(config, sender);
        assert!(service.is_err());
    }

    #[tokio::test]
    async fn test_config_access() {
        let config = test_config();
        let (sender, _receiver) = mpsc::channel();
        let service = GitlabService::new(config.clone(), sender).unwrap();

        assert_eq!(service.config().base_url, config.base_url);
        assert_eq!(service.config().private_token, config.private_token);
    }

    #[tokio::test]
    async fn test_error_conversion() {
        let client_error = ClientError::config("Test error");
        let glim_error: crate::result::GlimError = (&client_error).into();

        match glim_error {
            GeneralError(msg) => {
                assert!(msg.contains("Test error"));
            },
            _ => panic!("Expected GeneralError"),
        }
    }

    // Note: Integration tests with actual API calls would require
    // a test GitLab instance or mocked HTTP responses
}
