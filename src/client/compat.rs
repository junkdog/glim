//! Compatibility layer for the old GitlabClient interface
//!
//! This module provides a compatibility wrapper that maintains the same
//! public API as the original GitlabClient while using the new modular
//! client architecture underneath.

use std::sync::{mpsc::Sender, Arc};

use chrono::{DateTime, Utc};
use compact_str::CompactString;
use tokio::runtime::Runtime;

use super::{config::ClientConfig, poller::GitlabPoller, service::GitlabService};
use crate::{
    event::GlimEvent,
    glim_app::GlimConfig,
    id::{JobId, PipelineId, ProjectId},
    result::Result,
};

/// Compatibility wrapper for the old GitlabClient interface
///
/// This struct maintains the same public API as the original GitlabClient
/// but uses the new modular client architecture underneath.
pub struct GitlabClient {
    service: Arc<GitlabService>,
    log_response: bool,
    rt: Runtime,
    _poller_shutdown: Option<tokio::sync::broadcast::Sender<()>>,
}

impl GitlabClient {
    /// Create a new GitLab client (compatibility constructor)
    pub fn new(
        sender: Sender<GlimEvent>,
        host: CompactString,
        private_token: CompactString,
        search_filter: Option<CompactString>,
        debug: bool,
    ) -> Self {
        let config = ClientConfig::new(host, private_token)
            .with_search_filter(search_filter)
            .with_debug_logging(debug);

        let service =
            Arc::new(GitlabService::new(config, sender).expect("Failed to create GitLab service"));

        let rt = Runtime::new().expect("Failed to create Tokio runtime");

        let mut client = Self {
            service,
            log_response: debug,
            rt,
            _poller_shutdown: None,
        };

        client.register_polling();
        client
    }

    /// Create client from existing config (compatibility constructor)
    pub fn new_from_config(sender: Sender<GlimEvent>, config: GlimConfig, debug: bool) -> Self {
        Self::new(sender, config.gitlab_url, config.gitlab_token, config.search_filter, debug)
    }

    /// Update configuration (compatibility method)
    pub fn update_config(&mut self, config: GlimConfig) {
        let client_config = ClientConfig::from(config).with_debug_logging(self.log_response);

        // Note: This creates a new service instance since the config changed
        if let Ok(new_service) = GitlabService::new(client_config, self.service.sender().clone()) {
            self.service = Arc::new(new_service);
        }
    }

    /// Check if debug logging is enabled (compatibility method)
    pub fn debug(&self) -> bool {
        self.log_response
    }

    /// Download job log (compatibility method)
    pub fn dispatch_download_job_log(&self, project_id: ProjectId, job_id: JobId) {
        let service = Arc::clone(&self.service);
        self.rt.spawn(async move {
            if let Err(e) = service.download_job_log(project_id, job_id).await {
                tracing::warn!("Background job log download failed: {}", e);
            }
        });
    }

    /// Get jobs for a pipeline (compatibility method)
    pub fn dispatch_get_jobs(&self, project_id: ProjectId, pipeline_id: PipelineId) {
        let service = Arc::clone(&self.service);
        self.rt.spawn(async move {
            if let Err(e) = service
                .fetch_all_jobs(project_id, pipeline_id)
                .await
            {
                tracing::warn!("Background job fetch failed: {}", e);
            }
        });
    }

    /// Get pipelines for a project (compatibility method)
    pub fn dispatch_get_pipelines(&self, id: ProjectId, updated_after: Option<DateTime<Utc>>) {
        let service = Arc::clone(&self.service);
        self.rt.spawn(async move {
            if let Err(e) = service.fetch_pipelines(id, updated_after).await {
                tracing::warn!("Background pipeline fetch failed: {}", e);
            }
        });
    }

    /// List projects (compatibility method)
    pub fn dispatch_list_projects(&self, updated_after: Option<DateTime<Utc>>) {
        let service = Arc::clone(&self.service);
        self.rt.spawn(async move {
            if let Err(e) = service.fetch_projects(updated_after).await {
                tracing::warn!("Background project fetch failed: {}", e);
            }
        });
    }

    /// Validate configuration (compatibility method)
    pub fn validate_configuration(&self) -> Result<()> {
        self.rt.block_on(async {
            self.service
                .validate_connection()
                .await
                .map_err(|e| crate::result::GlimError::from(&e))
        })
    }

    /// Register polling (compatibility method)
    fn register_polling(&mut self) {
        let service = Arc::clone(&self.service);
        let polling_config = self.service.config().polling.clone();

        let poller = GitlabPoller::new(service, polling_config);
        let shutdown_sender = poller.shutdown_sender();

        // Spawn the poller task on our runtime
        self.rt.spawn(async move {
            if let Err(e) = poller.start().await {
                tracing::error!("GitLab poller failed: {}", e);
            }
        });

        self._poller_shutdown = Some(shutdown_sender);
    }

    /// Get reference to the underlying service (for advanced usage)
    pub fn service(&self) -> &GitlabService {
        &self.service
    }

    /// Get reference to the Tokio runtime (for advanced usage)
    pub fn runtime(&self) -> &Runtime {
        &self.rt
    }
}

impl Drop for GitlabClient {
    fn drop(&mut self) {
        // Send shutdown signal to poller if it exists
        if let Some(shutdown_sender) = &self._poller_shutdown {
            let _ = shutdown_sender.send(());
        }
    }
}
