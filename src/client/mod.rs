//! GitLab client modules
//!
//! This module provides a well-structured, testable GitLab API client
//! split into focused components following single responsibility principle.

pub mod api;
pub mod config;
pub mod error;
pub mod poller;
pub mod service;

#[cfg(test)]
mod tests;

// Re-export main types for convenience
pub use config::ClientConfig;
pub use error::ClientError;
pub use poller::GitlabPoller;
pub use service::GitlabService;

pub type Result<T> = std::result::Result<T, ClientError>;
