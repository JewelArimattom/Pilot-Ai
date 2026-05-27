import "./style.css";
import type { ActionPlan, ActionStep, PlanStatus } from "./types/action";

type TauriInvoke = <T>(cmd: string, payload?: Record<string, unknown>) => Promise<T>;
type TauriEvent = {
  listen: (event: string, handler: (e: any) => void) => Promise<() => void>;
  emit: (event: string, payload?: unknown) => Promise<void>;
};
type TauriGlobals = { __TAURI__?: { invoke?: TauriInvoke, event?: TauriEvent }; __TAURI_INTERNALS__?: { invoke?: TauriInvoke, event?: TauriEvent } };
type AppConfig = { selectedAppId?: string | null; selectedAppPath?: string | null };
type PlannerRequest = { request: string; ghostMode: boolean; teachMode: boolean; fastMode: boolean; screenImage?: string | null };

const $ = <T extends HTMLElement>(s: string) => document.querySelector<T>(s);
const app = $<HTMLDivElement>("#app")!;

function getTauri(): { invoke: TauriInvoke; event: TauriEvent } | null {
  const g = window as Window & TauriGlobals;
  if (g.__TAURI__) return g.__TAURI__ as any;
  if (g.__TAURI_INTERNALS__) return g.__TAURI_INTERNALS__ as any;
  return null;
}

if (location.search.includes("overlay=1")) {
  initOverlayApp();
} else {
  initMainApp();
}

function initOverlayApp() {
  document.body.style.background = "transparent";
  document.body.style.overflow = "hidden";

  // Inject animation keyframes
  const ks = document.createElement("style");
  ks.textContent = `
    @keyframes cursor-glow-pulse { 0%,100%{opacity:.5;transform:scale(1)} 50%{opacity:1;transform:scale(1.25)} }
    @keyframes ripple-expand { 0%{transform:scale(.3);opacity:.9} 100%{transform:scale(2.8);opacity:0} }
    @keyframes teach-bounce { 0%,60%,100%{transform:translateY(0)} 30%{transform:translateY(-5px)} }
    @keyframes teach-slide-in { from{opacity:0;transform:translateY(-50%) translateX(120%)} to{opacity:1;transform:translateY(-50%) translateX(0)} }
    @keyframes teach-slide-out { from{opacity:1;transform:translateY(-50%) translateX(0)} to{opacity:0;transform:translateY(-50%) translateX(120%)} }
    @keyframes teach-bar { from{width:var(--from-w,0%)} to{width:var(--to-w,100%)} }
  `;
  document.head.appendChild(ks);

  app.innerHTML = `
    <!-- Ripple container -->
    <div id="ripple-layer" style="position:fixed;inset:0;pointer-events:none;z-index:999990;"></div>

    <!-- Premium AI cursor -->
    <div id="overlay-cursor" style="
      position:fixed; left:-120px; top:-120px;
      pointer-events:none; z-index:999999;
      transition:left .45s cubic-bezier(.34,1.56,.64,1), top .45s cubic-bezier(.34,1.56,.64,1);
    ">
      <!-- Glow halo -->
      <div style="
        position:absolute; inset:-10px; border-radius:50%;
        background:radial-gradient(circle,rgba(110,86,207,.55) 0%,transparent 70%);
        animation:cursor-glow-pulse 1.8s ease infinite;
      "></div>
      <!-- Cursor SVG -->
      <svg width="30" height="38" viewBox="0 0 30 38" fill="none">
        <defs>
          <linearGradient id="cg" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0%" stop-color="#ffffff"/>
            <stop offset="100%" stop-color="#d8cfff"/>
          </linearGradient>
          <filter id="cs"><feDropShadow dx="0" dy="2" stdDeviation="2.5" flood-color="rgba(0,0,0,.55)"/></filter>
        </defs>
        <path d="M4 2L26 14L18 17L14 26L4 2Z" fill="url(#cg)" stroke="#7c63d9" stroke-width="1.4" stroke-linejoin="round" filter="url(#cs)"/>
        <circle cx="14" cy="26" r="2" fill="#a78bfa" opacity=".7"/>
      </svg>
      <!-- Tooltip label -->
      <div id="overlay-msg" style="
        position:absolute; top:40px; left:6px;
        background:rgba(8,8,20,.92); color:#fff;
        padding:6px 13px; border-radius:9px; font-size:12px;
        white-space:nowrap; opacity:0;
        transition:opacity .25s;
        border:1px solid rgba(110,86,207,.4);
        backdrop-filter:blur(12px);
        font-family:'Segoe UI',system-ui,sans-serif;
        box-shadow:0 4px 20px rgba(0,0,0,.5);
      "></div>
    </div>

    <!-- Right-side floating teaching panel -->
    <div id="teach-panel" style="
      position:fixed; right:20px; top:50%;
      transform:translateY(-50%) translateX(120%);
      width:310px; pointer-events:none; z-index:999995;
      background:rgba(7,7,18,.94);
      border:1px solid rgba(110,86,207,.5);
      border-radius:18px; padding:20px 22px;
      box-shadow:0 24px 64px rgba(0,0,0,.85),0 0 0 1px rgba(110,86,207,.15),inset 0 1px 0 rgba(255,255,255,.06);
      backdrop-filter:blur(24px);
      font-family:'Segoe UI',system-ui,sans-serif;
      transition:transform .45s cubic-bezier(.34,1.56,.64,1),opacity .35s ease;
      opacity:0;
    ">
      <!-- Header -->
      <div style="display:flex;align-items:center;gap:10px;margin-bottom:14px;">
        <div style="width:30px;height:30px;border-radius:9px;background:linear-gradient(135deg,#6e56cf,#a78bfa);display:flex;align-items:center;justify-content:center;font-size:15px;flex-shrink:0;">🎓</div>
        <div style="flex:1;min-width:0;">
          <div style="font-size:10px;color:rgba(255,255,255,.35);font-weight:600;letter-spacing:.6px;text-transform:uppercase;">Pilot AI · Teaching</div>
          <div id="tp-counter" style="font-size:12px;color:#a78bfa;font-weight:700;margin-top:1px;">Step 1 of 1</div>
        </div>
        <div style="background:rgba(110,86,207,.2);border:1px solid rgba(110,86,207,.45);border-radius:20px;padding:2px 9px;font-size:9px;color:#a78bfa;font-weight:800;letter-spacing:.5px;">LIVE</div>
      </div>
      <!-- Progress bar -->
      <div style="height:3px;background:rgba(255,255,255,.07);border-radius:2px;margin-bottom:16px;overflow:hidden;">
        <div id="tp-bar" style="height:100%;background:linear-gradient(90deg,#6e56cf,#a78bfa);border-radius:2px;width:0%;transition:width .6s ease;"></div>
      </div>
      <!-- Step title -->
      <div id="tp-title" style="font-size:15px;font-weight:700;color:#fff;margin-bottom:6px;line-height:1.35;"></div>
      <!-- Step description -->
      <div id="tp-desc" style="font-size:12px;color:rgba(255,255,255,.5);line-height:1.65;min-height:36px;"></div>
      <!-- Footer pulse dots -->
      <div style="display:flex;align-items:center;gap:8px;margin-top:16px;padding-top:13px;border-top:1px solid rgba(255,255,255,.06);">
        <div style="display:flex;gap:4px;">
          <div style="width:5px;height:5px;border-radius:50%;background:#6e56cf;animation:teach-bounce 1.2s ease infinite;"></div>
          <div style="width:5px;height:5px;border-radius:50%;background:#6e56cf;animation:teach-bounce 1.2s ease .15s infinite;"></div>
          <div style="width:5px;height:5px;border-radius:50%;background:#6e56cf;animation:teach-bounce 1.2s ease .3s infinite;"></div>
        </div>
        <span id="tp-status" style="font-size:11px;color:rgba(255,255,255,.28);">Watching...</span>
      </div>
    </div>
  `;

  const tauri = getTauri();
  if (!tauri) return;

  const cursor   = document.getElementById("overlay-cursor")!;
  const msgLabel = document.getElementById("overlay-msg")!;
  const rippleLayer = document.getElementById("ripple-layer")!;
  const teachPanel  = document.getElementById("teach-panel")!;
  const tpCounter   = document.getElementById("tp-counter")!;
  const tpBar       = document.getElementById("tp-bar")!;
  const tpTitle     = document.getElementById("tp-title")!;
  const tpDesc      = document.getElementById("tp-desc")!;
  const tpStatus    = document.getElementById("tp-status")!;

  tauri.event.listen("overlay-move", (e: any) => {
    const { x, y } = e.payload;
    cursor.style.left = x + "px";
    cursor.style.top  = y + "px";
  });

  tauri.event.listen("overlay-click", (e: any) => {
    // Scale cursor
    const svg = cursor.querySelector("svg") as SVGElement;
    if (svg) { svg.style.transform = "scale(.75)"; setTimeout(() => { svg.style.transform = "scale(1)"; }, 160); }
    // Ripple at cursor position
    const cx = parseInt(cursor.style.left, 10);
    const cy = parseInt(cursor.style.top,  10);
    const r  = document.createElement("div");
    r.style.cssText = `
      position:absolute;left:${cx-14}px;top:${cy-14}px;
      width:28px;height:28px;border-radius:50%;
      border:2px solid rgba(110,86,207,.85);
      animation:ripple-expand 600ms ease forwards;
    `;
    rippleLayer.appendChild(r);
    setTimeout(() => r.remove(), 650);
  });

  let msgTimer: ReturnType<typeof setTimeout>;
  tauri.event.listen("overlay-say", (e: any) => {
    const { text } = e.payload;
    msgLabel.textContent = text;
    msgLabel.style.opacity = "1";
    clearTimeout(msgTimer);
    msgTimer = setTimeout(() => { msgLabel.style.opacity = "0"; }, 4000);
  });

  // Teaching panel events
  tauri.event.listen("overlay-teach-step", (e: any) => {
    const { step, total, title, description, status, done } = e.payload as {
      step?: number; total?: number; title?: string;
      description?: string; status?: string; done?: boolean;
    };

    if (done) {
      teachPanel.style.opacity = "0";
      teachPanel.style.transform = "translateY(-50%) translateX(120%)";
      return;
    }

    // Show panel
    teachPanel.style.opacity = "1";
    teachPanel.style.transform = "translateY(-50%) translateX(0)";

    if (step !== undefined && total !== undefined) {
      tpCounter.textContent = `Step ${step} of ${total}`;
      tpBar.style.width = `${(step / total) * 100}%`;
    }
    if (title)       tpTitle.textContent = title;
    if (description) tpDesc.textContent  = description;
    if (status)      tpStatus.textContent = status;
  });
}

function initMainApp() {
document.body.classList.add("is-loading");

// ── Build UI ──
app.innerHTML = `
<div class="loading-screen" id="loading-screen">
  <div class="loading-card">
    <div class="loading-spinner"></div>
    <div>
      <div class="loading-title">Pilot AI</div>
      <div class="loading-subtitle">Initializing systems...</div>
    </div>
  </div>
</div>

<div class="main-layout">
  <!-- Sidebar -->
  <div class="sidebar">
    <div class="sidebar-logo">P</div>
    <button class="sidebar-btn active" id="btn-home" title="Home">⌂</button>
    <button class="sidebar-btn" id="btn-settings" title="Settings">⚙</button>
    <div class="sidebar-spacer"></div>
    <button class="sidebar-btn" id="btn-chat-toggle" title="Toggle Chat">💬</button>
  </div>

  <!-- Center -->
  <div class="center-content">
    <div class="topbar">
      <div class="topbar-left">
        <span class="topbar-dot"></span>
        <span class="topbar-title">Pilot AI Console</span>
      </div>
      <span class="topbar-tag">● Online</span>
    </div>

    <div class="workspace" id="workspace">
      <!-- Enable Toggle -->
      <div class="enable-bar">
        <div class="enable-bar-left">
          <div class="enable-bar-icon">🤖</div>
          <div>
            <h2>Pilot AI Assistant</h2>
            <p>Enable to open the floating assistant panel</p>
          </div>
        </div>
        <label class="switch" id="enable-switch">
          <input type="checkbox" id="tool-enable" />
          <div class="switch-track"></div>
          <div class="switch-thumb"></div>
        </label>
      </div>

      <!-- Settings -->
      <div class="settings-panel" id="settings-panel">
        <div class="settings-header">
          <h2>Configuration</h2>
          <span class="pill pill-muted" id="status-pill">Idle</span>
        </div>

        <div class="toggle-grid">
          <label class="toggle"><input id="ghost-mode" type="checkbox" checked /><span>Ghost mode</span></label>
          <label class="toggle"><input id="teach-mode" type="checkbox" checked /><span>Teach mode</span></label>
          <label class="toggle"><input id="vision-mode" type="checkbox" /><span>Vision mode</span></label>
          <label class="toggle"><input id="fast-mode" type="checkbox" /><span>Fast mode</span></label>
        </div>
      </div>
    </div>

    <div class="status-bar">
      <div class="status-bar-item"><span class="status-bar-dot" id="sb-dot"></span><span id="sb-text">Ready</span></div>
    </div>
  </div>

  <!-- Floating Chat Panel -->
  <div class="chat-panel" id="chat-panel">
    <div class="chat-header">
      <div class="chat-header-left">
        <div class="chat-header-icon">✦</div>
        <h3>Pilot AI</h3>
      </div>
      <button class="chat-close" id="chat-close">✕</button>
    </div>
    <div class="chat-messages" id="chat-messages">
      <div class="chat-empty">
        <div class="chat-empty-icon">✦</div>
        <h4>Pilot AI Assistant</h4>
        <p>Describe a task and I'll plan and execute it on your desktop</p>
      </div>
    </div>
    <div class="chat-input-area">
      <form id="command-form" class="chat-input-wrapper">
        <input id="command-input" class="chat-input" placeholder="Describe a task..." autocomplete="off" />
        <button type="submit" class="chat-send" id="chat-send-btn">→</button>
      </form>
    </div>
  </div>
</div>
`;

// ── Elements ──
const form = $<HTMLFormElement>("#command-form")!;
const input = $<HTMLInputElement>("#command-input")!;
const ghostToggle = $<HTMLInputElement>("#ghost-mode")!;
const teachToggle = $<HTMLInputElement>("#teach-mode")!;
const visionToggle = $<HTMLInputElement>("#vision-mode")!;
const fastToggle = $<HTMLInputElement>("#fast-mode")!;
const chatPanel = $<HTMLDivElement>("#chat-panel")!;
const chatMessages = $<HTMLDivElement>("#chat-messages")!;
const toolEnable = $<HTMLInputElement>("#tool-enable")!;
const chatCloseBtn = $<HTMLButtonElement>("#chat-close")!;
const chatToggleBtn = $<HTMLButtonElement>("#btn-chat-toggle")!;
const statusPill = $<HTMLSpanElement>("#status-pill")!;
const sbDot = $<HTMLSpanElement>("#sb-dot")!;
const sbText = $<HTMLSpanElement>("#sb-text")!;
const executeBtn = $<HTMLButtonElement>("#chat-send-btn")!;

// ── State ──
let currentPlan: ActionPlan | null = null;
let isPlanning = false;
let chatEnabled = false;

// ── Loading ──
function hideLoading() {
  const ls = $<HTMLDivElement>("#loading-screen");
  if (ls) { ls.classList.add("hide"); setTimeout(() => ls.remove(), 350); }
  document.body.classList.remove("is-loading");
}
setTimeout(hideLoading, 500);

// ── Chat Panel Toggle ──
function openChat() {
  chatPanel.classList.add("open");
  chatEnabled = true;
  chatToggleBtn.classList.add("active");
}
function closeChat() {
  chatPanel.classList.remove("open");
  chatEnabled = false;
  chatToggleBtn.classList.remove("active");
}

toolEnable.addEventListener("change", () => {
  if (toolEnable.checked) {
    openChat();
    getTauriInvoke()?.("set_overlay_mode", { enabled: true });
  } else {
    closeChat();
    getTauriInvoke()?.("set_overlay_mode", { enabled: false });
  }
});
chatCloseBtn.addEventListener("click", () => {
  toolEnable.checked = false;
  closeChat();
  getTauriInvoke()?.("set_overlay_mode", { enabled: false });
});
chatToggleBtn.addEventListener("click", () => {
  toolEnable.checked = !toolEnable.checked;
  if (toolEnable.checked) {
    openChat();
    getTauriInvoke()?.("set_overlay_mode", { enabled: true });
  } else {
    closeChat();
    getTauriInvoke()?.("set_overlay_mode", { enabled: false });
  }
});

// ── Chat Messages ──
function clearEmptyState() {
  const empty = chatMessages.querySelector(".chat-empty");
  if (empty) empty.remove();
}

function addUserMsg(text: string) {
  clearEmptyState();
  const div = document.createElement("div");
  div.className = "msg msg-user";
  div.innerHTML = `<span class="msg-label">You</span><div class="msg-bubble">${esc(text)}</div>`;
  chatMessages.appendChild(div);
  chatMessages.scrollTop = chatMessages.scrollHeight;
}

function addAiMsg(text: string): HTMLDivElement {
  clearEmptyState();
  const div = document.createElement("div");
  div.className = "msg msg-ai";
  div.innerHTML = `<span class="msg-label">Pilot AI</span><div class="msg-bubble">${text}</div>`;
  chatMessages.appendChild(div);
  chatMessages.scrollTop = chatMessages.scrollHeight;
  return div;
}

function esc(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

// ── Thinking Visualization ──
function addThinkingBlock(): { el: HTMLDivElement; addStep: (t: string) => HTMLDivElement; complete: () => void } {
  clearEmptyState();
  const block = document.createElement("div");
  block.className = "thinking-block";
  block.innerHTML = `
    <div class="thinking-header">
      <div class="thinking-dots"><span></span><span></span><span></span></div>
      <span>Thinking...</span>
    </div>
    <div class="thinking-steps"></div>
  `;
  chatMessages.appendChild(block);
  chatMessages.scrollTop = chatMessages.scrollHeight;

  const stepsContainer = block.querySelector(".thinking-steps")!;
  const steps: HTMLDivElement[] = [];

  return {
    el: block,
    addStep(text: string) {
      // Mark previous as done
      steps.forEach(s => { s.classList.remove("active"); s.classList.add("done"); s.querySelector(".thinking-step-icon")!.textContent = "✓"; });
      const step = document.createElement("div");
      step.className = "thinking-step active";
      step.innerHTML = `<span class="thinking-step-icon">◌</span><span>${text}</span>`;
      stepsContainer.appendChild(step);
      steps.push(step);
      chatMessages.scrollTop = chatMessages.scrollHeight;
      return step;
    },
    complete() {
      steps.forEach(s => { s.classList.remove("active"); s.classList.add("done"); s.querySelector(".thinking-step-icon")!.textContent = "✓"; });
      const header = block.querySelector(".thinking-header")!;
      header.innerHTML = `<span style="color:var(--green)">✓</span><span>Done thinking</span>`;
    }
  };
}

// ── Mouse Cursor Visualization ──
function addMouseViz(plan: ActionPlan): void {
  clearEmptyState();
  const container = document.createElement("div");
  container.className = "mouse-viz";

  const targets = plan.steps.slice(0, 4).map((s, i) => {
    const act = s.action.action.split(".").pop() || "action";
    const x = 30 + (i * 70) % 280;
    const y = 25 + (i % 2) * 50;
    return { act, x, y };
  });

  const targetDivs = targets.map(t =>
    `<div class="mouse-target" style="left:${t.x}px;top:${t.y}px">${t.act}</div>`
  ).join("");

  container.innerHTML = `
    <div class="mouse-viz-header">🖱 Mouse Automation Preview</div>
    <div class="mouse-canvas" id="mouse-canvas">
      ${targetDivs}
      <div class="mouse-cursor" id="mouse-cursor" style="left:10px;top:10px">
        <svg viewBox="0 0 24 24" fill="white" stroke="black" stroke-width="1">
          <path d="M5 3l14 8-6 2-3 6z"/>
        </svg>
      </div>
      <div class="mouse-click-ring" id="mouse-ring"></div>
      <div class="mouse-action-label" id="mouse-label"></div>
    </div>
  `;
  chatMessages.appendChild(container);
  chatMessages.scrollTop = chatMessages.scrollHeight;

  // Animate cursor through targets
  const cursor = container.querySelector<HTMLDivElement>("#mouse-cursor")!;
  const ring = container.querySelector<HTMLDivElement>("#mouse-ring")!;
  const label = container.querySelector<HTMLDivElement>("#mouse-label")!;

  let idx = 0;
  function animateNext() {
    if (idx >= targets.length) return;
    const t = targets[idx];
    cursor.style.left = t.x + "px";
    cursor.style.top = t.y + "px";
    label.textContent = t.act;

    setTimeout(() => {
      ring.style.left = (t.x + 8) + "px";
      ring.style.top = (t.y + 10) + "px";
      ring.classList.remove("animate");
      void ring.offsetWidth;
      ring.classList.add("animate");
      idx++;
      setTimeout(animateNext, 800);
    }, 650);
  }
  setTimeout(animateNext, 400);
}

// ── Execution Steps in Chat ──
function addExecSteps(plan: ActionPlan): { setRunning: (i: number) => void; setDone: (i: number) => void; setError: (i: number) => void } {
  const wrapper = document.createElement("div");
  wrapper.style.cssText = "display:flex;flex-direction:column;gap:6px;width:100%";

  const stepEls: HTMLDivElement[] = [];
  plan.steps.forEach((step, i) => {
    const el = document.createElement("div");
    el.className = "exec-step pending";
    el.innerHTML = `<div class="exec-step-icon">○</div><span class="exec-step-text">${esc(step.action.action)}</span>`;
    wrapper.appendChild(el);
    stepEls.push(el);
  });

  chatMessages.appendChild(wrapper);
  chatMessages.scrollTop = chatMessages.scrollHeight;

  return {
    setRunning(i: number) {
      if (!stepEls[i]) return;
      stepEls[i].className = "exec-step running";
      stepEls[i].querySelector(".exec-step-icon")!.textContent = "◌";
      chatMessages.scrollTop = chatMessages.scrollHeight;
    },
    setDone(i: number) {
      if (!stepEls[i]) return;
      stepEls[i].className = "exec-step done";
      stepEls[i].querySelector(".exec-step-icon")!.textContent = "✓";
    },
    setError(i: number) {
      if (!stepEls[i]) return;
      stepEls[i].className = "exec-step error";
      stepEls[i].querySelector(".exec-step-icon")!.textContent = "✕";
    }
  };
}

// ── Status Helpers ──
function setStatus(text: string, tone: "idle" | "working" | "error" | "ok") {
  sbText.textContent = text;
  sbDot.className = "status-bar-dot" + (tone === "working" ? " working" : tone === "error" ? " error" : "");
  statusPill.textContent = tone === "working" ? "Working" : tone === "ok" ? "Ready" : tone === "error" ? "Error" : "Idle";
  statusPill.className = "pill" + (tone === "idle" ? " pill-muted" : "");
}



// ── Tauri ──
function getTauriInvoke(): TauriInvoke | null {
  return getTauri()?.invoke ?? null;
}

async function captureScreen(invoke: TauriInvoke): Promise<string | null> {
  try { return await invoke<string>("capture_screen"); } catch { return null; }
}

function emptyPlan(status: PlanStatus, note: string): ActionPlan {
  return { planId: `plan-${Date.now()}`, createdAt: new Date().toISOString(), goal: null, status, steps: [], note };
}

async function planRequest(req: PlannerRequest): Promise<ActionPlan> {
  const invoke = getTauriInvoke();
  if (!invoke) return emptyPlan("needsUser", "AI planner unavailable. Use the desktop app.");
  return invoke<ActionPlan>("plan_actions", {
    request: req.request, ghostMode: req.ghostMode, teachMode: req.teachMode,
    fastMode: req.fastMode, screenImage: req.screenImage ?? null
  });
}

type StepResult = { stepId: string, status: string, message?: string | null };
type ExecutionReport = { planId: string, summary: string, blocked: boolean, results: StepResult[] };

async function executePlan(plan: ActionPlan, ghostMode: boolean): Promise<ExecutionReport> {
  const invoke = getTauriInvoke();
  if (!invoke) return { planId: plan.planId, summary: "Execution requires the Tauri backend.", blocked: true, results: [] };
  
  return await invoke<ExecutionReport>("execute_actions", {
    plan, policy: { allowedApps: ["desktop"], allowedActions: plan.steps.map(s => s.action.action), requireConfirm: false, ghostMode }
  });
}

// ── Plan Cache ──
// Bump CACHE_VERSION whenever the knowledge base or AI prompt changes
// so that old localStorage entries are automatically discarded.
const CACHE_VERSION   = 8;
const PLAN_CACHE_TTL  = 3_600_000; // 1 hour
const planCache       = new Map<string, { plan: ActionPlan; storedAt: number }>();
const MEMORY_KEY      = `pilot-ai:plan-memory-v${CACHE_VERSION}`;

function loadPlanMemory() {
  try {
    // Remove any old-version keys to avoid stale data piling up
    for (let i = 0; i < CACHE_VERSION; i++) {
      localStorage.removeItem(`pilot-ai:plan-memory-v${i}`);
      if (i === 0) localStorage.removeItem("pilot-ai:plan-memory"); // original key
    }
    const raw = localStorage.getItem(MEMORY_KEY);
    if (!raw) return;
    const entries = JSON.parse(raw) as Array<{ request: string; plan: ActionPlan; storedAt: number }>;
    if (Array.isArray(entries)) {
      entries.forEach(e => { if (e.request && e.plan) planCache.set(e.request, { plan: e.plan, storedAt: e.storedAt ?? Date.now() }); });
    }
  } catch {}
}

function persistPlanMemory() {
  try {
    const entries = Array.from(planCache.entries())
      .map(([r, v]) => ({ request: r, plan: v.plan, storedAt: v.storedAt }))
      .sort((a, b) => b.storedAt - a.storedAt)
      .slice(0, 50);
    localStorage.setItem(MEMORY_KEY, JSON.stringify(entries));
  } catch {}
}

loadPlanMemory();

// chatHistory holds the ongoing conversation turns so the AI has full context
// across multiple messages. It is reset only when a task fully completes.
let chatHistory: string[] = [];

// ── Main Form Handler ──
form.addEventListener("submit", async (e) => {
  e.preventDefault();
  const request = input.value.trim();
  if (!request || isPlanning) return;

  addUserMsg(request);
  input.value = "";
  isPlanning = true;
  setStatus("Planning...", "working");

  // Build full request including conversation history for AI context
  const fullRequest = chatHistory.length > 0
    ? chatHistory.join("\n") + "\nUser: " + request
    : "User: " + request;

  // ── Cache check: ONLY use cache when there is NO active conversation.
  //    If chatHistory has content, the user is in a dialogue (e.g. answering
  //    a clarifying question) — we MUST ask the AI fresh so it can reason
  //    about the conversation context.
  const isInDialogue = chatHistory.length > 0;
  if (!isInDialogue) {
    const cached = planCache.get(request.trim().toLowerCase());
    if (cached && Date.now() - cached.storedAt < PLAN_CACHE_TTL) {
      currentPlan = cached.plan;
      // Cached plans are only used if they have real steps — never re-use
      // a cached clarifying-question response.
      if (currentPlan.steps.length > 0 && currentPlan.status === "ready") {
        addAiMsg(`Found cached plan with ${currentPlan.steps.length} steps. Executing...`);
        const execResult = await runExecution(currentPlan);
        chatHistory = []; // Task completed — reset
        chatHistory.push(`Pilot AI executed a cached plan. Log:\n${execResult}`);
        isPlanning = false;
        return;
      }
      // If cached plan had no steps (was a clarifying question), discard it
      // and fall through to ask the AI fresh.
      planCache.delete(request.trim().toLowerCase());
    }
  }

  // Thinking visualization
  const thinking = addThinkingBlock();
  thinking.addStep("Analyzing your request...");
  await sleep(400);

  let screenImage: string | null = null;
  const invoke = getTauriInvoke();
  if (visionToggle.checked && invoke) {
    thinking.addStep("Capturing screen...");
    screenImage = await captureScreen(invoke);
    await sleep(300);
  }

  thinking.addStep("Generating action plan with AI...");

  try {
    currentPlan = await planRequest({
      request: fullRequest,
      ghostMode: ghostToggle.checked,
      teachMode: teachToggle.checked,
      fastMode: fastToggle.checked,
      screenImage
    });

    // ── AI returned a clarifying question (no steps) ──
    // This is the "human-like" flow: AI asks the user something before acting.
    if (currentPlan.steps.length === 0 || currentPlan.status !== "ready") {
      thinking.complete();
      const msg = currentPlan.note ?? "Could not generate a plan. Please clarify your request.";
      addAiMsg(msg);
      // Keep the conversation going — add both turns to history so the
      // next user reply arrives with full context.
      chatHistory.push("User: " + request);
      chatHistory.push("Pilot AI: " + msg);
      setStatus("Waiting for your reply...", "idle");
      isPlanning = false;
      return;
    }

    // ── AI has a concrete plan — start the agentic feedback loop ──
    thinking.addStep(`Plan ready — starting agentic execution`);
    await sleep(300);
    thinking.complete();

    // Show goal
    const goalText = currentPlan.goal ? `**Goal:** ${currentPlan.goal}<br/>` : "";
    const noteText = currentPlan.note ? `<br/><br/><strong>Note:</strong> ${currentPlan.note}` : "";
    addAiMsg(`${goalText}Starting step-by-step agentic execution. AI will adapt after each step.${noteText}`);

    if (ghostToggle.checked) addMouseViz(currentPlan);

    // ── Agentic loop: execute one step at a time, feed result back to AI ──
    const execResult = await runAgenticLoop(fullRequest);

    // Task done — reset history, keep log for follow-up context
    chatHistory = [];
    chatHistory.push(`Pilot AI completed task agentically. Execution Log:\n${execResult}`);

  } catch (err) {
    thinking.complete();
    addAiMsg("Planning failed. Check the console for errors.");
    setStatus("Error", "error");
    console.error(err);
  }

  isPlanning = false;
});

// ── Agentic Deciding Indicator ──
function addAgentDeciding(): HTMLDivElement {
  clearEmptyState();
  const el = document.createElement("div");
  el.className = "msg msg-ai agent-deciding-msg";
  el.innerHTML = `
    <span class="msg-label">Pilot AI</span>
    <div class="msg-bubble agent-deciding-bubble">
      <div class="thinking-dots" style="display:inline-flex;gap:4px;margin-right:8px">
        <span></span><span></span><span></span>
      </div>
      <span style="font-size:12px;color:rgba(255,255,255,.55)">Analyzing result, deciding next action...</span>
    </div>
  `;
  chatMessages.appendChild(el);
  chatMessages.scrollTop = chatMessages.scrollHeight;
  return el;
}

// ── True Agentic Loop ──
// Calls AI → executes ONE step → captures screen → sends result+screenshot back to AI → repeats.
async function runAgenticLoop(context: string): Promise<string> {
  const tauri     = getTauri();
  const invoke    = getTauriInvoke();
  const isTeach   = teachToggle.checked;
  const isGhost   = ghostToggle.checked;
  const MAX_STEPS = 30;
  let agentCtx    = context;   // grows with each step result
  let fullLog     = "";
  let stepNumber  = 0;
  let lastScreen: string | null = null; // screenshot fed to AI each iteration

  setStatus("Agentic execution...", "working");

  while (stepNumber < MAX_STEPS) {
    // Show "deciding..." bubble between steps (not the first call)
    let decidingEl: HTMLDivElement | null = null;
    if (stepNumber > 0) {
      decidingEl = addAgentDeciding();
      await sleep(200);
    }

    // ── Call AI with current context + latest screenshot ──
    let nextPlan: ActionPlan;
    try {
      nextPlan = await planRequest({
        request: agentCtx,
        ghostMode: isGhost,
        teachMode: isTeach,
        fastMode: fastToggle.checked,
        screenImage: lastScreen     // AI sees current screen state
      });
    } catch (err) {
      decidingEl?.remove();
      addAiMsg("AI call failed during agentic loop.");
      setStatus("Error", "error");
      console.error(err);
      break;
    }
    decidingEl?.remove();
    lastScreen = null; // reset; will capture fresh after execution

    // ── AI says task is done or needs user input ──
    if (nextPlan.steps.length === 0 || nextPlan.status !== "ready") {
      const msg = nextPlan.note ?? "✓ Task completed successfully!";
      addAiMsg(msg);
      setStatus("Complete", "ok");
      if (tauri) await tauri.event.emit("overlay-teach-step", { done: true });
      break;
    }

    // ── Execute only the FIRST step ──
    const step = nextPlan.steps[0];
    stepNumber++;

    // Show step in UI
    const stepEl = document.createElement("div");
    stepEl.className = "exec-step pending";
    stepEl.innerHTML = `<div class="exec-step-icon">○</div><span class="exec-step-text">[${stepNumber}] ${esc(step.action.action)}</span>`;
    chatMessages.appendChild(stepEl);
    chatMessages.scrollTop = chatMessages.scrollHeight;

    const setRunning = () => { stepEl.className = "exec-step running"; stepEl.querySelector(".exec-step-icon")!.textContent = "◌"; };
    const setDone    = () => { stepEl.className = "exec-step done";    stepEl.querySelector(".exec-step-icon")!.textContent = "✓"; };
    const setError   = () => { stepEl.className = "exec-step error";   stepEl.querySelector(".exec-step-icon")!.textContent = "✕"; };
    setRunning();

    // Teach overlay panel
    if (isTeach && tauri) {
      const title = step.action.action
        .replace(/^(desktop|photoshop)\./, "")
        .replace(/_/g, " ")
        .replace(/\b\w/g, c => c.toUpperCase());
      await tauri.event.emit("overlay-teach-step", {
        step: stepNumber, total: stepNumber + nextPlan.steps.length,
        title,
        description: step.explanation || step.expectedResult || step.action.action,
        status: "Executing...",
        done: false
      });
      await sleep(1800);
    }

    // ── Execute the step ──
    const miniPlan: ActionPlan = { ...nextPlan, steps: [step] };
    try {
      const report = await executePlan(miniPlan, isGhost);
      const result = report.results[0];

      // After hotkey or open_app, wait for the OS to react (window opens, etc.)
      const actionName = step.action.action;
      if (actionName === "desktop.hotkey" || actionName === "desktop.open_app") {
        await sleep(1500); // let OS open the window before screenshotting
      }

      // ── Capture screenshot so AI sees the CURRENT screen state ──
      if (invoke) {
        try { lastScreen = await captureScreen(invoke); } catch { lastScreen = null; }
      }

      if (result?.status === "failed") {
        setError();
        const errMsg = result.message ?? "Unknown error";
        const entry  = `[Step ${stepNumber}] ${actionName} → FAILED: ${errMsg}`;
        fullLog  += entry + "\n";
        agentCtx += `\n${entry}. A screenshot of the current screen is attached. Decide how to recover.`;
        addAiMsg(`✕ Step ${stepNumber} failed: ${errMsg} — AI will adapt.`);
        if (tauri) await tauri.event.emit("overlay-say", { text: `Failed: ${errMsg}` });
      } else {
        setDone();
        const okMsg = result?.message ?? "Completed";
        const entry = `[Step ${stepNumber}] ${actionName} → SUCCESS: ${okMsg}`;
        fullLog  += entry + "\n";
        // Give AI the result AND tell it NOT to repeat this step
        agentCtx += `\n${entry}. This step is DONE — do NOT repeat it. A screenshot of the current screen is attached. Decide the NEXT action needed to complete the goal.`;
        if (tauri) await tauri.event.emit("overlay-say", { text: okMsg });
      }
    } catch (err) {
      setError();
      const entry = `[Step ${stepNumber}] ${step.action.action} → ERROR: ${err}`;
      fullLog  += entry + "\n";
      agentCtx += `\n${entry}. Handle this error and decide what to do next.`;
      console.error(err);
    }
  }

  if (stepNumber >= MAX_STEPS) {
    addAiMsg(`⚠️ Reached the ${MAX_STEPS}-step safety limit. Task may be incomplete.`);
    setStatus("Limit reached", "idle");
    if (tauri) await tauri.event.emit("overlay-teach-step", { done: true });
  }

  return fullLog;
}

async function runExecution(plan: ActionPlan): Promise<string> {
  const isTeach = teachToggle.checked;
  const tauri   = getTauri();
  const steps   = addExecSteps(plan);
  setStatus(isTeach ? "Teaching mode..." : "Executing...", "working");

  // ── Helper: show a teach-panel card for the current step ──
  async function announceStep(i: number) {
    if (!isTeach || !tauri) return;
    const step = plan.steps[i];
    const rawAction = step.action.action ?? "";
    // Make action name human-readable: "desktop.open_app" → "Open App"
    const title = rawAction
      .replace(/^(desktop|photoshop)\./, "")
      .replace(/_/g, " ")
      .replace(/\b\w/g, c => c.toUpperCase());

    const description =
      step.explanation ||
      step.expectedResult ||
      `Performing: ${rawAction}`;

    await tauri.event.emit("overlay-teach-step", {
      step: i + 1,
      total: plan.steps.length,
      title,
      description,
      status: "Watching...",
      done: false,
    });
    // Give user time to read the panel before the step fires
    await sleep(2200);
  }

    if (isTeach && tauri) {
    // ── Teach mode: execute one step at a time ──
    let summary = "";
    let hasError = false;
    let detailedLog = "Mode: Teach\n";

    for (let i = 0; i < plan.steps.length; i++) {
      steps.setRunning(i);
      await announceStep(i);

      // Build a single-step mini-plan for this iteration
      const miniPlan: ActionPlan = { ...plan, steps: [plan.steps[i]] };
      const actionName = plan.steps[i].action.action;

      try {
        const report = await executePlan(miniPlan, ghostToggle.checked);
        const stepRes = report.results[0];

        if (stepRes && stepRes.status === "failed") {
          steps.setError(i);
          hasError = true;
          // Mark remaining as error
          for (let j = i + 1; j < plan.steps.length; j++) steps.setError(j);
          const errMsg = `Step ${i + 1} failed: ${stepRes.message || "Unknown error"}`;
          detailedLog += `Step ${i + 1} (${actionName}): FAILED - ${errMsg}\n`;
          addAiMsg(`✕ ${errMsg}`);
          await tauri.event.emit("overlay-say", { text: errMsg });
          await tauri.event.emit("overlay-teach-step", { done: true });
          setStatus("Execution failed", "error");
          return detailedLog;
        } else {
          steps.setDone(i);
          summary = report.summary;
          detailedLog += `Step ${i + 1} (${actionName}): SUCCESS\n`;
        }
      } catch (err) {
        steps.setError(i);
        for (let j = i + 1; j < plan.steps.length; j++) steps.setError(j);
        addAiMsg("Execution error during teach mode.");
        detailedLog += `Step ${i + 1} (${actionName}): ERROR - ${err}\n`;
        await tauri.event.emit("overlay-teach-step", { done: true });
        setStatus("Execution failed", "error");
        console.error(err);
        return detailedLog;
      }
    }

    // All done
    await tauri.event.emit("overlay-teach-step", { done: true });
    const doneMsg = hasError
      ? "Teaching finished with some errors."
      : `✓ ${summary || "All steps completed."}`;
    detailedLog += `Final Summary: ${summary}`;
    addAiMsg(doneMsg);
    await tauri.event.emit("overlay-say", { text: "✓ Done!" });
    setStatus("Complete", "ok");
    return detailedLog;

  } else {
    // ── Normal mode: execute whole plan at once ──
    let detailedLog = "Mode: Normal\n";
    try {
      const report = await executePlan(plan, ghostToggle.checked);

      for (let i = 0; i < plan.steps.length; i++) {
        steps.setRunning(i);
        await sleep(300);
        const actionName = plan.steps[i].action.action;
        const stepRes = report.results.find(r => r.stepId === plan.steps[i].stepId) || report.results[i];
        
        if (stepRes && stepRes.status === "failed") {
          steps.setError(i);
          for (let j = i + 1; j < plan.steps.length; j++) steps.setError(j);
          const errMsg = `Step failed: ${stepRes.message || "Unknown error"}`;
          detailedLog += `Step ${i + 1} (${actionName}): FAILED - ${errMsg}\n`;
          addAiMsg(`✕ ${errMsg}`);
          getTauri()?.event.emit("overlay-say", { text: errMsg });
          setStatus("Execution failed", "error");
          return detailedLog;
        } else {
          steps.setDone(i);
          detailedLog += `Step ${i + 1} (${actionName}): SUCCESS\n`;
        }
      }

      detailedLog += `Final Summary: ${report.summary}`;
      addAiMsg(`✓ ${report.summary}`);
      getTauri()?.event.emit("overlay-say", { text: report.summary });
      setStatus("Complete", "ok");
      return detailedLog;
    } catch (err) {
      plan.steps.forEach((_, i) => steps.setError(i));
      addAiMsg("Execution failed.");
      setStatus("Execution failed", "error");
      console.error(err);
      return `Execution failed: ${err}`;
    }
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise(r => setTimeout(r, ms));
}
} // end initMainApp


