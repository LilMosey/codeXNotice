# CodeX Notice 阶段 4 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 添加任务处理流水线，把检测到的任务、规则匹配、通知事件持久化和延迟队列入队串起来。

**Architecture:** 新增 `processor` 模块作为应用核心编排层。检测适配器负责产出 `TaskRecord`，规则模块负责返回 `NotificationDecision`，存储模块负责记录任务、事件和延迟队列。阶段 4 不发送真实通知，只记录 `Pending`、`Discarded`、`Delayed`、`Ignored` 状态。

**Tech Stack:** Rust、rusqlite、serde、Rust 单元测试。

---

## 文件结构

阶段 4 创建或修改这些文件：

- `src-tauri/src/lib.rs`：导出 `processor` 模块。
- `src-tauri/src/processor.rs`：新增任务处理流水线。
- `src-tauri/src/storage/events.rs`：新增按状态查询事件的辅助能力。

## Task 1: 用 TDD 添加任务处理流水线

**Files:**

- Create: `src-tauri/src/processor.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/storage/events.rs`

- [ ] **Step 1: 写失败测试**

`src-tauri/src/lib.rs` 增加：

```rust
pub mod processor;
```

`src-tauri/src/processor.rs` 写入：

```rust
use rusqlite::Connection;

use crate::domain::decision::NotificationEventStatus;
use crate::domain::rule::NotificationRule;
use crate::domain::task::TaskRecord;
use crate::storage::error::StorageError;

pub struct ProcessingOutcome {
    pub event_id: String,
    pub status: NotificationEventStatus,
}

pub fn process_task(
    _connection: &Connection,
    _rules: &[NotificationRule],
    _task: &TaskRecord,
    _source: &str,
    _now_epoch_seconds: i64,
    _delay_ttl_seconds: i64,
) -> Result<ProcessingOutcome, StorageError> {
    unimplemented!("task processing is implemented by this task")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::decision::{NotificationDecision, NotificationEventStatus};
    use crate::domain::rule::{
        DurationCondition, DurationRange, NotificationRule, OutsideWindowPolicy, TimeWindow,
        TimeWindowCondition,
    };
    use crate::domain::task::Weekday;
    use crate::storage::{events, schema};

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

    fn always_rule() -> NotificationRule {
        NotificationRule {
            id: "default-rule".to_string(),
            name: "默认规则".to_string(),
            enabled: true,
            duration: DurationCondition::Any,
            time_window: TimeWindowCondition::Always,
            outside_window: OutsideWindowPolicy::Discard,
        }
    }

    fn delayed_workday_rule() -> NotificationRule {
        NotificationRule {
            id: "workday-short".to_string(),
            name: "工作日中短任务".to_string(),
            enabled: true,
            duration: DurationCondition::Ranges(vec![DurationRange {
                min_seconds: 60,
                max_seconds: Some(50 * 60),
            }]),
            time_window: TimeWindowCondition::Windows(vec![TimeWindow {
                weekdays: vec![Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri],
                start_seconds: 8 * 3600,
                end_seconds: 20 * 3600,
            }]),
            outside_window: OutsideWindowPolicy::Delay,
        }
    }

    #[test]
    fn process_task_records_pending_event_when_rule_allows_send_now() {
        let connection = Connection::open_in_memory().expect("open database");
        schema::initialize(&connection).expect("initialize schema");
        let task = task(30 * 60, Weekday::Mon, 9 * 3600);

        let outcome = process_task(
            &connection,
            &[always_rule()],
            &task,
            "codex-sqlite",
            1_000,
            86_400,
        )
        .expect("process task");

        assert_eq!(outcome.status, NotificationEventStatus::Pending);
        let events = events::list_events(&connection).expect("list events");
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].decision,
            NotificationDecision::SendNow {
                rule_id: "default-rule".to_string()
            }
        );
        assert_eq!(events[0].status, NotificationEventStatus::Pending);
    }

    #[test]
    fn process_task_queues_delayed_event_for_selected_rule() {
        let connection = Connection::open_in_memory().expect("open database");
        schema::initialize(&connection).expect("initialize schema");
        let task = task(30 * 60, Weekday::Sat, 12 * 3600);

        let outcome = process_task(
            &connection,
            &[delayed_workday_rule(), always_rule()],
            &task,
            "codex-sqlite",
            1_000,
            86_400,
        )
        .expect("process task");

        assert_eq!(outcome.status, NotificationEventStatus::Delayed);
        let pending = events::list_pending_delayed_task_ids(&connection, "workday-short")
            .expect("list delayed tasks");
        assert_eq!(pending, vec!["task-1".to_string()]);
    }
}
```

- [ ] **Step 2: 运行测试并确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml processor::tests -- --nocapture`

Expected: 测试失败，原因是 `process_task` 尚未实现。

- [ ] **Step 3: 实现任务处理流水线**

实现 `process_task`：

- 调用 `events::record_task`
- 调用 `rules::evaluate`
- 把 `NotificationDecision` 映射为 `NotificationEventStatus`
- 调用 `events::record_event`
- 当状态是 `Delayed` 时调用 `events::queue_delayed_task`
- 返回 `ProcessingOutcome`

- [ ] **Step 4: 运行测试并确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml processor::tests -- --nocapture`

Expected: 处理流水线测试通过。

## Task 2: 阶段 4 验证和提交

- [ ] **Step 1: 运行 Rust 测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: 所有 Rust 测试通过。

- [ ] **Step 2: 运行前端构建**

Run: `npm run build`

Expected: TypeScript 和 Vite 构建通过。

- [ ] **Step 3: 检查 Git 状态**

Run: `git status --short`

Expected: 只看到阶段 4 相关源码和文档变更。

- [ ] **Step 4: 提交**

Run:

```bash
git add docs/superpowers/plans/2026-06-14-codex-notice-phase-4.md src-tauri/src
git commit -m "feat: add task processing pipeline"
```

Expected: 创建阶段 4 提交。

## 自检

设计文档要求在阶段 4 的覆盖情况：

- 检测任务进入规则判断：Task 1 覆盖。
- 通知决策落库：Task 1 覆盖。
- 延迟任务入队：Task 1 覆盖。
- 真实通知发送、延迟批次发送和 History UI：保留到后续阶段。

