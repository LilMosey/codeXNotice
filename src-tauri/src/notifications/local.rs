use std::process::Command;

use rusqlite::Connection;
use tauri_plugin_notification::NotificationExt;

use crate::domain::decision::NotificationEventStatus;
use crate::storage::error::StorageError;
use crate::storage::events;

pub trait LocalNotifier {
    fn notify(&self, title: &str, body: &str) -> Result<(), NotificationError>;
}

#[derive(Debug, thiserror::Error)]
pub enum NotificationError {
    #[error("notification command failed: {0}")]
    CommandFailed(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
}

pub struct DispatchSummary {
    pub attempted: usize,
    pub sent: usize,
    pub failed: usize,
}

pub struct MacOsNotifier;

impl LocalNotifier for MacOsNotifier {
    fn notify(&self, title: &str, body: &str) -> Result<(), NotificationError> {
        let script = format!(
            "display notification {} with title {}",
            apple_script_string(body),
            apple_script_string(title)
        );
        let output = Command::new("osascript").arg("-e").arg(script).output()?;

        if output.status.success() {
            Ok(())
        } else {
            Err(NotificationError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ))
        }
    }
}

pub struct TauriNotifier {
    app: tauri::AppHandle,
}

impl TauriNotifier {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl LocalNotifier for TauriNotifier {
    fn notify(&self, title: &str, body: &str) -> Result<(), NotificationError> {
        self.app
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show()
            .map_err(|error| NotificationError::CommandFailed(error.to_string()))
    }
}

pub fn dispatch_pending_notifications<N: LocalNotifier>(
    connection: &Connection,
    notifier: &N,
) -> Result<DispatchSummary, DispatchError> {
    let pending = events::list_events_by_status(connection, NotificationEventStatus::Pending)?;
    let mut sent = 0;
    let mut failed = 0;

    for event in &pending {
        let title = "CodeX Notice";
        let body = format!("Codex 任务已完成：{}", event.task_id);
        let next_status = match notifier.notify(title, &body) {
            Ok(()) => {
                sent += 1;
                NotificationEventStatus::Sent
            }
            Err(_) => {
                failed += 1;
                NotificationEventStatus::Failed
            }
        };
        events::update_event_status(connection, &event.id, next_status)?;
    }

    Ok(DispatchSummary {
        attempted: pending.len(),
        sent,
        failed,
    })
}

fn apple_script_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::domain::decision::NotificationDecision;
    use crate::domain::task::{TaskRecord, Weekday};
    use crate::storage::schema;

    struct RecordingNotifier {
        calls: RefCell<Vec<(String, String)>>,
        fail: bool,
    }

    impl RecordingNotifier {
        fn succeed() -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                fail: false,
            }
        }

        fn fail() -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                fail: true,
            }
        }
    }

    impl LocalNotifier for RecordingNotifier {
        fn notify(&self, title: &str, body: &str) -> Result<(), NotificationError> {
            self.calls
                .borrow_mut()
                .push((title.to_string(), body.to_string()));
            if self.fail {
                Err(NotificationError::CommandFailed("failed".to_string()))
            } else {
                Ok(())
            }
        }
    }

    fn task(id: &str) -> TaskRecord {
        TaskRecord {
            id: id.to_string(),
            title: "Codex task".to_string(),
            duration_seconds: 120,
            completed_at_weekday: Weekday::Mon,
            completed_at_seconds: 9 * 3600,
            success: true,
        }
    }

    fn seed_pending_event(connection: &Connection, event_id: &str, task_id: &str) {
        events::record_task(connection, &task(task_id), "codex-sqlite").expect("record task");
        events::record_event(
            connection,
            event_id,
            task_id,
            &NotificationDecision::SendNow {
                rule_id: "default-rule".to_string(),
            },
            NotificationEventStatus::Pending,
        )
        .expect("record event");
    }

    #[test]
    fn dispatch_pending_notifications_marks_sent_after_success() {
        let connection = Connection::open_in_memory().expect("open database");
        schema::initialize(&connection).expect("initialize schema");
        seed_pending_event(&connection, "event-1", "task-1");
        let notifier = RecordingNotifier::succeed();

        let summary =
            dispatch_pending_notifications(&connection, &notifier).expect("dispatch pending");

        assert_eq!(summary.attempted, 1);
        assert_eq!(summary.sent, 1);
        assert_eq!(summary.failed, 0);
        assert_eq!(notifier.calls.borrow().len(), 1);
        assert!(events::list_events_by_status(&connection, NotificationEventStatus::Pending)
            .expect("pending")
            .is_empty());
        assert_eq!(
            events::list_events_by_status(&connection, NotificationEventStatus::Sent)
                .expect("sent")
                .len(),
            1
        );
    }

    #[test]
    fn dispatch_pending_notifications_marks_failed_after_error() {
        let connection = Connection::open_in_memory().expect("open database");
        schema::initialize(&connection).expect("initialize schema");
        seed_pending_event(&connection, "event-1", "task-1");
        let notifier = RecordingNotifier::fail();

        let summary =
            dispatch_pending_notifications(&connection, &notifier).expect("dispatch pending");

        assert_eq!(summary.attempted, 1);
        assert_eq!(summary.sent, 0);
        assert_eq!(summary.failed, 1);
        assert_eq!(
            events::list_events_by_status(&connection, NotificationEventStatus::Failed)
                .expect("failed")
                .len(),
            1
        );
    }

    #[test]
    fn apple_script_string_escapes_quotes_and_backslashes() {
        assert_eq!(
            apple_script_string("a \"quote\" and \\ slash"),
            "\"a \\\"quote\\\" and \\\\ slash\""
        );
    }
}
