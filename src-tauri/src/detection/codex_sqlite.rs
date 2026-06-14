use std::path::{Path, PathBuf};

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
}
