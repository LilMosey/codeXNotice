# CodeX Notice 阶段 7 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 收窄 MVP 范围，只实现 macOS 本地通知闭环：扫描 Codex 任务、生成 Pending 事件、发送本地通知、更新事件状态。

**Architecture:** 暂缓钉钉和延迟合并。新增 `notifications` 模块，提供可测试的通知 trait 和 macOS `osascript` 发送器。新增事件状态更新能力和通知调度函数，处理 Pending 事件并标记为 `Sent` 或 `Failed`。Tauri 启动时后续可以周期调用扫描和通知派发。

**Tech Stack:** Rust、rusqlite、macOS `osascript`、Rust 单元测试。

---

## 当前 MVP 范围调整

第一版包含：

- 当前机器 Codex Desktop state SQLite 扫描
- 规则匹配
- 默认规则：所有耗时、每时每刻、窗口外丢弃
- macOS 本地通知
- 本地任务和通知历史持久化

第一版暂缓：

- 钉钉
- 延迟合并通知
- 飞书、企业微信、微信
- APNs 和 Apple Watch 独立推送

## 文件结构

阶段 7 创建或修改这些文件：

- `docs/superpowers/specs/2026-06-14-codex-notice-design.md`：同步 MVP 范围。
- `src-tauri/src/lib.rs`：导出 `notifications` 模块。
- `src-tauri/src/storage/events.rs`：新增按状态查询事件、更新事件状态。
- `src-tauri/src/notifications/mod.rs`：通知模块入口。
- `src-tauri/src/notifications/local.rs`：本地通知 sender 和 pending 派发逻辑。

## Task 1: 同步设计文档

- [ ] **Step 1: 修改设计文档**

将第一版范围中的钉钉和延迟合并改为后续阶段，明确 MVP 只做 macOS 本地通知，窗口外默认丢弃。

## Task 2: 用 TDD 添加事件状态查询和更新

- [ ] **Step 1: 写失败测试**

在 `storage::events` 中新增测试：创建 Pending 事件后，能按状态查出；更新为 Sent 后，Pending 查询为空。

- [ ] **Step 2: 实现查询和更新**

新增：

- `list_events_by_status`
- `update_event_status`

## Task 3: 用 TDD 添加本地通知派发

- [ ] **Step 1: 写失败测试**

新增测试 sender，验证 pending 事件会调用 sender，成功后标记 Sent；失败后标记 Failed。

- [ ] **Step 2: 实现通知模块**

新增：

- `LocalNotifier` trait
- `MacOsNotifier`
- `dispatch_pending_notifications`

## Task 4: 阶段 7 验证和提交

- [ ] **Step 1: 运行 Rust 测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

- [ ] **Step 2: 运行前端构建**

Run: `npm run build`

- [ ] **Step 3: 提交**

Run:

```bash
git add docs/superpowers/plans/2026-06-14-codex-notice-phase-7.md docs/superpowers/specs/2026-06-14-codex-notice-design.md src-tauri/src
git commit -m "feat: add local notification dispatch"
```

