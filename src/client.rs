use std::path::Path;
use std::sync::mpsc::Sender;

use chrono::{DateTime, Local, Utc};
use compact_str::{format_compact, CompactString, ToCompactString};
use itertools::Itertools;
use reqwest::{Client, RequestBuilder};
use serde::Deserialize;
use tokio::runtime::Runtime;
use tokio::time::sleep;

use crate::dispatcher::Dispatcher;
use crate::domain::{JobDto, PipelineDto, ProjectDto};
use crate::event::{GlimEvent, GlitchState, IntoGlimEvent};
use crate::glim_app::GlimConfig;
use crate::id::{JobId, PipelineId, ProjectId};
use crate::result::GlimError::{GeneralError, JsonDeserializeError};
use crate::result::*;
use tracing::{debug, error, info, instrument, warn};

pub struct GitlabClient {
    sender: Sender<GlimEvent>,
    base_url: CompactString,
    private_token: CompactString,
    client: Client,
    search_filter: Option<CompactString>,
    log_response: bool,
    rt: Runtime,
}

impl GitlabClient {
    pub fn new(
        sender: Sender<GlimEvent>,
        host: CompactString,
        private_token: CompactString,
        search_filter: Option<CompactString>,
        debug: bool,
    ) -> Self {
        let client = Self {
            sender,
            base_url: host,
            private_token,
            client: Client::new(),
            search_filter,
            rt: Runtime::new().unwrap(),
            log_response: debug,
        };
        client.register_polling();
        client
    }

    pub fn update_config(&mut self, config: GlimConfig) {
        self.base_url = config.gitlab_url;
        self.private_token = config.gitlab_token;
        self.search_filter = config.search_filter;
    }

    pub fn debug(&self) -> bool {
        self.log_response
    }

    pub fn new_from_config(sender: Sender<GlimEvent>, config: GlimConfig, debug: bool) -> Self {
        Self::new(
            sender,
            config.gitlab_url,
            config.gitlab_token,
            config.search_filter,
            debug,
        )
    }

    #[instrument(skip(self), fields(project_id = %project_id, job_id = %job_id))]
    pub fn dispatch_download_job_log(&self, project_id: ProjectId, job_id: JobId) {
        info!("Downloading job log from GitLab");
        let get_trace_request = self
            .client
            .get(format_compact!(
                "{}/projects/{project_id}/jobs/{job_id}/trace",
                self.base_url
            ).as_str())
            .header("PRIVATE-TOKEN", self.private_token.as_str());

        let sender = self.sender.clone();
        self.rt.spawn(async move {
            let event = Self::http_request(get_trace_request)
                .await
                .map(|trace| GlimEvent::JobLogDownloaded(project_id, job_id, trace))
                .unwrap_or_else(GlimEvent::Error);

            sender.dispatch(event)
        });
    }

    #[instrument(skip(self), fields(project_id = %project_id, pipeline_id = %pipeline_id))]
    pub fn dispatch_get_jobs(&self, project_id: ProjectId, pipeline_id: PipelineId) {
        debug!("Fetching jobs for pipeline");
        let base_url = format_compact!(
            "{}/projects/{project_id}/pipelines/{pipeline_id}",
            self.base_url
        );

        let get_jobs_request = self
            .client
            .get(format_compact!("{base_url}/jobs").as_str())
            .header("PRIVATE-TOKEN", self.private_token.as_str());
        let get_trigger_jobs_request = self
            .client
            .get(format_compact!("{base_url}/bridges").as_str())
            .header("PRIVATE-TOKEN", self.private_token.as_str());

        let sender = self.sender.clone();

        let debug = self.log_response;
        self.rt.spawn(async move {
            let jobs = match Self::http_json_request::<Vec<JobDto>>(get_jobs_request, debug).await {
                Ok(t) => t,
                Err(e) => {
                    error!(error = %e, project_id = %project_id, pipeline_id = %pipeline_id, "Failed to fetch jobs");
                    GlimError::GitlabGetJobsError(project_id, pipeline_id, e.to_compact_string());
                    return sender.dispatch(GlimEvent::Error(e));
                }
            };

            let triggered_jobs =
                match Self::http_json_request::<Vec<JobDto>>(get_trigger_jobs_request, debug).await
                {
                    Ok(t) => t,
                    Err(e) => {
                        warn!(error = %e, project_id = %project_id, pipeline_id = %pipeline_id, "Failed to fetch trigger jobs");
                        return sender.dispatch(GlimEvent::Error(e));
                    }
                };

            // combine jobs, sorted by id
            let jobs = jobs
                .into_iter()
                .chain(triggered_jobs.into_iter())
                .sorted_by_key(|j| j.id)
                .collect::<Vec<JobDto>>();

            debug!(job_count = jobs.len(), "Successfully fetched jobs");
            sender.dispatch((project_id, pipeline_id, jobs).into_glim_event())
        });
    }

    pub fn dispatch_get_pipelines(&self, id: ProjectId, updated_after: Option<DateTime<Utc>>) {
        let mut url = format_compact!("{}/projects/{id}/pipelines?per_page=60", self.base_url);
        if let Some(date) = updated_after {
            url.push_str(&format_compact!("?last_activity_after={}", date.to_rfc3339()));
        }

        self.dispatch::<Vec<PipelineDto>>(&url);
    }

    #[instrument(skip(self))]
    pub fn dispatch_list_projects(&self, updated_after: Option<DateTime<Utc>>) {
        info!(updated_after = ?updated_after, "Fetching projects from GitLab");
        self.dispatch_glitchy::<Vec<ProjectDto>>(&self.list_projects_url(updated_after, 100))
    }

    pub fn validate_configuration(&self) -> Result<()> {
        let url = self.list_projects_url(None, 1);
        let request = self.authenticated_get(&url);
        let debug = self.log_response;

        let response = self
            .rt
            .block_on(Self::http_json_request::<serde_json::Value>(request, debug))?;
        if response.is_array() {
            Ok(())
        } else {
            Err(GeneralError(format_compact!("Invalid configuration: {}", response)))
        }
    }

    fn list_projects_url(
        &self,
        updated_after: Option<DateTime<Utc>>,
        result_per_page: u8,
    ) -> CompactString {
        format_compact!(
            "{}/projects?search_namespaces=true{}{}&statistics=true&archived=false&membership=true&per_page={result_per_page}",
            self.base_url,
            self.search_filter.as_ref().map_or("".into(), |f| format_compact!("&search={}", f)),
            updated_after.map_or("".into(), |d| format_compact!("&last_activity_after={}", d.to_rfc3339())),
        )
    }

    fn register_polling(&self) {
        let sender = self.sender.clone();
        self.rt.spawn(async move {
            loop {
                sleep(std::time::Duration::from_secs(30)).await;
                sender.dispatch(GlimEvent::RequestActiveJobs);
                sleep(std::time::Duration::from_secs(30)).await;
                sender.dispatch(GlimEvent::RequestProjects);
            }
        });
    }

    /// Performs requests against the Gitlab API. Results are sent
    /// as [GlimEvent]s using [self.sender].
    fn dispatch<T>(&self, url: &str)
    where
        T: for<'de> Deserialize<'de> + IntoGlimEvent,
    {
        let request = self.authenticated_get(url);
        let sender = self.sender.clone();
        let debug = self.log_response;

        self.rt.spawn(async move {
            let event = match Self::http_json_request::<T>(request, debug).await {
                Ok(t) => t.into_glim_event(),
                Err(e) => GlimEvent::Error(e),
            };
            sender.dispatch(event)
        });
    }

    /// Performs requests against the Gitlab API. Results are sent
    /// as [GlimEvent]s using [self.sender].
    fn dispatch_glitchy<T>(&self, url: &str)
    where
        T: for<'de> Deserialize<'de> + IntoGlimEvent,
    {
        let request = self.authenticated_get(url);
        let sender = self.sender.clone();
        let debug = self.log_response;

        self.rt.spawn(async move {
            sender.dispatch(GlimEvent::GlitchOverride(GlitchState::Active));
            sleep(std::time::Duration::from_millis(400)).await;

            let event = match Self::http_json_request::<T>(request, debug).await {
                Ok(t) => t.into_glim_event(),
                Err(e) => GlimEvent::Error(e),
            };
            sender.dispatch(GlimEvent::GlitchOverride(GlitchState::Inactive));
            sender.dispatch(event)
        });
    }

    fn authenticated_get(&self, url: &str) -> RequestBuilder {
        self.client
            .get(url)
            .header("PRIVATE-TOKEN", self.private_token.as_str())
    }

    async fn http_json_request<T>(request: RequestBuilder, debug: bool) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let response = request.send().await?;
        let path = response.url().path().into();

        let status = response.status();
        let body = response.text().await?.into();

        if debug {
            Self::log_response_to_file(path, &body);
        }

        if status.is_success() {
            serde_json::from_str(&body).map_err(|e| JsonDeserializeError(e.classify(), body.clone()))
        } else {
            let api = serde_json::from_str::<GitlabApiError>(&body);
            if let Ok(api) = api {
                Err(GeneralError(format_compact!(
                    "HTTP {}\n {}",
                    api.error,
                    api.description()
                )))
            } else if let Ok(api2) = serde_json::from_str::<GitlabApiError2>(&body) {
                Err(GeneralError(format_compact!("HTTP {}", api2.message)))
            } else {
                Err(GeneralError(format_compact!("{}: {}", status, body)))
            }
        }
    }

    fn log_response_to_file(path: CompactString, body: &CompactString) {
        if !Path::new("glim-logs").exists() {
            std::fs::create_dir("glim-logs").expect("Unable to create directory");
        }

        let filename = format!(
            "glim-logs/{}_{}.json",
            Local::now().format("%Y-%m-%d_%H-%M-%S"),
            path.replace('/', "_"),
        );

        std::fs::write(filename, body).expect("Unable to write to file");
    }

    async fn http_request(request: RequestBuilder) -> Result<CompactString> {
        let body = request.send().await?.text().await?;

        Ok(body.into())
    }
}

#[derive(Debug, Deserialize)]
struct GitlabApiError {
    error: CompactString,
    error_description: Option<CompactString>,
}

#[derive(Debug, Deserialize)]
struct GitlabApiError2 {
    message: CompactString,
}

impl GitlabApiError {
    pub fn description(&self) -> CompactString {
        self.error_description.clone().unwrap_or("".into())
    }
}
