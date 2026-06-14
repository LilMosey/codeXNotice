use rusqlite::Connection;

use crate::domain::decision::{NotificationDecision, NotificationEventStatus};
use crate::domain::rule::NotificationRule;
use crate::domain::task::TaskRecord;
use crate::rules;
use crate::storage::error::StorageError;
use crate::storage::events;

pub struct ProcessingOutcome {
    pub event_id: String,
    pub status: NotificationEventStatus,
}

pub fn process_task(
    connection: &Connection,
    rules: &[NotificationRule],
    task: &TaskRecord,
    source: &str,
    now_epoch_seconds: i64,
    delay_ttl_seconds: i64,
) -> Result<ProcessingOutcome, StorageError> {
    events::record_task(connection, task, source)?;

    let decision = rules::evaluate(rules, task);
    let status = status_for_decision(&decision);
    let event_id = format!("{}:{}", task.id, status_name(status));

    events::record_event(connection, &event_id, &task.id, &decision, status)?;

    if let NotificationDecision::Delay { rule_id } = &decision {
        events::queue_delayed_task(
            connection,
            rule_id,
            &task.id,
            now_epoch_seconds,
            now_epoch_seconds + delay_ttl_seconds,
        )?;
    }

    Ok(ProcessingOutcome { event_id, status })
}

fn status_for_decision(decision: &NotificationDecision) -> NotificationEventStatus {
    match decision {
        NotificationDecision::Ignore => NotificationEventStatus::Ignored,
        NotificationDecision::SendNow { .. } => NotificationEventStatus::Pending,
        NotificationDecision::Discard { .. } => NotificationEventStatus::Discarded,
        NotificationDecision::Delay { .. } => NotificationEventStatus::Delayed,
    }
}

fn status_name(status: NotificationEventStatus) -> &'static str {
    match status {
        NotificationEventStatus::Ignored => "ignored",
        NotificationEventStatus::Pending => "pending",
        NotificationEventStatus::Sent => "sent",
        NotificationEventStatus::Discarded => "discarded",
        NotificationEventStatus::Delayed => "delayed",
        NotificationEventStatus::Failed => "failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::decision::{NotificationDecision, NotificationEventStatus};
    use crate::domain::rule::{
        DurationCondition, DurationRange, NotificationRule, OutsideWindowPolicy, TimeWindow,
        TimeWindowCondition,
    };
    use crate::domain::task::Weekday;
    use crate::storage::{events, schema};

    fn task(duration_seconds: u64, weekday: Weekday, seconds: u32) -> TaskRecord {
        TaskRecord {
            id: "task-1".to_string(),
            title: "Codex task".to_string(),
            duration_seconds,
            completed_at_weekday: weekday,
            completed_at_seconds: seconds,
            success: true,
        }
    }

    fn always_rule() -> NotificationRule {
        NotificationRule {
            id: "default-rule".to_string(),
            name: "默认规则".to_string(),
            enabled: true,
            duration: DurationCondition::Any,
            time_window: TimeWindowCondition::Always,
            outside_window: OutsideWindowPolicy::Discard,
        }
    }

    fn delayed_workday_rule() -> NotificationRule {
        NotificationRule {
            id: "workday-short".to_string(),
            name: "工作日中短任务".to_string(),
            enabled: true,
            duration: DurationCondition::Ranges(vec![DurationRange {
                min_seconds: 60,
                max_seconds: Some(50 * 60),
            }]),
            time_window: TimeWindowCondition::Windows(vec![TimeWindow {
                weekdays: vec![
                    Weekday::Mon,
                    Weekday::Tue,
                    Weekday::Wed,
                    Weekday::Thu,
                    Weekday::Fri,
                ],
                start_seconds: 8 * 3600,
                end_seconds: 20 * 3600,
            }]),
            outside_window: OutsideWindowPolicy::Delay,
        }
    }

    fn discard_workday_rule() -> NotificationRule {
        NotificationRule {
            outside_window: OutsideWindowPolicy::Discard,
            ..delayed_workday_rule()
        }
    }

    #[test]
    fn process_task_records_pending_event_when_rule_allows_send_now() {
        let connection = Connection::open_in_memory().expect("open database");
        schema::initialize(&connection).expect("initialize schema");
        let task = task(30 * 60, Weekday::Mon, 9 * 3600);

        let outcome = process_task(
            &connection,
            &[always_rule()],
            &task,
            "codex-sqlite",
            1_000,
            86_400,
        )
        .expect("process task");

        assert_eq!(outcome.status, NotificationEventStatus::Pending);
        let events = events::list_events(&connection).expect("list events");
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].decision,
            NotificationDecision::SendNow {
                rule_id: "default-rule".to_string()
            }
        );
        assert_eq!(events[0].status, NotificationEventStatus::Pending);
    }

    #[test]
    fn process_task_queues_delayed_event_for_selected_rule() {
        let connection = Connection::open_in_memory().expect("open database");
        schema::initialize(&connection).expect("initialize schema");
        let task = task(30 * 60, Weekday::Sat, 12 * 3600);

        let outcome = process_task(
            &connection,
            &[delayed_workday_rule(), always_rule()],
            &task,
            "codex-sqlite",
            1_000,
            86_400,
        )
        .expect("process task");

        assert_eq!(outcome.status, NotificationEventStatus::Delayed);
        let pending = events::list_pending_delayed_task_ids(&connection, "workday-short")
            .expect("list delayed tasks");
        assert_eq!(pending, vec!["task-1".to_string()]);
    }

    #[test]
    fn process_task_records_discarded_event_without_delay_queue() {
        let connection = Connection::open_in_memory().expect("open database");
        schema::initialize(&connection).expect("initialize schema");
        let task = task(30 * 60, Weekday::Sat, 12 * 3600);

        let outcome = process_task(
            &connection,
            &[discard_workday_rule()],
            &task,
            "codex-sqlite",
            1_000,
            86_400,
        )
        .expect("process task");

        assert_eq!(outcome.status, NotificationEventStatus::Discarded);
        let events = events::list_events(&connection).expect("list events");
        assert_eq!(events[0].status, NotificationEventStatus::Discarded);
        let pending = events::list_pending_delayed_task_ids(&connection, "workday-short")
            .expect("list delayed tasks");
        assert!(pending.is_empty());
    }

    #[test]
    fn process_task_records_ignored_event_when_no_duration_rule_matches() {
        let connection = Connection::open_in_memory().expect("open database");
        schema::initialize(&connection).expect("initialize schema");
        let task = task(30, Weekday::Mon, 9 * 3600);

        let outcome = process_task(
            &connection,
            &[delayed_workday_rule()],
            &task,
            "codex-sqlite",
            1_000,
            86_400,
        )
        .expect("process task");

        assert_eq!(outcome.status, NotificationEventStatus::Ignored);
        let events = events::list_events(&connection).expect("list events");
        assert_eq!(events[0].decision, NotificationDecision::Ignore);
        assert_eq!(events[0].status, NotificationEventStatus::Ignored);
    }
}
