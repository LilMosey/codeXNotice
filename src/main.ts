import "./styles.css";

type Weekday = "Mon" | "Tue" | "Wed" | "Thu" | "Fri" | "Sat" | "Sun";

type DurationCondition = "Any" | { Ranges: DurationRange[] };
type TimeWindowCondition = "Always" | { Windows: TimeWindow[] };
type OutsideWindowPolicy = "Discard" | "Delay";

interface DurationRange {
  min_seconds: number;
  max_seconds: number | null;
}

interface TimeWindow {
  weekdays: Weekday[];
  start_seconds: number;
  end_seconds: number;
}

interface NotificationRule {
  id: string;
  name: string;
  enabled: boolean;
  duration: DurationCondition;
  time_window: TimeWindowCondition;
  outside_window: OutsideWindowPolicy;
}

interface NotificationEventRecord {
  id: string;
  task_id: string;
  rule_id: string | null;
  status: string;
}

interface AppDiagnostics {
  database_path: string;
  codex_directory: string;
  rule_count: number;
  event_count: number;
}

type TabId = "rules" | "channels" | "history" | "diagnostics";

const weekdayLabels: Array<{ value: Weekday; label: string }> = [
  { value: "Mon", label: "周一" },
  { value: "Tue", label: "周二" },
  { value: "Wed", label: "周三" },
  { value: "Thu", label: "周四" },
  { value: "Fri", label: "周五" },
  { value: "Sat", label: "周六" },
  { value: "Sun", label: "周日" },
];

const state: {
  activeTab: TabId;
  rules: NotificationRule[];
  events: NotificationEventRecord[];
  diagnostics: AppDiagnostics | null;
  selectedRuleId: string | null;
  message: string;
} = {
  activeTab: "rules",
  rules: [],
  events: [],
  diagnostics: null,
  selectedRuleId: null,
  message: "正在加载配置...",
};

const root = document.querySelector<HTMLDivElement>("#root")!;

async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const tauri = (window as unknown as { __TAURI__?: { core?: { invoke?: Function } } }).__TAURI__;
  if (!tauri?.core?.invoke) {
    throw new Error("当前窗口没有连接到 Tauri 后端，请从 CodeX Notice.app 打开。");
  }

  return tauri.core.invoke(command, args) as Promise<T>;
}

async function loadInitialData() {
  try {
    const [rules, events, diagnostics] = await Promise.all([
      invoke<NotificationRule[]>("get_rules"),
      invoke<NotificationEventRecord[]>("get_events"),
      invoke<AppDiagnostics>("get_diagnostics"),
    ]);
    state.rules = rules;
    state.events = events;
    state.diagnostics = diagnostics;
    state.selectedRuleId = rules[0]?.id ?? null;
    state.message = "后台扫描运行中，每 30 秒检查一次 Codex 任务。";
  } catch (error) {
    state.message = String(error);
  }
  render();
}

function render() {
  root.innerHTML = `
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
          ${navButton("rules", "规则")}
          ${navButton("channels", "渠道")}
          ${navButton("history", "历史")}
          ${navButton("diagnostics", "诊断")}
        </nav>
      </aside>

      <section class="workspace">
        ${renderHeader()}
        <p class="message">${escapeHtml(state.message)}</p>
        ${renderActivePanel()}
      </section>
    </main>
  `;

  bindEvents();
}

function navButton(tab: TabId, label: string) {
  return `<button class="nav-item ${state.activeTab === tab ? "active" : ""}" data-tab="${tab}">${label}</button>`;
}

function renderHeader() {
  const titles: Record<TabId, [string, string]> = {
    rules: ["通知规则", "按从上到下的优先级匹配，第一条耗时命中的规则生效。"],
    channels: ["通知渠道", "当前 MVP 使用 macOS 系统通知，后续再接入钉钉和其他渠道。"],
    history: ["通知历史", "查看已经检测到的任务通知事件。"],
    diagnostics: ["诊断", "确认本地数据库、Codex 目录和配置数量。"],
  };
  const [title, subtitle] = titles[state.activeTab];
  const action =
    state.activeTab === "rules"
      ? `<button class="primary-action" data-action="add-rule">新增规则</button>`
      : state.activeTab === "history" || state.activeTab === "diagnostics"
        ? `<button class="secondary-action" data-action="refresh">刷新</button>`
        : "";

  return `
    <header class="toolbar">
      <div>
        <h1>${title}</h1>
        <p>${subtitle}</p>
      </div>
      ${action}
    </header>
  `;
}

function renderActivePanel() {
  if (state.activeTab === "channels") return renderChannels();
  if (state.activeTab === "history") return renderHistory();
  if (state.activeTab === "diagnostics") return renderDiagnostics();
  return renderRules();
}

function renderRules() {
  const selectedRule = state.rules.find((rule) => rule.id === state.selectedRuleId) ?? state.rules[0];
  return `
    <section class="rule-layout">
      <div class="rule-list" aria-label="通知规则列表">
        ${state.rules.map(renderRuleRow).join("")}
      </div>
      <form class="editor" data-role="rule-editor">
        ${selectedRule ? renderRuleEditor(selectedRule) : `<div class="empty-row">暂无规则，请新增一条。</div>`}
      </form>
    </section>
  `;
}

function renderRuleRow(rule: NotificationRule, index: number) {
  return `
    <article class="rule-row ${rule.id === state.selectedRuleId ? "selected" : ""}" data-rule-id="${rule.id}">
      <button class="icon-button" type="button" title="选择规则" data-action="select-rule" data-rule-id="${rule.id}">${index + 1}</button>
      <div class="rule-main">
        <strong>${escapeHtml(rule.name)}</strong>
        <span>${describeDuration(rule.duration)} · ${describeTimeWindow(rule.time_window)}</span>
      </div>
      <span class="status ${rule.enabled ? "enabled" : "disabled"}">${rule.enabled ? "启用" : "停用"}</span>
    </article>
  `;
}

function renderRuleEditor(rule: NotificationRule) {
  const ranges = durationToText(rule.duration);
  const window = firstWindow(rule.time_window);
  const always = rule.time_window === "Always";

  return `
    <div class="editor-grid">
      <label>
        <span>规则名称</span>
        <input name="name" value="${escapeAttribute(rule.name)}" />
      </label>

      <label class="toggle-line">
        <input type="checkbox" name="enabled" ${rule.enabled ? "checked" : ""} />
        <span>启用这条规则</span>
      </label>

      <label>
        <span>耗时范围（分钟）</span>
        <input name="duration" value="${escapeAttribute(ranges)}" placeholder="例如：1-50,60-70,120+" />
      </label>

      <label class="toggle-line">
        <input type="checkbox" name="always" ${always ? "checked" : ""} />
        <span>每时每刻都允许通知</span>
      </label>

      <fieldset ${always ? "disabled" : ""}>
        <legend>可通知星期</legend>
        <div class="weekday-grid">
          ${weekdayLabels
            .map(
              (day) => `
                <label class="checkbox-pill">
                  <input type="checkbox" name="weekday" value="${day.value}" ${window.weekdays.includes(day.value) ? "checked" : ""} />
                  <span>${day.label}</span>
                </label>
              `,
            )
            .join("")}
        </div>
      </fieldset>

      <div class="time-grid">
        <label>
          <span>开始时间</span>
          <input type="time" name="start" value="${secondsToTime(window.start_seconds)}" ${always ? "disabled" : ""} />
        </label>
        <label>
          <span>结束时间</span>
          <input type="time" name="end" value="${secondsToTime(window.end_seconds)}" ${always ? "disabled" : ""} />
        </label>
      </div>

      <label>
        <span>窗口外策略</span>
        <select name="outside">
          <option value="Discard" ${rule.outside_window === "Discard" ? "selected" : ""}>丢弃</option>
          <option value="Delay" ${rule.outside_window === "Delay" ? "selected" : ""}>延迟（后续合并通知实现后生效）</option>
        </select>
      </label>
    </div>

    <footer class="editor-actions">
      <button type="button" class="secondary-action" data-action="move-up">上移</button>
      <button type="button" class="secondary-action" data-action="move-down">下移</button>
      <button type="button" class="danger-action" data-action="delete-rule">删除</button>
      <button type="submit" class="primary-action">保存规则</button>
    </footer>
  `;
}

function renderChannels() {
  return `
    <section class="panel">
      <div class="channel-row">
        <div>
          <strong>macOS 系统通知</strong>
          <span>已启用。首次通知时请在系统弹窗中允许。</span>
        </div>
        <span class="status enabled">当前可用</span>
      </div>
      <div class="channel-row muted">
        <div>
          <strong>钉钉</strong>
          <span>暂不启用，后续版本接入。</span>
        </div>
        <span class="status disabled">未启用</span>
      </div>
    </section>
  `;
}

function renderHistory() {
  return `
    <section class="panel">
      ${state.events.length === 0 ? `<div class="empty-row">暂无通知历史。</div>` : ""}
      ${state.events
        .slice()
        .reverse()
        .map(
          (event) => `
            <div class="history-row">
              <strong>${escapeHtml(event.task_id)}</strong>
              <span>状态：${escapeHtml(event.status)} · 规则：${escapeHtml(event.rule_id ?? "无")}</span>
            </div>
          `,
        )
        .join("")}
    </section>
  `;
}

function renderDiagnostics() {
  const diagnostics = state.diagnostics;
  return `
    <section class="panel diagnostics-grid">
      <div><span>本地数据库</span><strong>${escapeHtml(diagnostics?.database_path ?? "-")}</strong></div>
      <div><span>Codex 目录</span><strong>${escapeHtml(diagnostics?.codex_directory ?? "-")}</strong></div>
      <div><span>规则数量</span><strong>${diagnostics?.rule_count ?? 0}</strong></div>
      <div><span>事件数量</span><strong>${diagnostics?.event_count ?? 0}</strong></div>
    </section>
  `;
}

function bindEvents() {
  root.querySelectorAll<HTMLButtonElement>("[data-tab]").forEach((button) => {
    button.addEventListener("click", () => {
      state.activeTab = button.dataset.tab as TabId;
      render();
    });
  });

  root.querySelector<HTMLButtonElement>("[data-action='add-rule']")?.addEventListener("click", addRule);
  root.querySelector<HTMLButtonElement>("[data-action='refresh']")?.addEventListener("click", refreshCurrentTab);

  root.querySelectorAll<HTMLButtonElement>("[data-action='select-rule']").forEach((button) => {
    button.addEventListener("click", () => {
      state.selectedRuleId = button.dataset.ruleId ?? null;
      render();
    });
  });

  root.querySelector<HTMLFormElement>("[data-role='rule-editor']")?.addEventListener("submit", saveCurrentRule);
  root.querySelector<HTMLButtonElement>("[data-action='delete-rule']")?.addEventListener("click", deleteCurrentRule);
  root.querySelector<HTMLButtonElement>("[data-action='move-up']")?.addEventListener("click", () => moveCurrentRule(-1));
  root.querySelector<HTMLButtonElement>("[data-action='move-down']")?.addEventListener("click", () => moveCurrentRule(1));
}

function addRule() {
  const rule: NotificationRule = {
    id: `rule-${Date.now()}`,
    name: `新规则 ${state.rules.length + 1}`,
    enabled: true,
    duration: "Any",
    time_window: "Always",
    outside_window: "Discard",
  };
  state.rules = [...state.rules, rule].slice(0, 50);
  state.selectedRuleId = rule.id;
  state.message = state.rules.length >= 50 ? "最多只能配置 50 条规则。" : "已新增规则，记得保存。";
  render();
}

async function saveCurrentRule(event: Event) {
  event.preventDefault();
  const selectedId = state.selectedRuleId;
  const form = event.currentTarget as HTMLFormElement;
  if (!selectedId) return;

  try {
    const updatedRule = readRuleFromForm(selectedId, form);
    state.rules = state.rules.map((rule) => (rule.id === selectedId ? updatedRule : rule));
    state.rules = await invoke<NotificationRule[]>("save_rules", { notificationRules: state.rules });
    state.message = "规则已保存，后台扫描下一轮会使用新配置。";
  } catch (error) {
    state.message = String(error);
  }
  render();
}

async function deleteCurrentRule() {
  if (!state.selectedRuleId) return;
  if (state.rules.length === 1) {
    state.message = "至少保留一条规则。";
    render();
    return;
  }
  state.rules = state.rules.filter((rule) => rule.id !== state.selectedRuleId);
  state.selectedRuleId = state.rules[0]?.id ?? null;
  state.rules = await invoke<NotificationRule[]>("save_rules", { notificationRules: state.rules });
  state.message = "规则已删除。";
  render();
}

async function moveCurrentRule(direction: -1 | 1) {
  const index = state.rules.findIndex((rule) => rule.id === state.selectedRuleId);
  const target = index + direction;
  if (index < 0 || target < 0 || target >= state.rules.length) return;
  const nextRules = [...state.rules];
  [nextRules[index], nextRules[target]] = [nextRules[target], nextRules[index]];
  state.rules = await invoke<NotificationRule[]>("save_rules", { notificationRules: nextRules });
  state.message = "规则优先级已更新。";
  render();
}

async function refreshCurrentTab() {
  try {
    if (state.activeTab === "history") {
      state.events = await invoke<NotificationEventRecord[]>("get_events");
      state.message = "历史已刷新。";
    }
    if (state.activeTab === "diagnostics") {
      state.diagnostics = await invoke<AppDiagnostics>("get_diagnostics");
      state.message = "诊断信息已刷新。";
    }
  } catch (error) {
    state.message = String(error);
  }
  render();
}

function readRuleFromForm(id: string, form: HTMLFormElement): NotificationRule {
  const data = new FormData(form);
  const always = data.get("always") === "on";
  const weekdays = data.getAll("weekday") as Weekday[];
  return {
    id,
    name: String(data.get("name") || "未命名规则").trim(),
    enabled: data.get("enabled") === "on",
    duration: parseDuration(String(data.get("duration") || "")),
    time_window: always
      ? "Always"
      : {
          Windows: [
            {
              weekdays: weekdays.length > 0 ? weekdays : weekdayLabels.map((day) => day.value),
              start_seconds: timeToSeconds(String(data.get("start") || "00:00")),
              end_seconds: timeToSeconds(String(data.get("end") || "23:59")),
            },
          ],
        },
    outside_window: String(data.get("outside")) === "Delay" ? "Delay" : "Discard",
  };
}

function parseDuration(value: string): DurationCondition {
  const trimmed = value.trim();
  if (!trimmed || trimmed === "*") return "Any";
  const ranges = trimmed.split(",").map((part) => {
    const item = part.trim();
    if (item.endsWith("+")) {
      return { min_seconds: minutesToSeconds(item.slice(0, -1)), max_seconds: null };
    }
    if (item.includes("-")) {
      const [min, max] = item.split("-");
      return { min_seconds: minutesToSeconds(min), max_seconds: minutesToSeconds(max) };
    }
    const seconds = minutesToSeconds(item);
    return { min_seconds: seconds, max_seconds: seconds };
  });
  return { Ranges: ranges };
}

function minutesToSeconds(value: string) {
  const minutes = Number(value.trim());
  if (!Number.isFinite(minutes) || minutes < 0) throw new Error("耗时范围格式不正确。");
  return Math.round(minutes * 60);
}

function durationToText(duration: DurationCondition) {
  if (duration === "Any") return "";
  return duration.Ranges.map((range) => {
    const min = range.min_seconds / 60;
    if (range.max_seconds === null) return `${min}+`;
    return `${min}-${range.max_seconds / 60}`;
  }).join(",");
}

function describeDuration(duration: DurationCondition) {
  if (duration === "Any") return "所有耗时";
  return durationToText(duration) + " 分钟";
}

function describeTimeWindow(window: TimeWindowCondition) {
  if (window === "Always") return "每时每刻";
  const first = firstWindow(window);
  return `${first.weekdays.length} 天 · ${secondsToTime(first.start_seconds)}-${secondsToTime(first.end_seconds)}`;
}

function firstWindow(window: TimeWindowCondition): TimeWindow {
  if (window === "Always" || window.Windows.length === 0) {
    return {
      weekdays: weekdayLabels.map((day) => day.value),
      start_seconds: 8 * 3600,
      end_seconds: 20 * 3600,
    };
  }
  return window.Windows[0];
}

function timeToSeconds(value: string) {
  const [hours, minutes] = value.split(":").map(Number);
  return hours * 3600 + minutes * 60;
}

function secondsToTime(seconds: number) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  return `${hours.toString().padStart(2, "0")}:${minutes.toString().padStart(2, "0")}`;
}

function escapeHtml(value: string) {
  return value.replace(/[&<>"']/g, (char) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#039;" })[char]!);
}

function escapeAttribute(value: string) {
  return escapeHtml(value);
}

render();
void loadInitialData();
