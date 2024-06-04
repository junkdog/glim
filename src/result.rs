use confy::ConfyError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, GlimError>;

#[derive(Debug, Error)]
pub enum GlimError {
    #[error("The provided Gitlab token is invalid.")]
    InvalidGitlabToken,
    #[error("The provided Gitlab token has expired.")]
    ExpiredGitlabToken,
    #[error("Failure reading configuration file.")]
    ConfigError(#[source] ConfyError),

    #[error("{0}")]
    GeneralError(String),
}

impl From<reqwest::Error> for GlimError {
    fn from(e: reqwest::Error) -> Self {
        match () {
            _ => GlimError::GeneralError(e.to_string()),
        }
    }
}

// impl From<Box<dyn std::error::Error>> for GlimError {
//     fn from(value: Box<dyn std::error::Error>) -> Self {
//         // match error type
//         // if value.is::<reqwest::Error>() {
//         //     let message = value.downcast::<reqwest::Error>().unwrap().to_string();
//         //     Error::GeneralError(message)
//         // } else if value.is::<serde_json::Error>() {
//         //     Error::Serde(*value.downcast::<serde_json::Error>().unwrap())
//         // } else if value.is::<std::io::Error>() {
//         //     Error::IoError(*value.downcast::<std::io::Error>().unwrap())
//         // } else {
//             GlimError::GeneralError(value.to_string())
//         // }
//     }
// }