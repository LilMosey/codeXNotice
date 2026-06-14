use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;

use crate::notifications::local::{
    dispatch_pending_notifications, DispatchError, DispatchSummary, LocalNotifier,
};
use crate::scanner::{self, ScanSummary, ScannerError};
use crate::storage::{rules, schema};

pub struct RuntimeConfig {
    pub app_database_path: PathBuf,
    pub codex_directory: PathBuf,
    pub now_epoch_seconds: i64,
    pub delay_ttl_seconds: i64,
}

pub struct RuntimeLoopConfig {
    pub app_database_path: PathBuf,
    pub codex_directory: PathBuf,
    pub delay_ttl_seconds: i64,
    pub scan_interval: Duration,
}

impl RuntimeLoopConfig {
    pub fn to_runtime_config(&self, now_epoch_seconds: i64) -> RuntimeConfig {
        RuntimeConfig {
            app_database_path: self.app_database_path.clone(),
            codex_directory: self.codex_directory.clone(),
            now_epoch_seconds,
            delay_ttl_seconds: self.delay_ttl_seconds,
        }
    }
}

pub struct RuntimeSummary {
    pub scan: ScanSummary,
    pub notifications: DispatchSummary,
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("storage error: {0}")]
    Storage(#[from] crate::storage::error::StorageError),
    #[error("scanner error: {0}")]
    Scanner(#[from] ScannerError),
    #[error("dispatch error: {0}")]
    Dispatch(#[from] DispatchError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn default_app_database_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    Path::new(&home)
        .join("Library")
        .join("Application Support")
        .join("CodeX Notice")
        .join("codex-notice.sqlite")
}

pub fn default_codex_directory() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(".codex")
}

pub fn default_runtime_loop_config() -> RuntimeLoopConfig {
    RuntimeLoopConfig {
        app_database_path: default_app_database_path(),
        codex_directory: default_codex_directory(),
        delay_ttl_seconds: 86_400,
        scan_interval: Duration::from_secs(30),
    }
}

pub fn run_once<N: LocalNotifier>(
    config: &RuntimeConfig,
    notifier: &N,
) -> Result<RuntimeSummary, RuntimeError> {
    if let Some(parent) = config.app_database_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let connection = Connection::open(&config.app_database_path)?;
    schema::initialize(&connection)?;
    let notification_rules = rules::list_rules(&connection)?;
    let scan = scanner::scan_codex_state_files(
        &connection,
        &notification_rules,
        &config.codex_directory,
        config.now_epoch_seconds,
        config.delay_ttl_seconds,
    )?;
    let notifications = dispatch_pending_notifications(&connection, notifier)?;

    Ok(RuntimeSummary {
        scan,
        notifications,
    })
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::time::Duration;

    use rusqlite::Connection;

    use super::*;
    use crate::notifications::local::NotificationError;

    struct RecordingNotifier {
        calls: RefCell<Vec<(String, String)>>,
    }

    impl RecordingNotifier {
        fn new() -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
            }
        }
    }

    impl LocalNotifier for RecordingNotifier {
        fn notify(&self, title: &str, body: &str) -> Result<(), NotificationError> {
            self.calls
                .borrow_mut()
                .push((title.to_string(), body.to_string()));
            Ok(())
        }
    }

    fn seed_codex_state_database(path: &std::path::Path) {
        let connection = Connection::open(path).expect("open codex state database");
        connection
            .execute_batch(
                r#"
                CREATE TABLE agent_jobs (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    status TEXT NOT NULL,
                    instruction TEXT NOT NULL,
                    output_schema_json TEXT,
                    input_headers_json TEXT NOT NULL,
                    input_csv_path TEXT NOT NULL,
                    output_csv_path TEXT NOT NULL,
                    auto_export INTEGER NOT NULL DEFAULT 1,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL,
                    started_at INTEGER,
                    completed_at INTEGER,
                    last_error TEXT,
                    max_runtime_seconds INTEGER
                );

                INSERT INTO agent_jobs (
                    id, name, status, instruction, input_headers_json, input_csv_path,
                    output_csv_path, created_at, updated_at, started_at, completed_at, last_error
                ) VALUES (
                    'job-1', 'Long Codex Task', 'completed', 'Do work', '{}', '', '',
                    1000, 4000, 1000, 4000, NULL
                );
                "#,
            )
            .expect("seed codex database");
    }

    #[test]
    fn runtime_loop_config_builds_iteration_config_with_current_time() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let loop_config = RuntimeLoopConfig {
            app_database_path: temp.path().join("app.sqlite"),
            codex_directory: temp.path().join("codex"),
            delay_ttl_seconds: 600,
            scan_interval: Duration::from_secs(15),
        };

        let config = loop_config.to_runtime_config(12_345);

        assert_eq!(config.app_database_path, loop_config.app_database_path);
        assert_eq!(config.codex_directory, loop_config.codex_directory);
        assert_eq!(config.now_epoch_seconds, 12_345);
        assert_eq!(config.delay_ttl_seconds, 600);
    }

    #[test]
    fn run_once_scans_codex_state_and_dispatches_local_notification() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let app_database_path = temp.path().join("app").join("codex-notice.sqlite");
        let codex_directory = temp.path().join("codex");
        std::fs::create_dir_all(&codex_directory).expect("create codex directory");
        seed_codex_state_database(&codex_directory.join("state_5.sqlite"));
        let notifier = RecordingNotifier::new();

        let summary = run_once(
            &RuntimeConfig {
                app_database_path,
                codex_directory,
                now_epoch_seconds: 1_000,
                delay_ttl_seconds: 86_400,
            },
            &notifier,
        )
        .expect("run once");

        assert_eq!(summary.scan.discovered, 1);
        assert_eq!(summary.scan.processed, 1);
        assert_eq!(summary.notifications.sent, 1);
        assert_eq!(notifier.calls.borrow().len(), 1);
    }
}
