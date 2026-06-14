# CodeX Notice 阶段 6 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 添加真实 Codex SQLite 状态文件扫描入口，发现 `state_*.sqlite` 文件，读取已完成 `agent_jobs`，并交给 scanner 处理。

**Architecture:** `detection::codex_sqlite` 继续负责 Codex SQLite 读取和状态文件发现；`scanner` 负责编排多文件扫描和去重处理。阶段 6 仍不启动后台定时器，只提供一次性函数，方便后续 Tauri 后台任务调用。

**Tech Stack:** Rust、rusqlite、tempfile、Rust 单元测试。

---

## 文件结构

阶段 6 创建或修改这些文件：

- `src-tauri/src/detection/codex_sqlite.rs`：新增 `find_state_databases`，按目录发现 `state_*.sqlite`。
- `src-tauri/src/scanner.rs`：新增 `scan_codex_state_files`，遍历状态库并复用 `scan_tasks`。

## Task 1: 用 TDD 添加 Codex state 数据库发现

**Files:**

- Modify: `src-tauri/src/detection/codex_sqlite.rs`

- [ ] **Step 1: 写失败测试**

在 `src-tauri/src/detection/codex_sqlite.rs` 的测试模块中新增：

```rust
#[test]
fn finds_only_codex_state_sqlite_files() {
    let directory = tempfile::tempdir().expect("create temp directory");
    std::fs::write(directory.path().join("state_5.sqlite"), "").expect("write state db");
    std::fs::write(directory.path().join("state_5.sqlite-wal"), "").expect("write wal");
    std::fs::write(directory.path().join("logs_2.sqlite"), "").expect("write logs db");

    let files = super::find_state_databases(directory.path()).expect("find state databases");

    assert_eq!(files, vec![directory.path().join("state_5.sqlite")]);
}
```

- [ ] **Step 2: 运行测试并确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml detection::codex_sqlite::tests::finds_only_codex_state_sqlite_files -- --nocapture`

Expected: 编译失败，原因是 `find_state_databases` 尚未定义。

- [ ] **Step 3: 实现 `find_state_databases`**

实现要求：

- 读取传入目录
- 只返回文件名以 `state_` 开头且以 `.sqlite` 结尾的文件
- 按路径排序，保证测试和扫描顺序稳定

- [ ] **Step 4: 运行测试并确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml detection::codex_sqlite::tests::finds_only_codex_state_sqlite_files -- --nocapture`

Expected: 测试通过。

## Task 2: 用 TDD 添加多 state 文件扫描入口

**Files:**

- Modify: `src-tauri/src/scanner.rs`

- [ ] **Step 1: 写失败测试**

在 `src-tauri/src/scanner.rs` 测试模块中新增：

```rust
#[test]
fn scan_codex_state_files_reads_state_databases_and_processes_tasks() {
    let app_connection = Connection::open_in_memory().expect("open app database");
    schema::initialize(&app_connection).expect("initialize schema");
    let directory = tempfile::tempdir().expect("create temp directory");
    let codex_db = directory.path().join("state_5.sqlite");
    seed_codex_state_database(&codex_db);

    let summary = super::scan_codex_state_files(
        &app_connection,
        &[rule()],
        directory.path(),
        1_000,
        86_400,
    )
    .expect("scan codex state files");

    assert_eq!(summary.discovered, 1);
    assert_eq!(summary.processed, 1);
    let events = events::list_events(&app_connection).expect("list events");
    assert_eq!(events.len(), 1);
}
```

同时在测试模块中加入 helper：

```rust
fn seed_codex_state_database(path: &std::path::Path) {
    let connection = Connection::open(path).expect("open codex state database");
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
        .expect("seed codex database");
}
```

- [ ] **Step 2: 运行测试并确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml scanner::tests::scan_codex_state_files_reads_state_databases_and_processes_tasks -- --nocapture`

Expected: 编译失败，原因是 `scan_codex_state_files` 尚未定义。

- [ ] **Step 3: 实现 `scan_codex_state_files`**

实现要求：

- 调用 `codex_sqlite::find_state_databases`
- 对每个数据库调用 `detect_completed_agent_jobs`
- 把所有任务合并到一个列表
- 调用已有 `scan_tasks`
- 返回 `ScanSummary`

- [ ] **Step 4: 运行测试并确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml scanner::tests::scan_codex_state_files_reads_state_databases_and_processes_tasks -- --nocapture`

Expected: 测试通过。

## Task 3: 阶段 6 验证和提交

- [ ] **Step 1: 运行 Rust 测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: 所有 Rust 测试通过。

- [ ] **Step 2: 运行前端构建**

Run: `npm run build`

Expected: TypeScript 和 Vite 构建通过。

- [ ] **Step 3: 检查 Git 状态**

Run: `git status --short`

Expected: 只看到阶段 6 相关源码和文档变更。

- [ ] **Step 4: 提交**

Run:

```bash
git add docs/superpowers/plans/2026-06-14-codex-notice-phase-6.md src-tauri/src
git commit -m "feat: scan Codex state databases"
```

Expected: 创建阶段 6 提交。

## 自检

设计文档要求在阶段 6 的覆盖情况：

- 发现 `~/.codex/state_*.sqlite`：Task 1 覆盖目录级发现。
- 读取 `agent_jobs` 并进入 scanner：Task 2 覆盖。
- 真实后台定时器、读取用户 home 下默认路径、Diagnostics UI：保留到后续阶段。

