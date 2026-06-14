use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationDecision {
    Ignore,
    SendNow { rule_id: String },
    Discard { rule_id: String },
    Delay { rule_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationEventStatus {
    Ignored,
    Pending,
    Sent,
    Discarded,
    Delayed,
    Failed,
}
