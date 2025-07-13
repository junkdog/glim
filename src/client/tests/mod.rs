//! Test utilities and common test fixtures for client modules

use chrono::{DateTime, Utc};
use serde_json::json;

use crate::{
    domain::{CommitDto, JobDto, PipelineDto, ProjectDto, StatisticsDto},
    id::{JobId, PipelineId, ProjectId},
};

/// Create a sample ProjectDto for testing
pub fn sample_project_dto() -> ProjectDto {
    ProjectDto {
        id: ProjectId::new(123),
        path_with_namespace: "group/project".into(),
        description: Some("Test project".into()),
        default_branch: "main".into(),
        ssh_url_to_repo: "git@gitlab.example.com:group/project.git".into(),
        web_url: "https://gitlab.example.com/group/project".into(),
        last_activity_at: DateTime::parse_from_rfc3339("2023-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        statistics: StatisticsDto {
            commit_count: 42,
            job_artifacts_size: 1024 * 1024,
            repository_size: 10 * 1024 * 1024,
        },
    }
}

/// Create a sample PipelineDto for testing
pub fn sample_pipeline_dto() -> PipelineDto {
    PipelineDto {
        id: PipelineId::new(456),
        project_id: ProjectId::new(123),
        status: crate::domain::PipelineStatus::Success,
        source: crate::domain::PipelineSource::Push,
        branch: "main".into(),
        web_url: "https://gitlab.example.com/group/project/-/pipelines/456".into(),
        created_at: DateTime::parse_from_rfc3339("2023-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339("2023-01-01T01:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
    }
}

/// Create a sample JobDto for testing
pub fn sample_job_dto() -> JobDto {
    JobDto {
        id: JobId::new(789),
        name: "test-job".into(),
        stage: "test".into(),
        commit: CommitDto {
            title: "Test commit".into(),
            author_name: "Test Author".into(),
        },
        status: crate::domain::PipelineStatus::Success,
        created_at: DateTime::parse_from_rfc3339("2023-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        started_at: Some(
            DateTime::parse_from_rfc3339("2023-01-01T00:01:00Z")
                .unwrap()
                .with_timezone(&Utc),
        ),
        finished_at: Some(
            DateTime::parse_from_rfc3339("2023-01-01T00:05:00Z")
                .unwrap()
                .with_timezone(&Utc),
        ),
        web_url: "https://gitlab.example.com/group/project/-/jobs/789".into(),
    }
}

/// Create JSON representation of a project DTO
pub fn sample_project_dto_json() -> serde_json::Value {
    json!({
        "id": 123,
        "path_with_namespace": "group/project",
        "description": "Test project",
        "default_branch": "main",
        "ssh_url_to_repo": "git@gitlab.example.com:group/project.git",
        "web_url": "https://gitlab.example.com/group/project",
        "last_activity_at": "2023-01-01T00:00:00Z",
        "statistics": {
            "commit_count": 42,
            "job_artifacts_size": 1048576,
            "repository_size": 10485760
        }
    })
}

/// Create JSON representation of a pipeline DTO
pub fn sample_pipeline_dto_json() -> serde_json::Value {
    json!({
        "id": 456,
        "project_id": 123,
        "status": "success",
        "source": "push",
        "ref": "main",
        "web_url": "https://gitlab.example.com/group/project/-/pipelines/456",
        "created_at": "2023-01-01T00:00:00Z",
        "updated_at": "2023-01-01T01:00:00Z"
    })
}

/// Create JSON representation of a job DTO
pub fn sample_job_dto_json() -> serde_json::Value {
    json!({
        "id": 789,
        "name": "test-job",
        "stage": "test",
        "commit": {
            "title": "Test commit",
            "author_name": "Test Author"
        },
        "status": "success",
        "created_at": "2023-01-01T00:00:00Z",
        "started_at": "2023-01-01T00:01:00Z",
        "finished_at": "2023-01-01T00:05:00Z",
        "web_url": "https://gitlab.example.com/group/project/-/jobs/789"
    })
}

/// Create GitLab API error response
pub fn gitlab_error_response(error: &str, description: Option<&str>) -> serde_json::Value {
    let mut json = json!({
        "error": error
    });

    if let Some(desc) = description {
        json["error_description"] = json!(desc);
    }

    json
}

/// Create GitLab API error response (format 2)
pub fn gitlab_error_response_2(message: &str) -> serde_json::Value {
    json!({
        "message": message
    })
}

/// Mock HTTP server for testing
#[cfg(test)]
#[allow(dead_code)]
pub struct MockServer {
    pub server: wiremock::MockServer,
}

#[cfg(test)]
#[allow(dead_code)]
impl MockServer {
    /// Start a new mock server
    pub async fn start() -> Self {
        let server = wiremock::MockServer::start().await;
        Self { server }
    }

    /// Get the base URL of the mock server
    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    /// Create a test config pointing to this mock server
    pub fn test_config(&self) -> crate::client::config::ClientConfig {
        crate::client::config::ClientConfig::new(self.base_url(), "test-token")
    }
}

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_data_creation() {
        let project = sample_project_dto();
        assert_eq!(project.id, ProjectId::new(123));
        assert_eq!(project.path_with_namespace, "group/project");

        let pipeline = sample_pipeline_dto();
        assert_eq!(pipeline.id, PipelineId::new(456));
        assert_eq!(pipeline.project_id, ProjectId::new(123));

        let job = sample_job_dto();
        assert_eq!(job.id, JobId::new(789));
        assert_eq!(job.name, "test-job");
    }

    #[test]
    fn test_json_serialization() {
        let project_json = sample_project_dto_json();
        assert_eq!(project_json["id"], 123);
        assert_eq!(project_json["path_with_namespace"], "group/project");

        let pipeline_json = sample_pipeline_dto_json();
        assert_eq!(pipeline_json["id"], 456);
        assert_eq!(pipeline_json["project_id"], 123);

        let job_json = sample_job_dto_json();
        assert_eq!(job_json["id"], 789);
        assert_eq!(job_json["name"], "test-job");
    }

    #[test]
    fn test_error_responses() {
        let error1 = gitlab_error_response("invalid_token", Some("Token is invalid"));
        assert_eq!(error1["error"], "invalid_token");
        assert_eq!(error1["error_description"], "Token is invalid");

        let error2 = gitlab_error_response_2("Project not found");
        assert_eq!(error2["message"], "Project not found");
    }
}
