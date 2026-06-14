use rusqlite::Connection;

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
}
