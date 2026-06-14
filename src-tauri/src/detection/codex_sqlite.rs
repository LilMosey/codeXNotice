use std::path::{Path, PathBuf};

use chrono::{DateTime, Datelike, Timelike, Utc};
use rusqlite::Connection;

use crate::domain::task::{TaskRecord, Weekday};

#[derive(Debug, thiserror::Error)]
pub enum CodexSqliteError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn find_state_databases(directory: &Path) -> Result<Vec<PathBuf>, CodexSqliteError> {
    let mut files = Vec::new();

    if !directory.exists() {
        return Ok(files);
    }

    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if file_name.starts_with("state_") && file_name.ends_with(".sqlite") {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}

pub fn detect_completed_agent_jobs(path: &Path) -> Result<Vec<TaskRecord>, CodexSqliteError> {
    let connection = Connection::open(path)?;
    if !table_exists(&connection, "agent_jobs")? {
        return Ok(Vec::new());
    }

    let mut statement = connection.prepare(
        r#"
        SELECT id, name, status, started_at, completed_at, last_error
        FROM agent_jobs
        WHERE completed_at IS NOT NULL
        "#,
    )?;

    let rows = statement.query_map([], |row| {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let status: String = row.get(2)?;
        let started_at: Option<i64> = row.get(3)?;
        let completed_at: Option<i64> = row.get(4)?;
        let last_error: Option<String> = row.get(5)?;

        let duration_seconds = match (started_at, completed_at) {
            (Some(started), Some(completed)) if completed >= started => {
                ((completed - started) / 1000) as u64
            }
            _ => 0,
        };

        Ok(TaskRecord {
            id,
            title: name,
            duration_seconds,
            completed_at_weekday: Weekday::Mon,
            completed_at_seconds: 0,
            success: status == "completed" && last_error.is_none(),
        })
    })?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(CodexSqliteError::from)
}

pub fn detect_recent_threads(
    path: &Path,
    minimum_updated_at_ms: i64,
) -> Result<Vec<TaskRecord>, CodexSqliteError> {
    let connection = Connection::open(path)?;
    if !table_exists(&connection, "threads")? {
        return Ok(Vec::new());
    }

    let mut statement = connection.prepare(
        r#"
        SELECT id, title, created_at_ms, updated_at_ms, created_at, updated_at
        FROM threads
        WHERE updated_at_ms IS NOT NULL
          AND updated_at_ms >= ?1
          AND source NOT LIKE '%subagent%'
        "#,
    )?;

    let rows = statement.query_map([minimum_updated_at_ms], |row| {
        let id: String = row.get(0)?;
        let title: String = row.get(1)?;
        let created_at_ms: Option<i64> = row.get(2)?;
        let updated_at_ms: Option<i64> = row.get(3)?;
        let created_at: i64 = row.get(4)?;
        let updated_at: i64 = row.get(5)?;

        let started_ms = created_at_ms.unwrap_or(created_at * 1000);
        let completed_ms = updated_at_ms.unwrap_or(updated_at * 1000);
        let duration_seconds = if completed_ms >= started_ms {
            ((completed_ms - started_ms) / 1000) as u64
        } else {
            0
        };
        let (completed_at_weekday, completed_at_seconds) =
            timestamp_parts(completed_ms / 1000);

        Ok(TaskRecord {
            id: format!("thread:{id}:{completed_ms}"),
            title: if title.trim().is_empty() {
                "Codex 桌面任务".to_string()
            } else {
                title
            },
            duration_seconds,
            completed_at_weekday,
            completed_at_seconds,
            success: true,
        })
    })?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(CodexSqliteError::from)
}

fn table_exists(connection: &Connection, table_name: &str) -> Result<bool, CodexSqliteError> {
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table_name],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn timestamp_parts(timestamp_seconds: i64) -> (Weekday, u32) {
    let datetime = DateTime::<Utc>::from_timestamp(timestamp_seconds, 0)
        .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).expect("unix epoch"));
    let weekday = match datetime.weekday() {
        chrono::Weekday::Mon => Weekday::Mon,
        chrono::Weekday::Tue => Weekday::Tue,
        chrono::Weekday::Wed => Weekday::Wed,
        chrono::Weekday::Thu => Weekday::Thu,
        chrono::Weekday::Fri => Weekday::Fri,
        chrono::Weekday::Sat => Weekday::Sat,
        chrono::Weekday::Sun => Weekday::Sun,
    };
    let seconds = datetime.num_seconds_from_midnight();
    (weekday, seconds)
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use tempfile::NamedTempFile;

    #[test]
    fn finds_only_codex_state_sqlite_files() {
        let directory = tempfile::tempdir().expect("create temp directory");
        std::fs::write(directory.path().join("state_5.sqlite"), "").expect("write state db");
        std::fs::write(directory.path().join("state_5.sqlite-wal"), "").expect("write wal");
        std::fs::write(directory.path().join("logs_2.sqlite"), "").expect("write logs db");

        let files = super::find_state_databases(directory.path()).expect("find state databases");

        assert_eq!(files, vec![directory.path().join("state_5.sqlite")]);
    }

    #[test]
    fn missing_codex_directory_returns_no_state_databases() {
        let directory = tempfile::tempdir().expect("create temp directory");
        let missing_path = directory.path().join("missing-codex");

        let files = super::find_state_databases(&missing_path).expect("find state databases");

        assert!(files.is_empty());
    }

    #[test]
    fn detects_completed_agent_jobs_once_from_sqlite() {
        let db = NamedTempFile::new().expect("create temp database");
        let connection = Connection::open(db.path()).expect("open temp database");
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
            .expect("seed database");

        let tasks = super::detect_completed_agent_jobs(db.path()).expect("detect jobs");

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "job-1");
        assert_eq!(tasks[0].title, "Long Codex Task");
        assert_eq!(tasks[0].duration_seconds, 3);
        assert!(tasks[0].success);
    }

    #[test]
    fn detects_recent_desktop_threads_from_sqlite() {
        let db = NamedTempFile::new().expect("create temp database");
        let connection = Connection::open(db.path()).expect("open temp database");
        connection
            .execute_batch(
                r#"
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

                INSERT INTO threads (
                    id, title, created_at, updated_at, source, cwd, created_at_ms, updated_at_ms
                ) VALUES
                ('thread-1', '普通 Codex 任务', 100, 140, 'vscode', '/tmp/project', 100000, 140000),
                ('thread-2', '旧任务', 10, 20, 'vscode', '/tmp/project', 10000, 20000),
                ('thread-3', '子任务审批', 100, 140, '{"subagent":{"other":"guardian"}}', '/tmp/project', 100000, 140000);
                "#,
            )
            .expect("seed database");

        let tasks = super::detect_recent_threads(db.path(), 120_000).expect("detect threads");

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "thread:thread-1:140000");
        assert_eq!(tasks[0].title, "普通 Codex 任务");
        assert_eq!(tasks[0].duration_seconds, 40);
    }

    #[test]
    fn missing_threads_table_returns_no_recent_threads() {
        let db = NamedTempFile::new().expect("create temp database");
        Connection::open(db.path()).expect("open temp database");

        let tasks = super::detect_recent_threads(db.path(), 1).expect("detect threads");

        assert!(tasks.is_empty());
    }
}
