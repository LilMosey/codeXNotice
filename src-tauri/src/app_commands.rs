use rusqlite::Connection;
use serde::Serialize;

use crate::domain::rule::NotificationRule;
use crate::runtime::default_app_database_path;
use crate::storage::{events, rules, schema};

#[derive(Debug, Serialize)]
pub struct AppDiagnostics {
    pub database_path: String,
    pub codex_directory: String,
    pub rule_count: usize,
    pub event_count: usize,
}

#[tauri::command]
pub fn get_rules() -> Result<Vec<NotificationRule>, String> {
    list_rules_at(&default_app_database_path())
}

#[tauri::command]
pub fn save_rules(notification_rules: Vec<NotificationRule>) -> Result<Vec<NotificationRule>, String> {
    save_rules_at(&default_app_database_path(), notification_rules)
}

#[tauri::command]
pub fn get_events() -> Result<Vec<events::NotificationEventRecord>, String> {
    let connection = open_app_connection(&default_app_database_path())?;
    events::list_events(&connection).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_diagnostics() -> Result<AppDiagnostics, String> {
    let database_path = default_app_database_path();
    let connection = open_app_connection(&database_path)?;
    let notification_rules = rules::list_rules(&connection).map_err(|error| error.to_string())?;
    let notification_events = events::list_events(&connection).map_err(|error| error.to_string())?;

    Ok(AppDiagnostics {
        database_path: database_path.display().to_string(),
        codex_directory: crate::runtime::default_codex_directory()
            .display()
            .to_string(),
        rule_count: notification_rules.len(),
        event_count: notification_events.len(),
    })
}

fn list_rules_at(path: &std::path::Path) -> Result<Vec<NotificationRule>, String> {
    let connection = open_app_connection(path)?;
    rules::list_rules(&connection).map_err(|error| error.to_string())
}

fn save_rules_at(
    path: &std::path::Path,
    notification_rules: Vec<NotificationRule>,
) -> Result<Vec<NotificationRule>, String> {
    let mut connection = open_app_connection(path)?;
    rules::replace_rules(&mut connection, &notification_rules).map_err(|error| error.to_string())?;
    rules::list_rules(&connection).map_err(|error| error.to_string())
}

fn open_app_connection(path: &std::path::Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    schema::initialize(&connection).map_err(|error| error.to_string())?;
    Ok(connection)
}

#[cfg(test)]
mod tests {
    use crate::domain::rule::{
        DurationCondition, NotificationRule, OutsideWindowPolicy, TimeWindowCondition,
    };

    fn default_rule() -> NotificationRule {
        NotificationRule {
            id: "rule-1".to_string(),
            name: "所有任务".to_string(),
            enabled: true,
            duration: DurationCondition::Any,
            time_window: TimeWindowCondition::Always,
            outside_window: OutsideWindowPolicy::Discard,
        }
    }

    #[test]
    fn save_rules_at_round_trips_rules_for_app_window() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let path = temp.path().join("app.sqlite");
        let rules = vec![default_rule()];

        let saved = super::save_rules_at(&path, rules.clone()).expect("save rules");
        let listed = super::list_rules_at(&path).expect("list rules");

        assert_eq!(saved, rules);
        assert_eq!(listed, rules);
    }
}
