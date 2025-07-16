//! Background polling for GitLab resources

use std::{sync::Arc, time::Duration};

use tokio::{sync::broadcast, time::sleep};
use tracing::{debug, error, info, instrument};

use super::{api::GitlabApi, config::PollingConfig, service::GitlabService};
use crate::{dispatcher::Dispatcher, event::GlimEvent};

/// Background poller for GitLab resources
///
/// Manages periodic fetching of projects and active jobs with configurable intervals
#[derive(Debug)]
#[allow(dead_code)]
pub struct GitlabPoller {
    api: Arc<GitlabApi>,
    sender: std::sync::mpsc::Sender<GlimEvent>,
    config: PollingConfig,
    shutdown_tx: broadcast::Sender<()>,
    shutdown_rx: broadcast::Receiver<()>,
}

#[allow(dead_code)]
impl GitlabPoller {
    /// Create a new GitLab poller
    pub fn new(
        api: Arc<GitlabApi>,
        sender: std::sync::mpsc::Sender<GlimEvent>,
        config: PollingConfig,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

        Self { api, sender, config, shutdown_tx, shutdown_rx }
    }

    /// Start polling in the background
    ///
    /// This will spawn two separate async tasks:
    /// - One for polling projects at the configured interval
    /// - One for polling active jobs at the configured interval
    #[instrument(skip(self))]
    pub async fn start(mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            projects_interval = ?self.config.projects_interval,
            jobs_interval = ?self.config.jobs_interval,
            "Starting GitLab poller"
        );

        // Spawn projects polling task
        let projects_task = {
            let api = Arc::clone(&self.api);
            let sender = self.sender.clone();
            let interval = self.config.projects_interval;
            let mut shutdown_rx = self.shutdown_tx.subscribe();

            tokio::spawn(async move {
                Self::poll_projects(api, sender, interval, &mut shutdown_rx).await;
            })
        };

        // Spawn jobs polling task
        let jobs_task = {
            let api = Arc::clone(&self.api);
            let sender = self.sender.clone();
            let interval = self.config.jobs_interval;
            let mut shutdown_rx = self.shutdown_tx.subscribe();

            tokio::spawn(async move {
                Self::poll_active_jobs(api, sender, interval, &mut shutdown_rx).await;
            })
        };

        // Wait for shutdown signal
        let _ = self.shutdown_rx.recv().await;

        info!("Shutting down GitLab poller");

        // Cancel polling tasks
        projects_task.abort();
        jobs_task.abort();

        // Wait a bit for graceful shutdown
        sleep(Duration::from_millis(100)).await;

        Ok(())
    }

    /// Send shutdown signal to stop polling
    pub fn shutdown(&self) {
        debug!("Sending shutdown signal to GitLab poller");
        let _ = self.shutdown_tx.send(());
    }

    /// Get a shutdown sender for external shutdown control
    pub fn shutdown_sender(&self) -> broadcast::Sender<()> {
        self.shutdown_tx.clone()
    }

    /// Update polling configuration
    pub fn update_config(&mut self, config: PollingConfig) {
        self.config = config;
    }

    /// Get current polling configuration
    pub fn config(&self) -> &PollingConfig {
        &self.config
    }

    // Private polling implementations

    /// Poll projects at regular intervals
    #[instrument(skip(api, sender, shutdown_rx), fields(interval = ?interval))]
    async fn poll_projects(
        api: Arc<GitlabApi>,
        sender: std::sync::mpsc::Sender<GlimEvent>,
        interval: Duration,
        shutdown_rx: &mut broadcast::Receiver<()>,
    ) {
        debug!("Starting projects polling loop");

        loop {
            tokio::select! {
                _ = sleep(interval) => {
                    debug!("Polling projects");
                    let service = GitlabService::from_api(api.clone(), sender.clone()).unwrap();
                    service.spawn_fetch_projects(None);
                }
                _ = shutdown_rx.recv() => {
                    debug!("Projects polling received shutdown signal");
                    break;
                }
            }
        }

        debug!("Projects polling loop ended");
    }

    /// Poll active jobs at regular intervals
    #[instrument(skip(_api, sender, shutdown_rx), fields(interval = ?interval))]
    async fn poll_active_jobs(
        _api: Arc<GitlabApi>,
        sender: std::sync::mpsc::Sender<GlimEvent>,
        interval: Duration,
        shutdown_rx: &mut broadcast::Receiver<()>,
    ) {
        debug!("Starting active jobs polling loop");

        loop {
            tokio::select! {
                _ = sleep(interval) => {
                    debug!("Requesting active jobs refresh");
                    // Dispatch event to request active jobs refresh
                    // The main application will handle which jobs to fetch
                    sender.dispatch(GlimEvent::JobsActiveFetch);
                }
                _ = shutdown_rx.recv() => {
                    debug!("Active jobs polling received shutdown signal");
                    break;
                }
            }
        }

        debug!("Active jobs polling loop ended");
    }
}

/// Builder for GitlabPoller with fluent API
#[derive(Debug)]
#[allow(dead_code)]
pub struct GitlabPollerBuilder {
    api: Option<Arc<GitlabApi>>,
    sender: Option<std::sync::mpsc::Sender<GlimEvent>>,
    config: PollingConfig,
}

#[allow(dead_code)]
impl GitlabPollerBuilder {
    /// Create a new poller builder
    pub fn new() -> Self {
        Self {
            api: None,
            sender: None,
            config: PollingConfig::default(),
        }
    }

    /// Set the GitLab API
    pub fn api(mut self, api: Arc<GitlabApi>) -> Self {
        self.api = Some(api);
        self
    }

    /// Set the event sender
    pub fn sender(mut self, sender: std::sync::mpsc::Sender<GlimEvent>) -> Self {
        self.sender = Some(sender);
        self
    }

    /// Set polling configuration
    pub fn config(mut self, config: PollingConfig) -> Self {
        self.config = config;
        self
    }

    /// Set projects polling interval
    pub fn projects_interval(mut self, interval: Duration) -> Self {
        self.config.projects_interval = interval;
        self
    }

    /// Set jobs polling interval
    pub fn jobs_interval(mut self, interval: Duration) -> Self {
        self.config.jobs_interval = interval;
        self
    }

    /// Build the GitLab poller
    pub fn build(self) -> Result<GitlabPoller, String> {
        let api = self.api.ok_or("GitLab API is required")?;
        let sender = self.sender.ok_or("Event sender is required")?;
        Ok(GitlabPoller::new(api, sender, self.config))
    }
}

impl Default for GitlabPollerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Spawn a GitLab poller as a background task
///
/// This is a convenience function for quickly starting background polling
#[allow(dead_code)]
pub async fn spawn_poller(
    api: Arc<GitlabApi>,
    sender: std::sync::mpsc::Sender<GlimEvent>,
    config: PollingConfig,
) -> broadcast::Sender<()> {
    let poller = GitlabPoller::new(api, sender, config);
    let shutdown_sender = poller.shutdown_sender();

    tokio::spawn(async move {
        if let Err(e) = poller.start().await {
            error!("GitLab poller failed: {}", e);
        }
    });

    shutdown_sender
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc, time::Duration};

    use super::*;
    use crate::client::{api::GitlabApi, config::ClientConfig};

    async fn test_api() -> Arc<GitlabApi> {
        let config = ClientConfig::new("https://gitlab.example.com", "test-token");
        let api = GitlabApi::force_new(config).unwrap();
        Arc::new(api)
    }

    fn test_sender() -> std::sync::mpsc::Sender<GlimEvent> {
        let (sender, _receiver) = mpsc::channel();
        sender
    }

    #[tokio::test]
    async fn test_poller_creation() {
        let api = test_api().await;
        let sender = test_sender();
        let config = PollingConfig::default();
        let poller = GitlabPoller::new(api, sender, config);

        assert_eq!(poller.config.projects_interval, Duration::from_secs(60));
        assert_eq!(poller.config.jobs_interval, Duration::from_secs(30));
    }

    #[tokio::test]
    async fn test_poller_builder() {
        let api = test_api().await;
        let sender = test_sender();
        let poller = GitlabPollerBuilder::new()
            .api(api)
            .sender(sender)
            .projects_interval(Duration::from_secs(120))
            .jobs_interval(Duration::from_secs(45))
            .build()
            .unwrap();

        assert_eq!(poller.config.projects_interval, Duration::from_secs(120));
        assert_eq!(poller.config.jobs_interval, Duration::from_secs(45));
    }

    #[test]
    fn test_builder_validation() {
        let result = GitlabPollerBuilder::new()
            .projects_interval(Duration::from_secs(120))
            .build();

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("API is required"));
    }

    #[tokio::test]
    async fn test_poller_shutdown() {
        let api = test_api().await;
        let sender = test_sender();
        let config = PollingConfig {
            projects_interval: Duration::from_millis(10),
            jobs_interval: Duration::from_millis(10),
        };

        let poller = GitlabPoller::new(api, sender, config);
        let shutdown_sender = poller.shutdown_sender();

        // Start poller in background
        let poller_task = tokio::spawn(async move { poller.start().await });

        // Let it run for a bit
        sleep(Duration::from_millis(50)).await;

        // Send shutdown signal
        let _ = shutdown_sender.send(());

        // Wait for poller to shutdown
        let result = tokio::time::timeout(Duration::from_secs(1), poller_task).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_spawn_poller_convenience() {
        let api = test_api().await;
        let sender = test_sender();
        let config = PollingConfig {
            projects_interval: Duration::from_millis(10),
            jobs_interval: Duration::from_millis(10),
        };

        let shutdown_sender = spawn_poller(api, sender, config).await;

        // Let it run for a bit
        sleep(Duration::from_millis(50)).await;

        // Send shutdown signal
        let _ = shutdown_sender.send(());

        // Give it time to shutdown
        sleep(Duration::from_millis(100)).await;
    }
}
