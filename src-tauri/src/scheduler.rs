use crate::domain::task::TaskRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeSummary {
    pub rule_id: String,
    pub task_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub longest_duration_seconds: u64,
    pub visible_titles: Vec<String>,
}

pub fn build_merge_summary(rule_id: &str, tasks: &[TaskRecord]) -> MergeSummary {
    MergeSummary {
        rule_id: rule_id.to_string(),
        task_count: tasks.len(),
        success_count: tasks.iter().filter(|task| task.success).count(),
        failure_count: tasks.iter().filter(|task| !task.success).count(),
        longest_duration_seconds: tasks
            .iter()
            .map(|task| task.duration_seconds)
            .max()
            .unwrap_or(0),
        visible_titles: if tasks.len() <= 10 {
            tasks.iter().map(|task| task.title.clone()).collect()
        } else {
            Vec::new()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::Weekday;

    fn task(id: &str, title: &str, duration_seconds: u64, success: bool) -> TaskRecord {
        TaskRecord {
            id: id.to_string(),
            title: title.to_string(),
            duration_seconds,
            completed_at_weekday: Weekday::Mon,
            completed_at_seconds: 8 * 3600,
            success,
        }
    }

    #[test]
    fn summary_shows_titles_when_task_count_is_ten_or_less() {
        let tasks = vec![
            task("1", "Build feature", 600, true),
            task("2", "Fix test", 1200, false),
        ];

        let summary = super::build_merge_summary("rule-1", &tasks);

        assert_eq!(
            summary,
            MergeSummary {
                rule_id: "rule-1".to_string(),
                task_count: 2,
                success_count: 1,
                failure_count: 1,
                longest_duration_seconds: 1200,
                visible_titles: vec!["Build feature".to_string(), "Fix test".to_string()],
            }
        );
    }

    #[test]
    fn summary_hides_titles_when_task_count_is_more_than_ten() {
        let tasks: Vec<TaskRecord> = (0..11)
            .map(|index| task(&index.to_string(), &format!("Task {index}"), 60, true))
            .collect();

        let summary = super::build_merge_summary("rule-1", &tasks);

        assert_eq!(summary.task_count, 11);
        assert!(summary.visible_titles.is_empty());
    }
}
