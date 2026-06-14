# CodeX Notice 阶段 5 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 添加 Codex 任务扫描器核心逻辑，从检测适配器读取任务，跳过已处理任务，并调用 processor 流水线生成通知事件。

**Architecture:** 新增 `scanner` 模块作为检测适配器和 processor 之间的编排层。新增存储能力判断任务是否已处理。阶段 5 只实现单次扫描函数，不启动真实后台定时器；后续 Tauri 后台任务可以定时调用该函数。

**Tech Stack:** Rust、rusqlite、tempfile、Rust 单元测试。

---

## 文件结构

阶段 5 创建或修改这些文件：

- `src-tauri/src/lib.rs`：导出 `scanner` 模块。
- `src-tauri/src/scanner.rs`：新增单次扫描处理逻辑。
- `src-tauri/src/storage/events.rs`：新增 `task_exists` 查询，供 scanner 去重。

## Task 1: 用 TDD 添加任务去重查询

**Files:**

- Modify: `src-tauri/src/storage/events.rs`

- [ ] **Step 1: 写失败测试**

在 `src-tauri/src/storage/events.rs` 测试模块中新增：

```rust
#[test]
fn task_exists_returns_true_after_task_is_recorded() {
    let connection = Connection::open_in_memory().expect("open database");
    schema::initialize(&connection).expect("initialize schema");

    assert!(!task_exists(&connection, "task-1").expect("check missing task"));

    record_task(&connection, &task("task-1"), "codex-sqlite").expect("record task");

    assert!(task_exists(&connection, "task-1").expect("check recorded task"));
}
```

- [ ] **Step 2: 运行测试并确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml storage::events::tests::task_exists_returns_true_after_task_is_recorded -- --nocapture`

Expected: 编译失败，原因是 `task_exists` 尚未定义。

- [ ] **Step 3: 实现 `task_exists`**

在 `src-tauri/src/storage/events.rs` 中新增：

```rust
pub fn task_exists(connection: &Connection, task_id: &str) -> Result<bool, StorageError> {
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM detected_tasks WHERE id = ?1",
        params![task_id],
        |row| row.get(0),
    )?;

    Ok(count > 0)
}
```

- [ ] **Step 4: 运行测试并确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml storage::events::tests::task_exists_returns_true_after_task_is_recorded -- --nocapture`

Expected: 测试通过。

## Task 2: 用 TDD 添加单次扫描器

**Files:**

- Create: `src-tauri/src/scanner.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 写失败测试**

`src-tauri/src/lib.rs` 增加：

```rust
pub mod scanner;
```

`src-tauri/src/scanner.rs` 写入：

```rust
use rusqlite::Connection;

use crate::domain::rule::NotificationRule;
use crate::domain::task::TaskRecord;
use crate::storage::error::StorageError;

pub struct ScanSummary {
    pub discovered: usize,
    pub skipped_existing: usize,
    pub processed: usize,
}

pub fn scan_tasks(
    _connection: &Connection,
    _rules: &[NotificationRule],
    _tasks: &[TaskRecord],
    _source: &str,
    _now_epoch_seconds: i64,
    _delay_ttl_seconds: i64,
) -> Result<ScanSummary, StorageError> {
    unimplemented!("scanner is implemented by this task")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::decision::NotificationEventStatus;
    use crate::domain::rule::{DurationCondition, NotificationRule, OutsideWindowPolicy, TimeWindowCondition};
    use crate::domain::task::Weekday;
    use crate::storage::{events, schema};

    fn task(id: &str) -> TaskRecord {
        TaskRecord {
            id: id.to_string(),
            title: format!("Task {id}"),
            duration_seconds: 120,
            completed_at_weekday: Weekday::Mon,
            completed_at_seconds: 9 * 3600,
            success: true,
        }
    }

    fn rule() -> NotificationRule {
        NotificationRule {
            id: "default-rule".to_string(),
            name: "默认规则".to_string(),
            enabled: true,
            duration: DurationCondition::Any,
            time_window: TimeWindowCondition::Always,
            outside_window: OutsideWindowPolicy::Discard,
        }
    }

    #[test]
    fn scan_tasks_processes_new_tasks_and_skips_existing_tasks() {
        let connection = Connection::open_in_memory().expect("open database");
        schema::initialize(&connection).expect("initialize schema");
        events::record_task(&connection, &task("task-1"), "codex-sqlite").expect("record existing task");
        let tasks = vec![task("task-1"), task("task-2"), task("task-3")];

        let summary = scan_tasks(&connection, &[rule()], &tasks, "codex-sqlite", 1_000, 86_400)
            .expect("scan tasks");

        assert_eq!(summary.discovered, 3);
        assert_eq!(summary.skipped_existing, 1);
        assert_eq!(summary.processed, 2);
        let events = events::list_events(&connection).expect("list events");
        assert_eq!(events.len(), 2);
        assert!(events.iter().all(|event| event.status == NotificationEventStatus::Pending));
    }
}
```

- [ ] **Step 2: 运行测试并确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml scanner::tests -- --nocapture`

Expected: 测试失败，原因是 `scan_tasks` 尚未实现。

- [ ] **Step 3: 实现扫描器**

实现 `scan_tasks`：

- 遍历输入任务
- 用 `events::task_exists` 跳过已处理任务
- 对新任务调用 `processor::process_task`
- 返回发现、跳过、处理数量

- [ ] **Step 4: 运行测试并确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml scanner::tests -- --nocapture`

Expected: scanner 测试通过。

## Task 3: 阶段 5 验证和提交

- [ ] **Step 1: 运行 Rust 测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: 所有 Rust 测试通过。

- [ ] **Step 2: 运行前端构建**

Run: `npm run build`

Expected: TypeScript 和 Vite 构建通过。

- [ ] **Step 3: 检查 Git 状态**

Run: `git status --short`

Expected: 只看到阶段 5 相关源码和文档变更。

- [ ] **Step 4: 提交**

Run:

```bash
git add docs/superpowers/plans/2026-06-14-codex-notice-phase-5.md src-tauri/src
git commit -m "feat: add task scanner"
```

Expected: 创建阶段 5 提交。

## 自检

设计文档要求在阶段 5 的覆盖情况：

- Codex 检测任务进入处理流水线：Task 2 覆盖单次扫描入口。
- 避免重复通知：Task 1 和 Task 2 覆盖。
- 真实后台定时器、读取真实 Codex SQLite 文件路径、Diagnostics UI：保留到后续阶段。

