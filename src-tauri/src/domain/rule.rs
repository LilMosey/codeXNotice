use super::task::Weekday;

#[derive(Debug, Clone)]
pub struct NotificationRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub duration: DurationCondition,
    pub time_window: TimeWindowCondition,
    pub outside_window: OutsideWindowPolicy,
}

#[derive(Debug, Clone)]
pub enum DurationCondition {
    Any,
    Ranges(Vec<DurationRange>),
}

#[derive(Debug, Clone)]
pub struct DurationRange {
    pub min_seconds: u64,
    pub max_seconds: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum TimeWindowCondition {
    Always,
    Windows(Vec<TimeWindow>),
}

#[derive(Debug, Clone)]
pub struct TimeWindow {
    pub weekdays: Vec<Weekday>,
    pub start_seconds: u32,
    pub end_seconds: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutsideWindowPolicy {
    Discard,
    Delay,
}
