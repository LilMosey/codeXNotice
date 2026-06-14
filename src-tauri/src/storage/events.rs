use rusqlite::{params, Connection};

use crate::domain::decision::{NotificationDecision, NotificationEventStatus};
use crate::domain::task::TaskRecord;
use crate::storage::error::StorageError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationEventRecord {
    pub id: String,
    pub task_id: String,
    pub rule_id: Option<String>,
    pub decision: NotificationDecision,
    pub status: NotificationEventStatus,
}

pub fn record_task(
    connection: &Connection,
    task: &TaskRecord,
    source: &str,
) -> Result<(), StorageError> {
    connection.execute(
        r#"
        INSERT INTO detected_tasks (
            id, title, duration_seconds, completed_weekday, completed_seconds,
            success, source, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, strftime('%s','now'))
        ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            duration_seconds = excluded.duration_seconds,
            completed_weekday = excluded.completed_weekday,
            completed_seconds = excluded.completed_seconds,
            success = excluded.success,
            source = excluded.source
        "#,
        params![
            task.id,
            task.title,
            task.duration_seconds as i64,
            serde_json::to_string(&task.completed_at_weekday)?,
            task.completed_at_seconds as i64,
            if task.success { 1 } else { 0 },
            source,
        ],
    )?;

    Ok(())
}

pub fn task_exists(connection: &Connection, task_id: &str) -> Result<bool, StorageError> {
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM detected_tasks WHERE id = ?1",
        params![task_id],
        |row| row.get(0),
    )?;

    Ok(count > 0)
}

pub fn record_event(
    connection: &Connection,
    event_id: &str,
    task_id: &str,
    decision: &NotificationDecision,
    status: NotificationEventStatus,
) -> Result<(), StorageError> {
    connection.execute(
        r#"
        INSERT INTO notification_events (
            id, task_id, rule_id, decision_json, status, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, strftime('%s','now'), strftime('%s','now'))
        "#,
        params![
            event_id,
            task_id,
            decision_rule_id(decision),
            serde_json::to_string(decision)?,
            serde_json::to_string(&status)?,
        ],
    )?;

    Ok(())
}

pub fn list_events(connection: &Connection) -> Result<Vec<NotificationEventRecord>, StorageError> {
    let mut statement = connection.prepare(
        r#"
        SELECT id, task_id, rule_id, decision_json, status
        FROM notification_events
        ORDER BY created_at ASC, id ASC
        "#,
    )?;

    let rows = statement.query_map([], |row| {
        Ok(StoredEventRow {
            id: row.get(0)?,
            task_id: row.get(1)?,
            rule_id: row.get(2)?,
            decision_json: row.get(3)?,
            status_json: row.get(4)?,
        })
    })?;

    let mut events = Vec::new();
    for row in rows {
        let row = row?;
        events.push(NotificationEventRecord {
            id: row.id,
            task_id: row.task_id,
            rule_id: row.rule_id,
            decision: serde_json::from_str(&row.decision_json)?,
            status: serde_json::from_str(&row.status_json)?,
        });
    }

    Ok(events)
}

pub fn queue_delayed_task(
    connection: &Connection,
    rule_id: &str,
    task_id: &str,
    queued_at: i64,
    expires_at: i64,
) -> Result<(), StorageError> {
    connection.execute(
        r#"
        INSERT OR IGNORE INTO delayed_tasks (
            rule_id, task_id, queued_at, expires_at, sent_at
        ) VALUES (?1, ?2, ?3, ?4, NULL)
        "#,
        params![rule_id, task_id, queued_at, expires_at],
    )?;

    Ok(())
}

pub fn list_pending_delayed_task_ids(
    connection: &Connection,
    rule_id: &str,
) -> Result<Vec<String>, StorageError> {
    let mut statement = connection.prepare(
        r#"
        SELECT task_id
        FROM delayed_tasks
        WHERE rule_id = ?1 AND sent_at IS NULL
        ORDER BY queued_at ASC, task_id ASC
        "#,
    )?;

    let rows = statement.query_map(params![rule_id], |row| row.get::<_, String>(0))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(StorageError::from)
}

fn decision_rule_id(decision: &NotificationDecision) -> Option<&str> {
    match decision {
        NotificationDecision::Ignore => None,
        NotificationDecision::SendNow { rule_id }
        | NotificationDecision::Discard { rule_id }
        | NotificationDecision::Delay { rule_id } => Some(rule_id),
    }
}

struct StoredEventRow {
    id: String,
    task_id: String,
    rule_id: Option<String>,
    decision_json: String,
    status_json: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::Weekday;
    use crate::storage::schema;

    fn task(id: &str) -> TaskRecord {
        TaskRecord {
            id: id.to_string(),
            title: "Codex task".to_string(),
            duration_seconds: 1800,
            completed_at_weekday: Weekday::Mon,
            completed_at_seconds: 9 * 3600,
            success: true,
        }
    }

    #[test]
    fn records_task_and_notification_event() {
        let connection = Connection::open_in_memory().expect("open database");
        schema::initialize(&connection).expect("initialize schema");
        let task = task("task-1");

        record_task(&connection, &task, "codex-sqlite").expect("record task");
        record_event(
            &connection,
            "event-1",
            "task-1",
            &NotificationDecision::SendNow {
                rule_id: "default-rule".to_string(),
            },
            NotificationEventStatus::Pending,
        )
        .expect("record event");

        let events = list_events(&connection).expect("list events");

        assert_eq!(
            events,
            vec![NotificationEventRecord {
                id: "event-1".to_string(),
                task_id: "task-1".to_string(),
                rule_id: Some("default-rule".to_string()),
                decision: NotificationDecision::SendNow {
                    rule_id: "default-rule".to_string(),
                },
                status: NotificationEventStatus::Pending,
            }]
        );
    }

    #[test]
    fn delayed_tasks_are_listed_once_per_rule() {
        let connection = Connection::open_in_memory().expect("open database");
        schema::initialize(&connection).expect("initialize schema");
        record_task(&connection, &task("task-1"), "codex-sqlite").expect("record task");
        record_task(&connection, &task("task-2"), "codex-sqlite").expect("record task");

        queue_delayed_task(&connection, "rule-1", "task-1", 100, 200).expect("queue task");
        queue_delayed_task(&connection, "rule-1", "task-1", 100, 200)
            .expect("queue task again");
        queue_delayed_task(&connection, "rule-1", "task-2", 100, 200)
            .expect("queue second task");

        let pending = list_pending_delayed_task_ids(&connection, "rule-1").expect("list pending");

        assert_eq!(pending, vec!["task-1".to_string(), "task-2".to_string()]);
    }

    #[test]
    fn task_exists_returns_true_after_task_is_recorded() {
        let connection = Connection::open_in_memory().expect("open database");
        schema::initialize(&connection).expect("initialize schema");

        assert!(!task_exists(&connection, "task-1").expect("check missing task"));

        record_task(&connection, &task("task-1"), "codex-sqlite").expect("record task");

        assert!(task_exists(&connection, "task-1").expect("check recorded task"));
    }
}
