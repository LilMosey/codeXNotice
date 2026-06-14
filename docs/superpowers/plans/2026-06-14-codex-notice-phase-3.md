# CodeX Notice 阶段 3 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 添加任务历史和通知决策持久化能力，为 History 页面、延迟合并队列和后续真实通知发送打基础。

**Architecture:** 扩展 SQLite schema，新增 `detected_tasks`、`notification_events`、`delayed_tasks` 三类数据。新增 `storage::events` 仓储，负责记录任务、记录通知决策、查询待延迟任务。规则匹配仍由 `rules` 模块负责，存储层只保存结果。

**Tech Stack:** Rust、rusqlite、serde、serde_json、tempfile、Rust 单元测试。

---

## 文件结构

阶段 3 创建或修改这些文件：

- `src-tauri/src/domain/decision.rs`：让通知决策支持序列化和状态落库。
- `src-tauri/src/domain/task.rs`：让任务记录支持相等比较，方便仓储测试。
- `src-tauri/src/storage/schema.rs`：新增任务历史、通知事件、延迟任务表。
- `src-tauri/src/storage/mod.rs`：导出 `events` 模块。
- `src-tauri/src/storage/events.rs`：新增任务和通知事件仓储。

## Task 1: 扩展领域模型

**Files:**

- Modify: `src-tauri/src/domain/decision.rs`
- Modify: `src-tauri/src/domain/task.rs`

- [ ] **Step 1: 修改 derive**

`src-tauri/src/domain/task.rs` 中 `TaskRecord` 修改为：

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRecord {
    pub id: String,
    pub title: String,
    pub duration_seconds: u64,
    pub completed_at_weekday: Weekday,
    pub completed_at_seconds: u32,
    pub success: bool,
}
```

`src-tauri/src/domain/decision.rs` 修改为：

```rust
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
```

- [ ] **Step 2: 运行现有测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: 现有测试仍然通过。

## Task 2: 用 TDD 扩展 schema

**Files:**

- Modify: `src-tauri/src/storage/schema.rs`

- [ ] **Step 1: 写失败测试**

在 `src-tauri/src/storage/schema.rs` 测试模块中新增：

```rust
#[test]
fn initialize_creates_history_and_delay_tables() {
    let connection = Connection::open_in_memory().expect("open in-memory database");

    initialize(&connection).expect("initialize schema");

    let table_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('detected_tasks', 'notification_events', 'delayed_tasks')",
            [],
            |row| row.get(0),
        )
        .expect("count tables");

    assert_eq!(table_count, 3);
}
```

- [ ] **Step 2: 运行测试并确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml storage::schema::tests::initialize_creates_history_and_delay_tables -- --nocapture`

Expected: 测试失败，因为新表尚未创建。

- [ ] **Step 3: 实现新表**

在 `initialize` 的 `execute_batch` 中追加：

```sql
CREATE TABLE IF NOT EXISTS detected_tasks (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    duration_seconds INTEGER NOT NULL,
    completed_weekday TEXT NOT NULL,
    completed_seconds INTEGER NOT NULL,
    success INTEGER NOT NULL,
    source TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS notification_events (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    rule_id TEXT,
    decision_json TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY(task_id) REFERENCES detected_tasks(id)
);

CREATE TABLE IF NOT EXISTS delayed_tasks (
    rule_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    queued_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    sent_at INTEGER,
    PRIMARY KEY(rule_id, task_id),
    FOREIGN KEY(task_id) REFERENCES detected_tasks(id)
);
```

- [ ] **Step 4: 运行测试并确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml storage::schema::tests -- --nocapture`

Expected: schema 测试全部通过。

## Task 3: 用 TDD 添加事件仓储

**Files:**

- Create: `src-tauri/src/storage/events.rs`
- Modify: `src-tauri/src/storage/mod.rs`

- [ ] **Step 1: 写失败测试**

`src-tauri/src/storage/mod.rs` 增加：

```rust
pub mod events;
```

`src-tauri/src/storage/events.rs` 写入：

```rust
use rusqlite::Connection;

use crate::domain::decision::{NotificationDecision, NotificationEventStatus};
use crate::domain::task::TaskRecord;
use crate::storage::error::StorageError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationEventRecord {
    pub id: String,
    pub task_id: String,
    pub rule_id: Option<String>,
    pub decision: NotificationDecision,
    pub status: NotificationEventStatus,
}

pub fn record_task(_connection: &Connection, _task: &TaskRecord, _source: &str) -> Result<(), StorageError> {
    unimplemented!("task recording is implemented by this task")
}

pub fn record_event(
    _connection: &Connection,
    _event_id: &str,
    _task_id: &str,
    _decision: &NotificationDecision,
    _status: NotificationEventStatus,
) -> Result<(), StorageError> {
    unimplemented!("event recording is implemented by this task")
}

pub fn list_events(_connection: &Connection) -> Result<Vec<NotificationEventRecord>, StorageError> {
    unimplemented!("event listing is implemented by this task")
}

pub fn queue_delayed_task(
    _connection: &Connection,
    _rule_id: &str,
    _task_id: &str,
    _queued_at: i64,
    _expires_at: i64,
) -> Result<(), StorageError> {
    unimplemented!("delayed task queueing is implemented by this task")
}

pub fn list_pending_delayed_task_ids(_connection: &Connection, _rule_id: &str) -> Result<Vec<String>, StorageError> {
    unimplemented!("pending delayed task listing is implemented by this task")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::Weekday;
    use crate::storage::schema;

    fn task(id: &str) -> TaskRecord {
        TaskRecord {
            id: id.to_string(),
            title: "Codex task".to_string(),
            duration_seconds: 1800,
            completed_at_weekday: Weekday::Mon,
            completed_at_seconds: 9 * 3600,
            success: true,
        }
    }

    #[test]
    fn records_task_and_notification_event() {
        let connection = Connection::open_in_memory().expect("open database");
        schema::initialize(&connection).expect("initialize schema");
        let task = task("task-1");

        record_task(&connection, &task, "codex-sqlite").expect("record task");
        record_event(
            &connection,
            "event-1",
            "task-1",
            &NotificationDecision::SendNow {
                rule_id: "default-rule".to_string(),
            },
            NotificationEventStatus::Pending,
        )
        .expect("record event");

        let events = list_events(&connection).expect("list events");

        assert_eq!(
            events,
            vec![NotificationEventRecord {
                id: "event-1".to_string(),
                task_id: "task-1".to_string(),
                rule_id: Some("default-rule".to_string()),
                decision: NotificationDecision::SendNow {
                    rule_id: "default-rule".to_string(),
                },
                status: NotificationEventStatus::Pending,
            }]
        );
    }

    #[test]
    fn delayed_tasks_are_listed_once_per_rule() {
        let connection = Connection::open_in_memory().expect("open database");
        schema::initialize(&connection).expect("initialize schema");
        record_task(&connection, &task("task-1"), "codex-sqlite").expect("record task");
        record_task(&connection, &task("task-2"), "codex-sqlite").expect("record task");

        queue_delayed_task(&connection, "rule-1", "task-1", 100, 200).expect("queue task");
        queue_delayed_task(&connection, "rule-1", "task-1", 100, 200).expect("queue task again");
        queue_delayed_task(&connection, "rule-1", "task-2", 100, 200).expect("queue second task");

        let pending = list_pending_delayed_task_ids(&connection, "rule-1").expect("list pending");

        assert_eq!(pending, vec!["task-1".to_string(), "task-2".to_string()]);
    }
}
```

- [ ] **Step 2: 运行测试并确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml storage::events::tests -- --nocapture`

Expected: 测试失败，原因是仓储函数尚未实现。

- [ ] **Step 3: 实现事件仓储**

实现 `record_task`、`record_event`、`list_events`、`queue_delayed_task`、`list_pending_delayed_task_ids`，使用 `serde_json` 存储枚举字段。

- [ ] **Step 4: 运行测试并确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml storage::events::tests -- --nocapture`

Expected: 事件仓储测试通过。

## Task 4: 阶段 3 验证和提交

- [ ] **Step 1: 运行 Rust 测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: 所有 Rust 测试通过。

- [ ] **Step 2: 运行前端构建**

Run: `npm run build`

Expected: TypeScript 和 Vite 构建通过。

- [ ] **Step 3: 检查 Git 状态**

Run: `git status --short`

Expected: 只看到阶段 3 相关源码和文档变更。

- [ ] **Step 4: 提交**

Run:

```bash
git add docs/superpowers/plans/2026-06-14-codex-notice-phase-3.md src-tauri/src
git commit -m "feat: persist notification events"
```

Expected: 创建阶段 3 提交。

## 自检

设计文档要求在阶段 3 的覆盖情况：

- 检测到的任务保存：Task 3 覆盖。
- 通知事件保存：Task 3 覆盖。
- 延迟任务队列雏形：Task 2 和 Task 3 覆盖。
- History 页面、真实通知发送、延迟批次发送状态：保留到后续阶段。

