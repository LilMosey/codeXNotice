import "./styles.css";

document.querySelector<HTMLDivElement>("#root")!.innerHTML = `
  <main class="app-window">
    <aside class="sidebar">
      <div class="brand">
        <span class="brand-mark">CN</span>
        <div>
          <strong>CodeX Notice</strong>
          <span>任务通知</span>
        </div>
      </div>
      <nav class="nav-list" aria-label="主导航">
        <button class="nav-item active">规则</button>
        <button class="nav-item">渠道</button>
        <button class="nav-item">历史</button>
        <button class="nav-item">诊断</button>
      </nav>
    </aside>

    <section class="workspace">
      <header class="toolbar">
        <div>
          <h1>通知规则</h1>
          <p>按优先级匹配 Codex Desktop 任务，第一条耗时命中的规则生效。</p>
        </div>
        <button class="primary-action">新增规则</button>
      </header>

      <section class="rule-list" aria-label="通知规则列表">
        <article class="rule-row">
          <div class="drag-handle" aria-hidden="true">::</div>
          <div class="rule-main">
            <strong>默认规则</strong>
            <span>所有耗时 · 每时每刻 · macOS 通知</span>
          </div>
          <span class="status enabled">启用</span>
        </article>
        <article class="empty-row">
          后续会在这里配置钉钉、时间窗口、窗口外丢弃或延迟合并。
        </article>
      </section>
    </section>
  </main>
`;
