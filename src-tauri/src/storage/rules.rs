use rusqlite::{params, Connection};

use crate::domain::rule::{
    DurationCondition, NotificationRule, OutsideWindowPolicy, TimeWindowCondition,
};
use crate::storage::error::StorageError;

pub fn list_rules(connection: &Connection) -> Result<Vec<NotificationRule>, StorageError> {
    let mut statement = connection.prepare(
        r#"
        SELECT id, name, enabled, duration_json, time_window_json, outside_window
        FROM notification_rules
        ORDER BY rule_order ASC
        "#,
    )?;

    let rows = statement.query_map([], |row| {
        let duration_json: String = row.get(3)?;
        let time_window_json: String = row.get(4)?;
        let outside_window_json: String = row.get(5)?;

        Ok(StoredRuleRow {
            id: row.get(0)?,
            name: row.get(1)?,
            enabled: row.get::<_, i64>(2)? == 1,
            duration_json,
            time_window_json,
            outside_window_json,
        })
    })?;

    let mut rules = Vec::new();
    for row in rows {
        let row = row?;
        rules.push(NotificationRule {
            id: row.id,
            name: row.name,
            enabled: row.enabled,
            duration: serde_json::from_str::<DurationCondition>(&row.duration_json)?,
            time_window: serde_json::from_str::<TimeWindowCondition>(&row.time_window_json)?,
            outside_window: serde_json::from_str::<OutsideWindowPolicy>(&row.outside_window_json)?,
        });
    }

    Ok(rules)
}

pub fn replace_rules(
    connection: &mut Connection,
    rules: &[NotificationRule],
) -> Result<(), StorageError> {
    let transaction = connection.transaction()?;
    transaction.execute("DELETE FROM notification_rules", [])?;

    for (index, rule) in rules.iter().enumerate() {
        transaction.execute(
            r#"
            INSERT INTO notification_rules (
                id, name, enabled, rule_order, duration_json, time_window_json,
                outside_window, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, strftime('%s','now'), strftime('%s','now'))
            "#,
            params![
                rule.id,
                rule.name,
                if rule.enabled { 1 } else { 0 },
                index as i64,
                serde_json::to_string(&rule.duration)?,
                serde_json::to_string(&rule.time_window)?,
                serde_json::to_string(&rule.outside_window)?,
            ],
        )?;
    }

    transaction.commit()?;
    Ok(())
}

struct StoredRuleRow {
    id: String,
    name: String,
    enabled: bool,
    duration_json: String,
    time_window_json: String,
    outside_window_json: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::rule::{
        DurationCondition, DurationRange, OutsideWindowPolicy, TimeWindow, TimeWindowCondition,
    };
    use crate::domain::task::Weekday;
    use crate::storage::schema;

    fn custom_rule(id: &str, name: &str) -> NotificationRule {
        NotificationRule {
            id: id.to_string(),
            name: name.to_string(),
            enabled: true,
            duration: DurationCondition::Ranges(vec![DurationRange {
                min_seconds: 60,
                max_seconds: Some(50 * 60),
            }]),
            time_window: TimeWindowCondition::Windows(vec![TimeWindow {
                weekdays: vec![Weekday::Mon, Weekday::Tue],
                start_seconds: 8 * 3600,
                end_seconds: 20 * 3600,
            }]),
            outside_window: OutsideWindowPolicy::Delay,
        }
    }

    #[test]
    fn list_rules_returns_default_rule_after_schema_initialization() {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        schema::initialize(&connection).expect("initialize schema");

        let rules = list_rules(&connection).expect("list rules");

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, "default-rule");
        assert_eq!(rules[0].name, "默认规则");
    }

    #[test]
    fn replace_rules_preserves_order_and_round_trips_rule_json() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        schema::initialize(&connection).expect("initialize schema");
        let rules = vec![
            custom_rule("rule-1", "工作日中短任务"),
            custom_rule("rule-2", "周末任务"),
        ];

        replace_rules(&mut connection, &rules).expect("replace rules");

        let saved_rules = list_rules(&connection).expect("list rules");
        assert_eq!(saved_rules, rules);
    }
}
