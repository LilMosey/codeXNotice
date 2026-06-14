# CodeX Notice

CodeX Notice 是一个 macOS 桌面 App，用来在本机 Codex Desktop 任务完成时发送系统通知。适合 Codex 跑长任务时，人离开电脑但希望任务结束后能收到提醒的场景。

当前版本支持 macOS 本地通知。Apple Watch 是否同步提醒，取决于用户自己的 macOS、iPhone 和 Apple Watch 通知同步设置。

## 当前能力

- 打开 App 后自动后台扫描当前机器上的 Codex Desktop 任务
- 默认每 30 秒扫描一次
- 支持普通 Codex 桌面会话和 `agent_jobs` 任务检测
- 支持 macOS 原生系统通知
- 支持通知规则配置
- 支持规则优先级上移、下移
- 支持通知历史和诊断信息查看

暂未实现：

- 钉钉通知
- 飞书、企业微信、微信
- 非通知时间段后的延迟合并补发
- `.dmg` 安装包
- Apple Watch 直接推送 API

## 使用方法

1. 打开 `CodeX Notice.app`
2. 如果 macOS 弹出通知权限请求，选择允许
3. 保持 CodeX Notice 运行
4. 在 Codex Desktop 中运行一个任务
5. 任务完成后最多等待 30 秒
6. 正常情况下会看到 `CodeX Notice` 的 macOS 通知弹窗

如果想查看是否检测到任务，可以打开 App 的 `历史` 页面并点击 `刷新`。

## 规则配置

进入 App 的 `规则` 页面可以配置通知规则。

规则按从上到下的顺序匹配。第一条耗时条件命中的规则生效，后面的规则不会再参与判断。

每条规则支持：

- 规则名称
- 启用或停用
- 耗时范围
- 通知星期
- 开始时间和结束时间
- 窗口外策略
- 上移、下移优先级

耗时范围填写单位是分钟。

示例：

```text
1-50,60-70,120+
```

含义：

- 1 到 50 分钟
- 60 到 70 分钟
- 120 分钟以上

耗时范围留空时，表示所有耗时都会命中。

如果勾选 `每时每刻都允许通知`，星期和时间不会限制通知。修改星期或时间后，会自动取消 `每时每刻`。

当前版本的窗口外策略默认是 `丢弃`。`延迟` 选项会保留在界面里，但延迟合并通知逻辑后续版本再实现。

## 通知权限

CodeX Notice 使用 App 原生系统通知，不再通过 `osascript` 发送。

如果历史里有事件，但没有看到通知弹窗，请检查：

1. 打开 macOS `系统设置`
2. 进入 `通知`
3. 找到 `CodeX Notice`
4. 打开 `允许通知`
5. 打开横幅或提醒样式
6. 确认当前没有专注模式屏蔽通知

## 给别人安装

当前仓库已经包含 macOS DMG 安装包：

```text
release/CodeX Notice_0.1.0_aarch64.dmg
```

把这个 `.dmg` 文件上传到 GitHub 后，别人下载即可安装。

别人收到后：

1. 双击打开 `CodeX Notice_0.1.0_aarch64.dmg`
2. 在弹出的安装窗口里，把 `CodeX Notice.app` 拖到 `Applications`
3. 打开 `应用程序` 文件夹
4. 双击 `CodeX Notice.app`
5. 如果 macOS 提示无法验证开发者，需要在 `系统设置 -> 隐私与安全性` 中选择仍要打开
6. 首次通知时允许通知权限

当前 App 没有签名和公证，所以别人的 macOS 可能会出现安全提示。这是正常现象。正式发布给更多用户时，建议申请 Apple Developer 账号，对 App 进行签名和公证。

## 从源码打包

开发环境需要安装：

- Node.js
- Rust
- npm

安装依赖：

```bash
npm install
```

运行检查：

```bash
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
```

打包 macOS App：

```bash
npm run tauri build
```

打包完成后产物在：

```text
src-tauri/target/release/bundle/macos/CodeX Notice.app
```

如果需要生成可分发的 DMG，可以使用当前项目采用的朴素 DMG 打包方式：

```bash
mkdir -p /private/tmp/codex-notice-dmg-root
ditto "src-tauri/target/release/bundle/macos/CodeX Notice.app" "/private/tmp/codex-notice-dmg-root/CodeX Notice.app"
ln -sf /Applications /private/tmp/codex-notice-dmg-root/Applications
hdiutil create -volname "CodeX Notice" -srcfolder /private/tmp/codex-notice-dmg-root -ov -format UDZO "release/CodeX Notice_0.1.0_aarch64.dmg"
hdiutil verify "release/CodeX Notice_0.1.0_aarch64.dmg"
```

生成后的 DMG 在：

```text
release/CodeX Notice_0.1.0_aarch64.dmg
```

## 验收建议

1. 退出旧版本 CodeX Notice
2. 打开最新打包出来的 `CodeX Notice.app`
3. 确认通知权限已允许
4. 新建一个 Codex 对话或任务
5. 等 Codex 回复完成
6. 最多等待 30 秒
7. 检查是否出现系统通知
8. 打开 `历史` 页面并点击 `刷新`，确认有对应事件

如果历史有事件但没有弹窗，优先排查 macOS 通知权限。

如果历史没有事件，优先检查：

- CodeX Notice 是否保持运行
- 当前 Codex 是否是桌面版
- App 的 `诊断` 页面是否能看到本地数据库和 Codex 目录

## 数据说明

CodeX Notice 会读取当前用户本机的 Codex 本地状态文件，用于判断任务是否完成。

它只读取 Codex 状态，不写入 Codex 自己的数据文件。CodeX Notice 自己的数据保存在：

```text
~/Library/Application Support/CodeX Notice/codex-notice.sqlite
```

本地数据库保存内容包括：

- 通知规则
- 已检测任务
- 通知事件
- 诊断信息

## 后续计划

- 支持 `.dmg` 安装包
- 支持签名和公证
- 支持钉钉通知
- 支持延迟合并通知
- 支持更清晰的通知内容，展示任务标题而不是内部线程 id
- 支持菜单栏常驻和退出控制
