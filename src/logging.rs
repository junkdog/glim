use std::{path::PathBuf, sync::mpsc::Sender};

use compact_str::CompactString;
use directories::ProjectDirs;
use tracing::{Level, Metadata};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer,
};

use crate::event::GlimEvent;

/// Configuration for the logging system
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Log level for console output
    pub console_level: Level,
    /// Log level for file output
    pub file_level: Level,
    /// Directory where log files should be written
    pub log_dir: Option<PathBuf>,
    /// Whether to enable JSON formatted logs for structured output
    pub json_format: bool,
    /// Maximum number of log files to keep for rotation
    pub max_files: Option<usize>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            console_level: Level::INFO,
            file_level: Level::DEBUG,
            log_dir: Some(Self::default_log_dir()),
            json_format: false,
            max_files: Some(10),
        }
    }
}

impl LoggingConfig {
    /// Get the OS-appropriate default log directory
    pub fn default_log_dir() -> PathBuf {
        if let Some(proj_dirs) = ProjectDirs::from("", "", "glim") {
            // Use the cache directory for logs (more appropriate for temporary/log files)
            // On Linux: ~/.cache/glim
            // On macOS: ~/Library/Caches/glim
            // On Windows: %LOCALAPPDATA%\glim\cache
            proj_dirs.cache_dir().to_path_buf()
        } else {
            // Fallback to current directory if we can't determine OS directories
            PathBuf::from("glim-logs")
        }
    }

    /// Create logging configuration from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // Override log levels from environment
        if let Ok(level) = std::env::var("GLIM_LOG_LEVEL") {
            if let Ok(parsed_level) = level.parse::<Level>() {
                config.console_level = parsed_level;
                config.file_level = parsed_level;
            }
        }

        // Override log directory from environment
        if let Ok(log_dir) = std::env::var("GLIM_LOG_DIR") {
            config.log_dir = Some(PathBuf::from(log_dir));
        }

        // Disable file logging if requested
        if std::env::var("GLIM_NO_FILE_LOGS").is_ok() {
            config.log_dir = None;
        }

        // Enable JSON format for structured logging
        if std::env::var("GLIM_JSON_LOGS").is_ok() {
            config.json_format = true;
        }

        config
    }
}

/// Custom tracing layer that bridges logs to the internal UI logging system
pub struct InternalLogsLayer {
    sender: Sender<GlimEvent>,
    min_level: Level,
}

impl InternalLogsLayer {
    pub fn new(sender: Sender<GlimEvent>, min_level: Level) -> Self {
        Self { sender, min_level }
    }
}

impl<S> Layer<S> for InternalLogsLayer
where
    S: tracing::Subscriber,
{
    fn enabled(
        &self,
        metadata: &Metadata<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        metadata.level() <= &self.min_level
    }

    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // Extract the message from the event
        let mut visitor = LogMessageVisitor::new();
        event.record(&mut visitor);

        if let Some(message) = visitor.message {
            // Send the log message to the internal logs system
            let _ = self.sender.send(GlimEvent::LogEntry(message));
        }
    }
}

/// Visitor to extract log messages from tracing events
struct LogMessageVisitor {
    message: Option<CompactString>,
}

impl LogMessageVisitor {
    fn new() -> Self {
        Self { message: None }
    }
}

impl tracing::field::Visit for LogMessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{value:?}").trim_matches('"').into());
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.into());
        }
    }
}

/// Initialize the logging system with the given configuration
pub fn init_logging(
    config: LoggingConfig,
    _event_sender: Option<Sender<GlimEvent>>,
) -> Result<Option<WorkerGuard>, Box<dyn std::error::Error>> {
    let mut layers = vec![];
    let mut guard = None;

    // Create file logging layer if log directory is specified
    if let Some(log_dir) = &config.log_dir {
        // Ensure log directory exists
        std::fs::create_dir_all(log_dir)?;

        let file_appender = tracing_appender::rolling::daily(log_dir, "glim.log");
        let (non_blocking, file_guard) = tracing_appender::non_blocking(file_appender);
        guard = Some(file_guard);

        let file_layer = if config.json_format {
            fmt::layer()
                .json()
                .with_writer(non_blocking)
                .with_filter(
                    EnvFilter::builder()
                        .with_default_directive(config.file_level.into())
                        .from_env_lossy(),
                )
                .boxed()
        } else {
            fmt::layer()
                .with_writer(non_blocking)
                .with_filter(
                    EnvFilter::builder()
                        .with_default_directive(config.file_level.into())
                        .from_env_lossy(),
                )
                .boxed()
        };

        layers.push(file_layer);
    }

    // Create console logging layer (for development/debugging)
    if std::env::var("GLIM_CONSOLE_LOGS").is_ok() {
        let console_layer = fmt::layer()
            .with_target(true)
            .with_file(false)
            .with_line_number(false)
            .with_filter(
                EnvFilter::builder()
                    .with_default_directive(config.console_level.into())
                    .from_env_lossy(),
            )
            .boxed();

        layers.push(console_layer);
    }

    // Create internal logs bridge layer if event sender is provided
    // Note: This layer sends log events to the UI system, but there's no UI component
    // to display them anymore after removing the internal logs widget.
    // Keeping it commented out to avoid unnecessary event processing.
    /*
    if let Some(sender) = event_sender {
        let internal_layer = InternalLogsLayer::new(sender, Level::INFO).boxed();
        layers.push(internal_layer);
    }
    */

    // Initialize the subscriber with all layers
    let subscriber = tracing_subscriber::registry().with(layers);
    subscriber.init();

    Ok(guard)
}

/// Convenience macro for logging with structured fields
#[macro_export]
macro_rules! log_with_context {
    ($level:expr, $message:expr, $($field:ident = $value:expr),*) => {
        match $level {
            tracing::Level::ERROR => tracing::error!($message, $($field = $value),*),
            tracing::Level::WARN => tracing::warn!($message, $($field = $value),*),
            tracing::Level::INFO => tracing::info!($message, $($field = $value),*),
            tracing::Level::DEBUG => tracing::debug!($message, $($field = $value),*),
            tracing::Level::TRACE => tracing::trace!($message, $($field = $value),*),
        }
    };
}
