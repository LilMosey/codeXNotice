# CodeX Notice 阶段 1 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 建立 CodeX Notice 的可测试项目骨架，并实现第一版核心规则引擎、延迟合并摘要和 Codex SQLite 检测适配器雏形。

**Architecture:** 阶段 1 先搭建 Tauri + Rust + React/Vite 的工程结构，但优先把 Rust 后台核心逻辑做成可单元测试的纯模块。UI 只提供最小可运行页面，完整 Rules/Channels/History/Diagnostics 页面放到后续阶段。

**Tech Stack:** Tauri 2、Rust、React、TypeScript、Vite、SQLite 读取能力、Rust 单元测试。

---

## 文件结构

阶段 1 创建或修改这些文件：

- `package.json`：前端和 Tauri 脚本入口。
- `index.html`：Vite 入口 HTML。
- `src/main.tsx`：React 入口。
- `src/App.tsx`：最小应用页面。
- `src/styles.css`：基础样式。
- `src-tauri/Cargo.toml`：Rust crate 和依赖。
- `src-tauri/tauri.conf.json`：Tauri 应用配置。
- `src-tauri/src/main.rs`：Tauri 入口。
- `src-tauri/src/lib.rs`：核心模块导出。
- `src-tauri/src/domain/mod.rs`：领域模块导出。
- `src-tauri/src/domain/task.rs`：任务记录模型。
- `src-tauri/src/domain/rule.rs`：通知规则模型。
- `src-tauri/src/domain/decision.rs`：规则匹配结果。
- `src-tauri/src/rules.rs`：规则引擎。
- `src-tauri/src/scheduler.rs`：延迟合并摘要逻辑。
- `src-tauri/src/detection/mod.rs`：检测模块导出。
- `src-tauri/src/detection/codex_sqlite.rs`：Codex SQLite 检测适配器雏形。

## Task 1: 创建最小项目骨架

**Files:**

- Create: `package.json`
- Create: `index.html`
- Create: `src/main.tsx`
- Create: `src/App.tsx`
- Create: `src/styles.css`
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`

- [ ] **Step 1: 创建前端最小文件**

写入 `package.json`：

```json
{
  "name": "codex-notice",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "tauri": "tauri",
    "test:rust": "cargo test --manifest-path src-tauri/Cargo.toml"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "react": "^18.3.1",
    "react-dom": "^18.3.1"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.0.0",
    "@types/react": "^18.3.3",
    "@types/react-dom": "^18.3.0",
    "typescript": "^5.5.4",
    "vite": "^5.4.0"
  }
}
```

写入 `index.html`：

```html
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>CodeX Notice</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

写入 `src/main.tsx`：

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
```

写入 `src/App.tsx`：

```tsx
export default function App() {
  return (
    <main className="app-shell">
      <section className="intro">
        <p className="eyebrow">CodeX Notice</p>
        <h1>Codex 任务完成后自动通知</h1>
        <p>
          第一版将支持当前机器 Codex Desktop 任务检测、优先级通知规则、钉钉通知和 macOS 本地通知。
        </p>
      </section>
    </main>
  );
}
```

写入 `src/styles.css`：

```css
:root {
  color: #18212f;
  background: #f7f8fb;
  font-family:
    Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI",
    sans-serif;
}

body {
  margin: 0;
}

.app-shell {
  min-height: 100vh;
  display: grid;
  place-items: center;
  padding: 32px;
}

.intro {
  width: min(680px, 100%);
}

.eyebrow {
  margin: 0 0 12px;
  color: #2f6f73;
  font-size: 14px;
  font-weight: 700;
  text-transform: uppercase;
}

h1 {
  margin: 0 0 16px;
  font-size: 34px;
  line-height: 1.15;
}

p {
  font-size: 16px;
  line-height: 1.7;
}
```

- [ ] **Step 2: 创建 Rust/Tauri 最小文件**

写入 `src-tauri/Cargo.toml`：

```toml
[package]
name = "codex-notice"
version = "0.1.0"
description = "Codex task completion notifications"
authors = ["CodeX Notice contributors"]
edition = "2021"

[lib]
name = "codex_notice"
path = "src/lib.rs"

[[bin]]
name = "codex-notice"
path = "src/main.rs"

[dependencies]
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tauri = { version = "2", features = [] }
thiserror = "1"

[dev-dependencies]
tempfile = "3"
```

写入 `src-tauri/tauri.conf.json`：

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "CodeX Notice",
  "version": "0.1.0",
  "identifier": "cn.lilmosey.codexnotice",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "CodeX Notice",
        "width": 960,
        "height": 680
      }
    ]
  },
  "bundle": {
    "active": true,
    "targets": ["app", "dmg"],
    "icon": []
  }
}
```

写入 `src-tauri/src/lib.rs`：

```rust
pub mod detection;
pub mod domain;
pub mod rules;
pub mod scheduler;
```

写入 `src-tauri/src/main.rs`：

```rust
fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to run CodeX Notice");
}
```

- [ ] **Step 3: 运行基础检查**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: 如果依赖已可用，命令编译通过并显示 `test result: ok`；如果网络受限导致依赖无法下载，记录错误并请求网络权限或改用本机已有模板。

## Task 2: 用 TDD 实现规则优先级和耗时匹配

**Files:**

- Create: `src-tauri/src/domain/mod.rs`
- Create: `src-tauri/src/domain/task.rs`
- Create: `src-tauri/src/domain/rule.rs`
- Create: `src-tauri/src/domain/decision.rs`
- Create: `src-tauri/src/rules.rs`

- [ ] **Step 1: 写失败测试**

写入 `src-tauri/src/domain/mod.rs`：

```rust
pub mod decision;
pub mod rule;
pub mod task;
```

写入 `src-tauri/src/domain/task.rs`：

```rust
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
```

写入 `src-tauri/src/domain/rule.rs`：

```rust
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
```

写入 `src-tauri/src/domain/decision.rs`：

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationDecision {
    Ignore,
    SendNow { rule_id: String },
    Discard { rule_id: String },
    Delay { rule_id: String },
}
```

写入 `src-tauri/src/rules.rs`：

```rust
#[cfg(test)]
mod tests {
    use crate::domain::decision::NotificationDecision;
    use crate::domain::rule::{
        DurationCondition, DurationRange, NotificationRule, OutsideWindowPolicy,
        TimeWindowCondition,
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
}
```

- [ ] **Step 2: 运行测试并确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml rules::tests -- --nocapture`

Expected: 编译失败，原因是 `super::evaluate` 尚未定义。

- [ ] **Step 3: 写最小实现**

在 `src-tauri/src/rules.rs` 测试模块上方加入：

```rust
use crate::domain::decision::NotificationDecision;
use crate::domain::rule::{DurationCondition, NotificationRule};
use crate::domain::task::TaskRecord;

pub fn evaluate(rules: &[NotificationRule], task: &TaskRecord) -> NotificationDecision {
    for rule in rules.iter().filter(|rule| rule.enabled) {
        if duration_matches(&rule.duration, task.duration_seconds) {
            return NotificationDecision::SendNow {
                rule_id: rule.id.clone(),
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
```

- [ ] **Step 4: 运行测试并确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml rules::tests -- --nocapture`

Expected: 两个测试通过。

## Task 3: 用 TDD 实现时间窗口和窗口外策略

**Files:**

- Modify: `src-tauri/src/rules.rs`

- [ ] **Step 1: 写失败测试**

在 `src-tauri/src/rules.rs` 的测试模块中加入：

```rust
use crate::domain::rule::TimeWindow;

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
```

- [ ] **Step 2: 运行测试并确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml rules::tests -- --nocapture`

Expected: 新测试失败，因为 `evaluate` 还没有判断时间窗口。

- [ ] **Step 3: 写最小实现**

将 `src-tauri/src/rules.rs` 实现替换为：

```rust
use crate::domain::decision::NotificationDecision;
use crate::domain::rule::{
    DurationCondition, NotificationRule, OutsideWindowPolicy, TimeWindow, TimeWindowCondition,
};
use crate::domain::task::{TaskRecord, Weekday};

pub fn evaluate(rules: &[NotificationRule], task: &TaskRecord) -> NotificationDecision {
    for rule in rules.iter().filter(|rule| rule.enabled) {
        if duration_matches(&rule.duration, task.duration_seconds) {
            if time_window_matches(&rule.time_window, task.completed_at_weekday, task.completed_at_seconds) {
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
        TimeWindowCondition::Windows(windows) => {
            windows.iter().any(|window| single_window_matches(window, weekday, seconds))
        }
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
```

- [ ] **Step 4: 运行测试并确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml rules::tests -- --nocapture`

Expected: 所有规则测试通过。

## Task 4: 用 TDD 实现延迟合并摘要

**Files:**

- Create: `src-tauri/src/scheduler.rs`

- [ ] **Step 1: 写失败测试**

写入 `src-tauri/src/scheduler.rs`：

```rust
use crate::domain::task::{TaskRecord, Weekday};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeSummary {
    pub rule_id: String,
    pub task_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub longest_duration_seconds: u64,
    pub visible_titles: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
```

- [ ] **Step 2: 运行测试并确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml scheduler::tests -- --nocapture`

Expected: 编译失败，原因是 `build_merge_summary` 尚未定义。

- [ ] **Step 3: 写最小实现**

在 `src-tauri/src/scheduler.rs` 中测试模块上方加入：

```rust
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
```

- [ ] **Step 4: 运行测试并确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml scheduler::tests -- --nocapture`

Expected: 两个延迟合并摘要测试通过。

## Task 5: 用 TDD 实现 Codex SQLite 检测适配器雏形

**Files:**

- Create: `src-tauri/src/detection/mod.rs`
- Create: `src-tauri/src/detection/codex_sqlite.rs`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: 增加 SQLite 依赖**

修改 `src-tauri/Cargo.toml` 的 `[dependencies]`，增加：

```toml
rusqlite = { version = "0.31", features = ["bundled"] }
```

- [ ] **Step 2: 写失败测试**

写入 `src-tauri/src/detection/mod.rs`：

```rust
pub mod codex_sqlite;
```

写入 `src-tauri/src/detection/codex_sqlite.rs`：

```rust
#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use tempfile::NamedTempFile;

    #[test]
    fn detects_completed_agent_jobs_once_from_sqlite() {
        let db = NamedTempFile::new().expect("create temp database");
        let connection = Connection::open(db.path()).expect("open temp database");
        connection
            .execute_batch(
                r#"
                CREATE TABLE agent_jobs (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    status TEXT NOT NULL,
                    instruction TEXT NOT NULL,
                    output_schema_json TEXT,
                    input_headers_json TEXT NOT NULL,
                    input_csv_path TEXT NOT NULL,
                    output_csv_path TEXT NOT NULL,
                    auto_export INTEGER NOT NULL DEFAULT 1,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL,
                    started_at INTEGER,
                    completed_at INTEGER,
                    last_error TEXT,
                    max_runtime_seconds INTEGER
                );

                INSERT INTO agent_jobs (
                    id, name, status, instruction, input_headers_json, input_csv_path,
                    output_csv_path, created_at, updated_at, started_at, completed_at, last_error
                ) VALUES (
                    'job-1', 'Long Codex Task', 'completed', 'Do work', '{}', '', '',
                    1000, 4000, 1000, 4000, NULL
                );
                "#,
            )
            .expect("seed database");

        let tasks = super::detect_completed_agent_jobs(db.path()).expect("detect jobs");

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "job-1");
        assert_eq!(tasks[0].title, "Long Codex Task");
        assert_eq!(tasks[0].duration_seconds, 3);
        assert!(tasks[0].success);
    }
}
```

- [ ] **Step 3: 运行测试并确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml detection::codex_sqlite::tests -- --nocapture`

Expected: 编译失败，原因是 `detect_completed_agent_jobs` 尚未定义。

- [ ] **Step 4: 写最小实现**

在 `src-tauri/src/detection/codex_sqlite.rs` 测试模块上方加入：

```rust
use std::path::Path;

use rusqlite::Connection;

use crate::domain::task::{TaskRecord, Weekday};

#[derive(Debug, thiserror::Error)]
pub enum CodexSqliteError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

pub fn detect_completed_agent_jobs(path: &Path) -> Result<Vec<TaskRecord>, CodexSqliteError> {
    let connection = Connection::open(path)?;
    let mut statement = connection.prepare(
        r#"
        SELECT id, name, status, started_at, completed_at, last_error
        FROM agent_jobs
        WHERE completed_at IS NOT NULL
        "#,
    )?;

    let rows = statement.query_map([], |row| {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let status: String = row.get(2)?;
        let started_at: Option<i64> = row.get(3)?;
        let completed_at: Option<i64> = row.get(4)?;
        let last_error: Option<String> = row.get(5)?;

        let duration_seconds = match (started_at, completed_at) {
            (Some(started), Some(completed)) if completed >= started => {
                ((completed - started) / 1000) as u64
            }
            _ => 0,
        };

        Ok(TaskRecord {
            id,
            title: name,
            duration_seconds,
            completed_at_weekday: Weekday::Mon,
            completed_at_seconds: 0,
            success: status == "completed" && last_error.is_none(),
        })
    })?;

    rows.collect::<Result<Vec<_>, _>>().map_err(CodexSqliteError::from)
}
```

- [ ] **Step 5: 运行测试并确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml detection::codex_sqlite::tests -- --nocapture`

Expected: SQLite 检测测试通过。

## Task 6: 阶段 1 验证

**Files:**

- Review all files created in tasks 1-5.

- [ ] **Step 1: 运行 Rust 测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: 所有 Rust 测试通过。

- [ ] **Step 2: 如果依赖已安装，运行前端构建**

Run: `npm run build`

Expected: TypeScript 和 Vite 构建通过。

- [ ] **Step 3: 记录 git 状态**

Run: `git status --short`

Expected: 如果仓库已初始化，看到阶段 1 新增文件；如果目录还不是 git 仓库，记录该事实，不强行提交。

## 自检

设计文档要求在阶段 1 的覆盖情况：

- Tauri + Rust + Web UI 骨架：Task 1 覆盖。
- 有序规则和第一命中优先级：Task 2 覆盖。
- 通知时间窗口和窗口外策略：Task 3 覆盖。
- 延迟合并通知的 10 条展示限制：Task 4 覆盖。
- Codex SQLite 检测适配器雏形：Task 5 覆盖。
- 完整 UI、钉钉发送、macOS 通知、打包：保留到后续阶段。

