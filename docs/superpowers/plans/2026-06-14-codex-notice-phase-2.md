# CodeX Notice 阶段 2 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 CodeX Notice 添加本地 SQLite 存储地基，支持数据库初始化、默认规则创建、规则持久化读取和规则排序。

**Architecture:** 新增 `storage` 模块，统一管理 SQLite schema、迁移和规则仓储。复杂规则字段先用 JSON 存储，保持 v1 迭代灵活；规则引擎仍使用领域模型，不直接依赖数据库行结构。

**Tech Stack:** Rust、rusqlite、serde、serde_json、tempfile、Rust 单元测试。

---

## 文件结构

阶段 2 创建或修改这些文件：

- `src-tauri/src/lib.rs`：导出 `storage` 模块。
- `src-tauri/src/domain/rule.rs`：为规则模型补充序列化、反序列化和相等比较能力。
- `src-tauri/src/domain/task.rs`：为星期枚举补充序列化、反序列化和调试比较能力。
- `src-tauri/src/storage/mod.rs`：导出存储子模块。
- `src-tauri/src/storage/error.rs`：定义存储错误。
- `src-tauri/src/storage/schema.rs`：创建数据库表和默认数据。
- `src-tauri/src/storage/rules.rs`：规则仓储，负责保存和读取规则。

## Task 1: 规则模型支持序列化和比较

**Files:**

- Modify: `src-tauri/src/domain/rule.rs`
- Modify: `src-tauri/src/domain/task.rs`

- [ ] **Step 1: 修改领域模型 derive**

`src-tauri/src/domain/task.rs` 中修改为：

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct TaskRecord {
    pub id: String,
    pub title: String,
    pub duration_seconds: u64,
    pub completed_at_weekday: Weekday,
    pub completed_at_seconds: u32,
    pub success: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}
```

`src-tauri/src/domain/rule.rs` 中修改为：

```rust
use serde::{Deserialize, Serialize};

use super::task::Weekday;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub duration: DurationCondition,
    pub time_window: TimeWindowCondition,
    pub outside_window: OutsideWindowPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DurationCondition {
    Any,
    Ranges(Vec<DurationRange>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurationRange {
    pub min_seconds: u64,
    pub max_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeWindowCondition {
    Always,
    Windows(Vec<TimeWindow>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeWindow {
    pub weekdays: Vec<Weekday>,
    pub start_seconds: u32,
    pub end_seconds: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutsideWindowPolicy {
    Discard,
    Delay,
}
```

- [ ] **Step 2: 运行现有测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: 现有 8 个测试仍然通过。

## Task 2: 用 TDD 添加数据库 schema 和默认规则

**Files:**

- Create: `src-tauri/src/storage/mod.rs`
- Create: `src-tauri/src/storage/error.rs`
- Create: `src-tauri/src/storage/schema.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 写失败测试**

`src-tauri/src/storage/mod.rs` 写入：

```rust
pub mod error;
pub mod schema;
pub mod rules;
```

`src-tauri/src/storage/error.rs` 写入：

```rust
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
```

`src-tauri/src/storage/schema.rs` 写入测试：

```rust
use rusqlite::Connection;

use crate::storage::error::StorageError;

pub fn initialize(_connection: &Connection) -> Result<(), StorageError> {
    unimplemented!("schema initialization is implemented by this task")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_creates_required_tables() {
        let connection = Connection::open_in_memory().expect("open in-memory database");

        initialize(&connection).expect("initialize schema");

        let table_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('app_settings', 'notification_rules')",
                [],
                |row| row.get(0),
            )
            .expect("count tables");

        assert_eq!(table_count, 2);
    }

    #[test]
    fn initialize_creates_default_rule_once() {
        let connection = Connection::open_in_memory().expect("open in-memory database");

        initialize(&connection).expect("initialize schema");
        initialize(&connection).expect("initialize schema again");

        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM notification_rules", [], |row| row.get(0))
            .expect("count rules");
        let rule_name: String = connection
            .query_row(
                "SELECT name FROM notification_rules WHERE rule_order = 0",
                [],
                |row| row.get(0),
            )
            .expect("read default rule");

        assert_eq!(count, 1);
        assert_eq!(rule_name, "默认规则");
    }
}
```

`src-tauri/src/lib.rs` 增加：

```rust
pub mod storage;
```

- [ ] **Step 2: 运行测试并确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml storage::schema::tests -- --nocapture`

Expected: 测试失败，原因是 `initialize` 尚未实现。

- [ ] **Step 3: 实现 schema 和默认规则**

将 `src-tauri/src/storage/schema.rs` 中的 `initialize` 替换为：

```rust
use rusqlite::{params, Connection};

use crate::domain::rule::{DurationCondition, NotificationRule, OutsideWindowPolicy, TimeWindowCondition};
use crate::storage::error::StorageError;

pub fn initialize(connection: &Connection) -> Result<(), StorageError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS notification_rules (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            enabled INTEGER NOT NULL,
            rule_order INTEGER NOT NULL UNIQUE,
            duration_json TEXT NOT NULL,
            time_window_json TEXT NOT NULL,
            outside_window TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        "#,
    )?;

    let existing_rules: i64 = connection.query_row(
        "SELECT COUNT(*) FROM notification_rules",
        [],
        |row| row.get(0),
    )?;

    if existing_rules == 0 {
        let default_rule = NotificationRule {
            id: "default-rule".to_string(),
            name: "默认规则".to_string(),
            enabled: true,
            duration: DurationCondition::Any,
            time_window: TimeWindowCondition::Always,
            outside_window: OutsideWindowPolicy::Discard,
        };

        connection.execute(
            r#"
            INSERT INTO notification_rules (
                id, name, enabled, rule_order, duration_json, time_window_json,
                outside_window, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, strftime('%s','now'), strftime('%s','now'))
            "#,
            params![
                default_rule.id,
                default_rule.name,
                1,
                0,
                serde_json::to_string(&default_rule.duration)?,
                serde_json::to_string(&default_rule.time_window)?,
                serde_json::to_string(&default_rule.outside_window)?,
            ],
        )?;
    }

    Ok(())
}
```

- [ ] **Step 4: 运行测试并确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml storage::schema::tests -- --nocapture`

Expected: schema 测试通过。

## Task 3: 用 TDD 添加规则仓储读取和保存

**Files:**

- Create: `src-tauri/src/storage/rules.rs`

- [ ] **Step 1: 写失败测试**

`src-tauri/src/storage/rules.rs` 写入：

```rust
use rusqlite::Connection;

use crate::domain::rule::NotificationRule;
use crate::storage::error::StorageError;

pub fn list_rules(_connection: &Connection) -> Result<Vec<NotificationRule>, StorageError> {
    unimplemented!("rule listing is implemented by this task")
}

pub fn replace_rules(_connection: &mut Connection, _rules: &[NotificationRule]) -> Result<(), StorageError> {
    unimplemented!("rule replacement is implemented by this task")
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
```

- [ ] **Step 2: 运行测试并确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml storage::rules::tests -- --nocapture`

Expected: 测试失败，原因是 `list_rules` 和 `replace_rules` 尚未实现。

- [ ] **Step 3: 实现规则仓储**

将 `src-tauri/src/storage/rules.rs` 的函数实现替换为：

```rust
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

pub fn replace_rules(connection: &mut Connection, rules: &[NotificationRule]) -> Result<(), StorageError> {
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
```

- [ ] **Step 4: 运行测试并确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml storage::rules::tests -- --nocapture`

Expected: 规则仓储测试通过。

## Task 4: 阶段 2 验证和提交

**Files:**

- Review all files created or modified in tasks 1-3.

- [ ] **Step 1: 运行 Rust 测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: 所有 Rust 测试通过。

- [ ] **Step 2: 运行前端构建**

Run: `npm run build`

Expected: TypeScript 和 Vite 构建通过。

- [ ] **Step 3: 检查 Git 状态**

Run: `git status --short`

Expected: 只看到阶段 2 相关源码和文档变更。

- [ ] **Step 4: 提交**

Run:

```bash
git add docs/superpowers/plans/2026-06-14-codex-notice-phase-2.md src-tauri/src src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat: add local rule storage"
```

Expected: 创建阶段 2 提交。

## 自检

设计文档要求在阶段 2 的覆盖情况：

- 本地 SQLite 存储：Task 2 覆盖。
- 用户规则持久化：Task 3 覆盖。
- 默认规则：Task 2 和 Task 3 覆盖。
- 规则顺序：Task 3 覆盖。
- 完整 UI、钉钉发送、macOS 通知、延迟批次持久化：保留到后续阶段。

