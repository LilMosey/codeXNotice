use crate::domain::decision::NotificationDecision;
use crate::domain::rule::{
    DurationCondition, NotificationRule, OutsideWindowPolicy, TimeWindow, TimeWindowCondition,
};
use crate::domain::task::{TaskRecord, Weekday};

pub fn evaluate(rules: &[NotificationRule], task: &TaskRecord) -> NotificationDecision {
    for rule in rules.iter().filter(|rule| rule.enabled) {
        if duration_matches(&rule.duration, task.duration_seconds) {
            if time_window_matches(
                &rule.time_window,
                task.completed_at_weekday,
                task.completed_at_seconds,
            ) {
                return NotificationDecision::SendNow {
                    rule_id: rule.id.clone(),
                };
            }

            return match rule.outside_window {
                OutsideWindowPolicy::Discard => NotificationDecision::Discard {
                    rule_id: rule.id.clone(),
                },
                OutsideWindowPolicy::Delay => NotificationDecision::Delay {
                    rule_id: rule.id.clone(),
                },
            };
        }
    }

    NotificationDecision::Ignore
}

fn duration_matches(condition: &DurationCondition, duration_seconds: u64) -> bool {
    match condition {
        DurationCondition::Any => true,
        DurationCondition::Ranges(ranges) => ranges.iter().any(|range| {
            duration_seconds >= range.min_seconds
                && range
                    .max_seconds
                    .map(|max_seconds| duration_seconds <= max_seconds)
                    .unwrap_or(true)
        }),
    }
}

fn time_window_matches(condition: &TimeWindowCondition, weekday: Weekday, seconds: u32) -> bool {
    match condition {
        TimeWindowCondition::Always => true,
        TimeWindowCondition::Windows(windows) => windows
            .iter()
            .any(|window| single_window_matches(window, weekday, seconds)),
    }
}

fn single_window_matches(window: &TimeWindow, weekday: Weekday, seconds: u32) -> bool {
    if window.start_seconds <= window.end_seconds {
        window.weekdays.contains(&weekday)
            && seconds >= window.start_seconds
            && seconds <= window.end_seconds
    } else {
        window.weekdays.contains(&weekday)
            && (seconds >= window.start_seconds || seconds <= window.end_seconds)
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::decision::NotificationDecision;
    use crate::domain::rule::{
        DurationCondition, DurationRange, NotificationRule, OutsideWindowPolicy,
        TimeWindow, TimeWindowCondition,
    };
    use crate::domain::task::{TaskRecord, Weekday};

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

    fn rule(id: &str, duration: DurationCondition) -> NotificationRule {
        NotificationRule {
            id: id.to_string(),
            name: id.to_string(),
            enabled: true,
            duration,
            time_window: TimeWindowCondition::Always,
            outside_window: OutsideWindowPolicy::Discard,
        }
    }

    #[test]
    fn first_duration_matching_rule_wins() {
        let rules = vec![
            rule(
                "workday-short",
                DurationCondition::Ranges(vec![DurationRange {
                    min_seconds: 60,
                    max_seconds: Some(50 * 60),
                }]),
            ),
            rule("catch-all", DurationCondition::Any),
        ];

        let decision = super::evaluate(&rules, &task(30 * 60, Weekday::Sat, 12 * 3600));

        assert_eq!(
            decision,
            NotificationDecision::SendNow {
                rule_id: "workday-short".to_string()
            }
        );
    }

    #[test]
    fn unmatched_duration_is_ignored() {
        let rules = vec![rule(
            "only-hour",
            DurationCondition::Ranges(vec![DurationRange {
                min_seconds: 60 * 60,
                max_seconds: Some(70 * 60),
            }]),
        )];

        let decision = super::evaluate(&rules, &task(30 * 60, Weekday::Mon, 9 * 3600));

        assert_eq!(decision, NotificationDecision::Ignore);
    }

    fn rule_with_window(
        id: &str,
        duration: DurationCondition,
        weekdays: Vec<Weekday>,
        start_seconds: u32,
        end_seconds: u32,
        outside_window: OutsideWindowPolicy,
    ) -> NotificationRule {
        NotificationRule {
            id: id.to_string(),
            name: id.to_string(),
            enabled: true,
            duration,
            time_window: TimeWindowCondition::Windows(vec![TimeWindow {
                weekdays,
                start_seconds,
                end_seconds,
            }]),
            outside_window,
        }
    }

    #[test]
    fn matching_duration_outside_window_uses_selected_rule_delay_policy() {
        let rules = vec![
            rule_with_window(
                "workday-short",
                DurationCondition::Ranges(vec![DurationRange {
                    min_seconds: 60,
                    max_seconds: Some(50 * 60),
                }]),
                vec![Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri],
                8 * 3600,
                20 * 3600,
                OutsideWindowPolicy::Delay,
            ),
            rule("catch-all", DurationCondition::Any),
        ];

        let decision = super::evaluate(&rules, &task(30 * 60, Weekday::Sat, 12 * 3600));

        assert_eq!(
            decision,
            NotificationDecision::Delay {
                rule_id: "workday-short".to_string()
            }
        );
    }

    #[test]
    fn matching_duration_outside_window_uses_selected_rule_discard_policy() {
        let rules = vec![rule_with_window(
            "workday-short",
            DurationCondition::Ranges(vec![DurationRange {
                min_seconds: 60,
                max_seconds: Some(50 * 60),
            }]),
            vec![Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri],
            8 * 3600,
            20 * 3600,
            OutsideWindowPolicy::Discard,
        )];

        let decision = super::evaluate(&rules, &task(30 * 60, Weekday::Sat, 12 * 3600));

        assert_eq!(
            decision,
            NotificationDecision::Discard {
                rule_id: "workday-short".to_string()
            }
        );
    }

    #[test]
    fn cross_midnight_window_matches_after_midnight() {
        let rules = vec![rule_with_window(
            "night",
            DurationCondition::Any,
            vec![Weekday::Mon, Weekday::Tue],
            22 * 3600,
            2 * 3600,
            OutsideWindowPolicy::Discard,
        )];

        let decision = super::evaluate(&rules, &task(10 * 60, Weekday::Tue, 1 * 3600));

        assert_eq!(
            decision,
            NotificationDecision::SendNow {
                rule_id: "night".to_string()
            }
        );
    }
}
