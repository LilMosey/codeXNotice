#[derive(Debug, Clone)]
pub struct TaskRecord {
    pub id: String,
    pub title: String,
    pub duration_seconds: u64,
    pub completed_at_weekday: Weekday,
    pub completed_at_seconds: u32,
    pub success: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}
