use std::path::Path;

use rusqlite::Connection;

use crate::detection::codex_sqlite::{self, CodexSqliteError};
use crate::domain::rule::NotificationRule;
use crate::domain::task::TaskRecord;
use crate::processor;
use crate::storage::error::StorageError;
use crate::storage::events;

pub struct ScanSummary {
    pub discovered: usize,
    pub skipped_existing: usize,
    pub processed: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum ScannerError {
    #[error("codex sqlite error: {0}")]
    CodexSqlite(#[from] CodexSqliteError),
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
}

pub fn scan_codex_state_files(
    connection: &Connection,
    rules: &[NotificationRule],
    codex_directory: &Path,
    now_epoch_seconds: i64,
    delay_ttl_seconds: i64,
) -> Result<ScanSummary, ScannerError> {
    let mut tasks = Vec::new();
    let minimum_thread_updated_at_ms = (now_epoch_seconds - 600).max(0) * 1000;
    let maximum_thread_updated_at_ms = (now_epoch_seconds - 60).max(0) * 1000;

    for database_path in codex_sqlite::find_state_databases(codex_directory)? {
        tasks.extend(codex_sqlite::detect_completed_agent_jobs(&database_path)?);
        tasks.extend(codex_sqlite::detect_recent_threads(
            &database_path,
            minimum_thread_updated_at_ms,
            maximum_thread_updated_at_ms,
        )?);
    }

    scan_tasks(
        connection,
        rules,
        &tasks,
        "codex-sqlite",
        now_epoch_seconds,
        delay_ttl_seconds,
    )
    .map_err(ScannerError::from)
}

pub fn scan_tasks(
    connection: &Connection,
    rules: &[NotificationRule],
    tasks: &[TaskRecord],
    source: &str,
    now_epoch_seconds: i64,
    delay_ttl_seconds: i64,
) -> Result<ScanSummary, StorageError> {
    let mut skipped_existing = 0;
    let mut processed = 0;

    for task in tasks {
        if events::task_exists(connection, &task.id)? {
            skipped_existing += 1;
            continue;
        }

        processor::process_task(
            connection,
            rules,
            task,
            source,
            now_epoch_seconds,
            delay_ttl_seconds,
        )?;
        processed += 1;
    }

    Ok(ScanSummary {
        discovered: tasks.len(),
        skipped_existing,
        processed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::decision::NotificationEventStatus;
    use crate::domain::rule::{
        DurationCondition, NotificationRule, OutsideWindowPolicy, TimeWindowCondition,
    };
    use crate::domain::task::Weekday;
    use crate::storage::{events, schema};
    use rusqlite::Connection;

    fn task(id: &str) -> TaskRecord {
        TaskRecord {
            id: id.to_string(),
            title: format!("Task {id}"),
            duration_seconds: 120,
            completed_at_weekday: Weekday::Mon,
            completed_at_seconds: 9 * 3600,
            success: true,
        }
    }

    fn rule() -> NotificationRule {
        NotificationRule {
            id: "default-rule".to_string(),
            name: "默认规则".to_string(),
            enabled: true,
            duration: DurationCondition::Any,
            time_window: TimeWindowCondition::Always,
            outside_window: OutsideWindowPolicy::Discard,
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

                CREATE TABLE threads (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL,
                    source TEXT NOT NULL,
                    cwd TEXT NOT NULL,
                    created_at_ms INTEGER,
                    updated_at_ms INTEGER
                );

                INSERT INTO agent_jobs (
                    id, name, status, instruction, input_headers_json, input_csv_path,
                    output_csv_path, created_at, updated_at, started_at, completed_at, last_error
                ) VALUES (
                    'job-1', 'Long Codex Task', 'completed', 'Do work', '{}', '', '',
                    1000, 4000, 1000, 4000, NULL
                );

                INSERT INTO threads (
                    id, title, created_at, updated_at, source, cwd, created_at_ms, updated_at_ms
                ) VALUES (
                    'thread-1', 'Desktop Thread', 840, 900, 'vscode', '/tmp/project', 840000, 900000
                );
                "#,
            )
            .expect("seed codex database");
    }

    #[test]
    fn scan_tasks_processes_new_tasks_and_skips_existing_tasks() {
        let connection = Connection::open_in_memory().expect("open database");
        schema::initialize(&connection).expect("initialize schema");
        events::record_task(&connection, &task("task-1"), "codex-sqlite")
            .expect("record existing task");
        let tasks = vec![task("task-1"), task("task-2"), task("task-3")];

        let summary = scan_tasks(&connection, &[rule()], &tasks, "codex-sqlite", 1_000, 86_400)
            .expect("scan tasks");

        assert_eq!(summary.discovered, 3);
        assert_eq!(summary.skipped_existing, 1);
        assert_eq!(summary.processed, 2);
        let events = events::list_events(&connection).expect("list events");
        assert_eq!(events.len(), 2);
        assert!(events
            .iter()
            .all(|event| event.status == NotificationEventStatus::Pending));
    }

    #[test]
    fn scan_codex_state_files_reads_state_databases_and_processes_tasks() {
        let app_connection = Connection::open_in_memory().expect("open app database");
        schema::initialize(&app_connection).expect("initialize schema");
        let directory = tempfile::tempdir().expect("create temp directory");
        let codex_db = directory.path().join("state_5.sqlite");
        seed_codex_state_database(&codex_db);

        let summary = super::scan_codex_state_files(
            &app_connection,
            &[rule()],
            directory.path(),
            1_000,
            86_400,
        )
        .expect("scan codex state files");

        assert_eq!(summary.discovered, 2);
        assert_eq!(summary.processed, 2);
        let events = events::list_events(&app_connection).expect("list events");
        assert_eq!(events.len(), 2);
    }
}
