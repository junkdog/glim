//! Integration tests for GitLab client modules

use std::{sync::Arc, time::Duration};

use tokio::time::timeout;
use wiremock::{
    matchers::{header, method, path, query_param},
    Mock, ResponseTemplate,
};

use crate::{
    client::{
        api::GitlabApi,
        config::{ClientConfig, PipelineQuery, ProjectQuery},
        poller::GitlabPollerBuilder,
        service::GitlabService,
    },
    id::{JobId, PipelineId, ProjectId},
};

use super::{
    gitlab_error_response, jobs_json_response, pipelines_json_response, projects_json_response,
    test_polling_config, MockServer,
};

#[tokio::test]
async fn test_api_get_projects_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/projects"))
        .and(header("PRIVATE-TOKEN", "test-token"))
        .and(query_param("search_namespaces", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&projects_json_response()))
        .mount(&mock_server.server)
        .await;

    let config = mock_server.test_config();
    let api = GitlabApi::new(config).unwrap();

    let query = ProjectQuery::new().with_per_page(100);
    let projects = api.get_projects(&query).await.unwrap();

    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].id, ProjectId::new(123));
    assert_eq!(projects[0].path_with_namespace, "group/project");
}

#[tokio::test]
async fn test_api_get_projects_authentication_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/projects"))
        .and(header("PRIVATE-TOKEN", "test-token"))
        .respond_with(
            ResponseTemplate::new(401).set_body_json(&gitlab_error_response("invalid_token", None)),
        )
        .mount(&mock_server.server)
        .await;

    let config = mock_server.test_config();
    let api = GitlabApi::new(config).unwrap();

    let query = ProjectQuery::new();
    let result = api.get_projects(&query).await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        crate::client::error::ClientError::Authentication
    ));
}

#[tokio::test]
async fn test_api_get_pipelines_success() {
    let mock_server = MockServer::start().await;
    let project_id = ProjectId::new(123);

    Mock::given(method("GET"))
        .and(path(format!("/projects/{}/pipelines", project_id)))
        .and(header("PRIVATE-TOKEN", "test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&pipelines_json_response()))
        .mount(&mock_server.server)
        .await;

    let config = mock_server.test_config();
    let api = GitlabApi::new(config).unwrap();

    let query = PipelineQuery::new().with_per_page(60);
    let pipelines = api.get_pipelines(project_id, &query).await.unwrap();

    assert_eq!(pipelines.len(), 1);
    assert_eq!(pipelines[0].id, PipelineId::new(456));
    assert_eq!(pipelines[0].project_id, project_id);
}

#[tokio::test]
async fn test_api_get_jobs_success() {
    let mock_server = MockServer::start().await;
    let project_id = ProjectId::new(123);
    let pipeline_id = PipelineId::new(456);

    // Mock both jobs and bridges endpoints
    Mock::given(method("GET"))
        .and(path(format!(
            "/projects/{}/pipelines/{}/jobs",
            project_id, pipeline_id
        )))
        .and(header("PRIVATE-TOKEN", "test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&jobs_json_response()))
        .mount(&mock_server.server)
        .await;

    Mock::given(method("GET"))
        .and(path(format!(
            "/projects/{}/pipelines/{}/bridges",
            project_id, pipeline_id
        )))
        .and(header("PRIVATE-TOKEN", "test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&serde_json::json!([])))
        .mount(&mock_server.server)
        .await;

    let config = mock_server.test_config();
    let api = GitlabApi::new(config).unwrap();

    let jobs = api.get_jobs(project_id, pipeline_id).await.unwrap();

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].id, JobId::new(789));
    assert_eq!(jobs[0].name, "test-job");
}

#[tokio::test]
async fn test_api_get_job_trace_success() {
    let mock_server = MockServer::start().await;
    let project_id = ProjectId::new(123);
    let job_id = JobId::new(789);

    let trace_content = "Build started\nRunning tests\nBuild completed\n";

    Mock::given(method("GET"))
        .and(path(format!("/projects/{}/jobs/{}/trace", project_id, job_id)))
        .and(header("PRIVATE-TOKEN", "test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(trace_content))
        .mount(&mock_server.server)
        .await;

    let config = mock_server.test_config();
    let api = GitlabApi::new(config).unwrap();

    let trace = api.get_job_trace(project_id, job_id).await.unwrap();

    assert_eq!(trace, trace_content);
}

#[tokio::test]
async fn test_api_validate_connection_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/projects"))
        .and(header("PRIVATE-TOKEN", "test-token"))
        .and(query_param("per_page", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&projects_json_response()))
        .mount(&mock_server.server)
        .await;

    let config = mock_server.test_config();
    let api = GitlabApi::new(config).unwrap();

    let result = api.validate_connection().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_service_fetch_projects_with_events() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/projects"))
        .and(header("PRIVATE-TOKEN", "test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&projects_json_response()))
        .mount(&mock_server.server)
        .await;

    let config = mock_server.test_config();
    let (sender, receiver) = std::sync::mpsc::channel();
    let service = GitlabService::new(config, sender).unwrap();

    let result = service.fetch_projects(None).await;
    assert!(result.is_ok());

    // Check that an event was sent
    let event = receiver.try_recv();
    assert!(event.is_ok());
    // Event should be GlimEvent::ProjectsUpdated but we can't easily check
    // the exact type here without more complex setup
}

#[tokio::test]
async fn test_service_error_handling() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/projects"))
        .and(header("PRIVATE-TOKEN", "test-token"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server.server)
        .await;

    let config = mock_server.test_config();
    let (sender, receiver) = std::sync::mpsc::channel();
    let service = GitlabService::new(config, sender).unwrap();

    let result = service.fetch_projects(None).await;
    assert!(result.is_err());

    // Check that an error event was sent
    let event = receiver.try_recv();
    assert!(event.is_ok());
    // Should be GlimEvent::Error
}

#[tokio::test]
async fn test_poller_lifecycle() {
    let mock_server = MockServer::start().await;

    // Mock the requests that will be made during polling
    Mock::given(method("GET"))
        .and(path("/projects"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&projects_json_response()))
        .mount(&mock_server.server)
        .await;

    let config = mock_server.test_config();
    let (sender, receiver) = std::sync::mpsc::channel();
    let service = Arc::new(GitlabService::new(config, sender).unwrap());

    let poller = GitlabPollerBuilder::new()
        .service(service)
        .config(test_polling_config())
        .build()
        .unwrap();

    let shutdown_sender = poller.shutdown_sender();

    // Start poller in background
    let poller_task = tokio::spawn(async move {
        poller.start().await
    });

    // Let it run for a short time
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Send shutdown signal
    let _ = shutdown_sender.send(());

    // Wait for poller to shutdown
    let result = timeout(Duration::from_secs(1), poller_task).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_ok());

    // Check that we received some events during polling
    let mut event_count = 0;
    while receiver.try_recv().is_ok() {
        event_count += 1;
    }
    
    // We should have received at least some events during the polling period
    assert!(event_count > 0);
}

#[tokio::test]
async fn test_config_validation_in_integration() {
    // Test invalid URL
    let result = GitlabApi::new(ClientConfig::new("not-a-url", "token"));
    assert!(result.is_err());

    // Test empty token
    let result = GitlabApi::new(ClientConfig::new("https://gitlab.com", ""));
    assert!(result.is_err());

    // Test valid config
    let result = GitlabApi::new(ClientConfig::new("https://gitlab.com", "token"));
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_url_building_with_query_params() {
    let mock_server = MockServer::start().await;

    // Test with search filter and updated_after
    Mock::given(method("GET"))
        .and(path("/projects"))
        .and(query_param("search", "frontend"))
        .and(query_param("search_namespaces", "true"))
        .and(query_param("statistics", "true"))
        .and(query_param("archived", "false"))
        .and(query_param("membership", "true"))
        .and(query_param("per_page", "50"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&projects_json_response()))
        .mount(&mock_server.server)
        .await;

    let config = mock_server.test_config();
    let api = GitlabApi::new(config).unwrap();

    let query = ProjectQuery::new()
        .with_search_filter(Some("frontend".into()))
        .with_per_page(50);

    let result = api.get_projects(&query).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_concurrent_requests() {
    let mock_server = MockServer::start().await;

    // Mock multiple endpoints
    Mock::given(method("GET"))
        .and(path("/projects"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&projects_json_response()))
        .mount(&mock_server.server)
        .await;

    Mock::given(method("GET"))
        .and(path("/projects/123/pipelines"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&pipelines_json_response()))
        .mount(&mock_server.server)
        .await;

    let config = mock_server.test_config();
    let api = GitlabApi::new(config).unwrap();

    // Make concurrent requests
    let (projects_result, pipelines_result) = tokio::join!(
        api.get_projects(&ProjectQuery::new()),
        api.get_pipelines(ProjectId::new(123), &PipelineQuery::new())
    );

    assert!(projects_result.is_ok());
    assert!(pipelines_result.is_ok());
    assert_eq!(projects_result.unwrap().len(), 1);
    assert_eq!(pipelines_result.unwrap().len(), 1);
}