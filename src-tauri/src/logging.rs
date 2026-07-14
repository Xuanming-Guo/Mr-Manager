use std::fmt;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use thiserror::Error;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::FmtContext;
use tracing_subscriber::fmt::format::{FormatEvent, FormatFields, Writer};
use tracing_subscriber::registry::LookupSpan;

use crate::security::redaction;

// Retained for continuity with existing app-data logs after the product rename.
const LOG_FILE_PREFIX: &str = "desktop-manager.log";
const RETAINED_LOG_FILES: usize = 7;

#[derive(Debug, Error)]
pub enum LoggingError {
    #[error("the application log directory could not be created")]
    Directory(#[source] std::io::Error),
    #[error("old application logs could not be pruned")]
    Retention(#[source] std::io::Error),
    #[error("the structured logging subscriber could not be initialized: {0}")]
    Subscriber(String),
}

pub struct LoggingGuard {
    _worker: WorkerGuard,
}

pub fn initialize(log_directory: &Path) -> Result<LoggingGuard, LoggingError> {
    std::fs::create_dir_all(log_directory).map_err(LoggingError::Directory)?;
    prune_rotated_logs(log_directory, RETAINED_LOG_FILES).map_err(LoggingError::Retention)?;

    let appender = tracing_appender::rolling::daily(log_directory, LOG_FILE_PREFIX);
    let (writer, worker) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_env_filter("mr_manager_lib=info,mr_manager=info,tauri=warn")
        .with_writer(writer)
        .with_ansi(false)
        .event_format(RedactingEventFormat)
        .try_init()
        .map_err(|error| LoggingError::Subscriber(error.to_string()))?;

    Ok(LoggingGuard { _worker: worker })
}

fn prune_rotated_logs(directory: &Path, retained: usize) -> std::io::Result<()> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name();
        if !name.to_string_lossy().starts_with(LOG_FILE_PREFIX) {
            continue;
        }
        let metadata = entry.metadata()?;
        if metadata.is_file() {
            files.push((metadata.modified().unwrap_or(UNIX_EPOCH), entry.path()));
        }
    }
    files.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| path_name(&left.1).cmp(&path_name(&right.1)))
    });

    let remove_count = files.len().saturating_sub(retained);
    for (_, path) in files.into_iter().take(remove_count) {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

fn path_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

struct RedactingEventFormat;

impl<S, N> FormatEvent<S, N> for RedactingEventFormat
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        _context: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default();
        let metadata = event.metadata();
        write!(
            writer,
            "{timestamp_ms} {} {}",
            metadata.level(),
            metadata.target()
        )?;

        let mut visitor = RedactingVisitor::default();
        event.record(&mut visitor);
        for (name, value) in visitor.fields {
            if name == "message" {
                write!(writer, " {}", redaction::redact(&value))?;
            } else {
                write!(writer, " {name}={}", redaction::redact(&value))?;
            }
        }
        writeln!(writer)
    }
}

#[derive(Default)]
struct RedactingVisitor {
    fields: Vec<(String, String)>,
}

impl Visit for RedactingVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.fields
            .push((field.name().to_owned(), format!("{value:?}")));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.fields
            .push((field.name().to_owned(), value.to_owned()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retention_only_removes_application_log_files() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        for index in 0..10 {
            std::fs::write(
                directory
                    .path()
                    .join(format!("{LOG_FILE_PREFIX}.{index:02}")),
                b"fixture",
            )
            .expect("fixture log should be written");
        }
        let unrelated = directory.path().join("keep-me.txt");
        std::fs::write(&unrelated, b"fixture").expect("fixture file should be written");

        prune_rotated_logs(directory.path(), 3).expect("retention should succeed");

        let retained_logs = std::fs::read_dir(directory.path())
            .expect("directory should be readable")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(LOG_FILE_PREFIX)
            })
            .count();
        assert_eq!(retained_logs, 3);
        assert!(unrelated.exists());
    }
}
