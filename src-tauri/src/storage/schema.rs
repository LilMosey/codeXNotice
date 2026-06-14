use rusqlite::{params, Connection};

use crate::domain::rule::{
    DurationCondition, NotificationRule, OutsideWindowPolicy, TimeWindowCondition,
};
use crate::storage::error::StorageError;

pub fn initialize(connection: &Connection) -> Result<(), StorageError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS notification_rules (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            enabled INTEGER NOT NULL,
            rule_order INTEGER NOT NULL UNIQUE,
            duration_json TEXT NOT NULL,
            time_window_json TEXT NOT NULL,
            outside_window TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS detected_tasks (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            duration_seconds INTEGER NOT NULL,
            completed_weekday TEXT NOT NULL,
            completed_seconds INTEGER NOT NULL,
            success INTEGER NOT NULL,
            source TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS notification_events (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            rule_id TEXT,
            decision_json TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            FOREIGN KEY(task_id) REFERENCES detected_tasks(id)
        );

        CREATE TABLE IF NOT EXISTS delayed_tasks (
            rule_id TEXT NOT NULL,
            task_id TEXT NOT NULL,
            queued_at INTEGER NOT NULL,
            expires_at INTEGER NOT NULL,
            sent_at INTEGER,
            PRIMARY KEY(rule_id, task_id),
            FOREIGN KEY(task_id) REFERENCES detected_tasks(id)
        );
        "#,
    )?;

    let existing_rules: i64 =
        connection.query_row("SELECT COUNT(*) FROM notification_rules", [], |row| row.get(0))?;

    if existing_rules == 0 {
        let default_rule = NotificationRule {
            id: "default-rule".to_string(),
            name: "默认规则".to_string(),
            enabled: true,
            duration: DurationCondition::Any,
            time_window: TimeWindowCondition::Always,
            outside_window: OutsideWindowPolicy::Discard,
        };

        connection.execute(
            r#"
            INSERT INTO notification_rules (
                id, name, enabled, rule_order, duration_json, time_window_json,
                outside_window, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, strftime('%s','now'), strftime('%s','now'))
            "#,
            params![
                default_rule.id,
                default_rule.name,
                1,
                0,
                serde_json::to_string(&default_rule.duration)?,
                serde_json::to_string(&default_rule.time_window)?,
                serde_json::to_string(&default_rule.outside_window)?,
            ],
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_creates_required_tables() {
        let connection = Connection::open_in_memory().expect("open in-memory database");

        initialize(&connection).expect("initialize schema");

        let table_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('app_settings', 'notification_rules')",
                [],
                |row| row.get(0),
            )
            .expect("count tables");

        assert_eq!(table_count, 2);
    }

    #[test]
    fn initialize_creates_default_rule_once() {
        let connection = Connection::open_in_memory().expect("open in-memory database");

        initialize(&connection).expect("initialize schema");
        initialize(&connection).expect("initialize schema again");

        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM notification_rules", [], |row| row.get(0))
            .expect("count rules");
        let rule_name: String = connection
            .query_row(
                "SELECT name FROM notification_rules WHERE rule_order = 0",
                [],
                |row| row.get(0),
            )
            .expect("read default rule");

        assert_eq!(count, 1);
        assert_eq!(rule_name, "默认规则");
    }

    #[test]
    fn initialize_creates_history_and_delay_tables() {
        let connection = Connection::open_in_memory().expect("open in-memory database");

        initialize(&connection).expect("initialize schema");

        let table_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('detected_tasks', 'notification_events', 'delayed_tasks')",
                [],
                |row| row.get(0),
            )
            .expect("count tables");

        assert_eq!(table_count, 3);
    }
}
