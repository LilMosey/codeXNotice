#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationDecision {
    Ignore,
    SendNow { rule_id: String },
    Discard { rule_id: String },
    Delay { rule_id: String },
}
