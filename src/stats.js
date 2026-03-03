import { invoke } from "@tauri-apps/api/core";

let refreshInterval = null;
let showRaw = false;
let lastRecentData = [];

export function initStats() {
  const toggle = document.getElementById("stats-raw-toggle");
  if (toggle) {
    toggle.addEventListener("click", () => {
      showRaw = !showRaw;
      toggle.textContent = showRaw ? "Raw" : "Clean";
      renderRecentInput(lastRecentData);
    });
  }
}

export function startStatsRefresh() {
  loadStats();
  if (!refreshInterval) {
    refreshInterval = setInterval(loadStats, 2000);
  }
}

export function stopStatsRefresh() {
  if (refreshInterval) {
    clearInterval(refreshInterval);
    refreshInterval = null;
  }
}

async function loadStats() {
  try {
    const [stats, recent, windows] = await Promise.all([
      invoke("get_input_stats"),
      invoke("get_recent_input", { limitBytes: 5120 }),
      invoke("get_top_windows"),
    ]);

    renderWpm(stats.wpm);
    renderKeystrokCards(stats.keystroke_counts);
    renderMouseCards(stats.mouse_distances_physical);
    renderTopWindows(windows);
    lastRecentData = recent;
    renderRecentInput(recent);
  } catch (e) {
    console.error("Failed to load stats:", e);
  }
}

function renderWpm(wpm) {
  const el = document.getElementById("stats-wpm");
  if (el) el.textContent = Math.round(wpm);
}

const PERIOD_ORDER = ["second", "minute", "hour", "day", "week", "month", "year"];
const PERIOD_LABELS = {
  second: "1s",
  minute: "1m",
  hour: "1h",
  day: "24h",
  week: "7d",
  month: "30d",
  year: "1y",
};

function renderKeystrokCards(counts) {
  const container = document.getElementById("stats-keystrokes");
  if (!container) return;
  container.innerHTML = "";
  for (const period of PERIOD_ORDER) {
    const val = counts[period] || 0;
    container.appendChild(makeCard(formatNumber(val), PERIOD_LABELS[period]));
  }
}

function renderMouseCards(distances) {
  const container = document.getElementById("stats-mouse");
  if (!container) return;
  container.innerHTML = "";
  for (const period of PERIOD_ORDER) {
    const inches = distances[period] || 0;
    container.appendChild(makeCard(formatDistance(inches), PERIOD_LABELS[period]));
  }
}

function makeCard(value, label) {
  const card = document.createElement("div");
  card.className = "stats-card";
  card.innerHTML = `<div class="stats-card-value">${value}</div><div class="stats-card-label">${label}</div>`;
  return card;
}

function renderTopWindows(windows) {
  const container = document.getElementById("stats-windows");
  if (!container) return;
  if (windows.length === 0) {
    container.innerHTML = '<div class="stats-empty">No window activity yet</div>';
    return;
  }
  container.innerHTML = "";
  for (const w of windows) {
    const row = document.createElement("div");
    row.className = "stats-window-row";
    const title = w.title || "(untitled)";
    row.innerHTML = `<span class="stats-window-title">${escapeHtml(title)}</span><span class="stats-window-duration">${formatDuration(w.duration_secs)}</span>`;
    container.appendChild(row);
  }
}

function renderRecentInput(groups) {
  const container = document.getElementById("stats-recent");
  if (!container) return;
  if (groups.length === 0) {
    container.innerHTML = '<div class="stats-empty">No input recorded yet</div>';
    return;
  }
  container.innerHTML = "";
  for (const g of groups) {
    const group = document.createElement("div");
    group.className = "stats-input-group";
    const title = g.app_name || "(untitled)";

    const labelDiv = document.createElement("div");
    labelDiv.className = "stats-input-label";
    labelDiv.textContent = title;

    const textDiv = document.createElement("div");
    textDiv.className = "stats-input-text";

    if (showRaw) {
      textDiv.innerHTML = renderRawHtml(g.text);
    } else {
      textDiv.textContent = replayClean(g.text);
    }

    group.appendChild(labelDiv);
    group.appendChild(textDiv);
    container.appendChild(group);
  }
}

// --- Special key tag pattern ---
const TAG_RE = /\[(BS|ENTER|TAB|ESC|DEL|LEFT|RIGHT|UP|DOWN)\]/g;

// Display labels for raw mode pills
const TAG_DISPLAY = {
  BS: "\u232B",      // ⌫
  ENTER: "\u23CE",   // ⏎
  TAB: "\u21E5",     // ⇥
  ESC: "Esc",
  DEL: "Del",
  LEFT: "\u2190",    // ←
  RIGHT: "\u2192",   // →
  UP: "\u2191",      // ↑
  DOWN: "\u2193",    // ↓
};

// Pill CSS class per tag type
const TAG_CLASS = {
  BS: "key-destructive",
  DEL: "key-destructive",
  ENTER: "key-action",
  TAB: "key-action",
  ESC: "key-nav",
  LEFT: "key-nav",
  RIGHT: "key-nav",
  UP: "key-nav",
  DOWN: "key-nav",
};

/**
 * Clean view: replay the raw stream, applying backspaces/enters/tabs
 * and stripping navigation keys. Returns the "final text" as if displayed on screen.
 */
function replayClean(raw) {
  const result = [];
  let i = 0;
  while (i < raw.length) {
    if (raw[i] === "[") {
      const slice = raw.slice(i);
      const m = slice.match(/^\[(BS|ENTER|TAB|ESC|DEL|LEFT|RIGHT|UP|DOWN)\]/);
      if (m) {
        switch (m[1]) {
          case "BS":
            if (result.length > 0) result.pop();
            break;
          case "ENTER":
            result.push("\n");
            break;
          case "TAB":
            result.push("\t");
            break;
          // DEL, ESC, arrows: skip in clean view
        }
        i += m[0].length;
        continue;
      }
    }
    result.push(raw[i]);
    i++;
  }
  return result.join("");
}

/**
 * Raw view: returns HTML with visual pills for special keys
 * and regular characters shown as-is.
 */
function renderRawHtml(raw) {
  let html = "";
  let i = 0;
  while (i < raw.length) {
    if (raw[i] === "[") {
      const slice = raw.slice(i);
      const m = slice.match(/^\[(BS|ENTER|TAB|ESC|DEL|LEFT|RIGHT|UP|DOWN)\]/);
      if (m) {
        const tag = m[1];
        const cls = TAG_CLASS[tag] || "key-nav";
        const label = TAG_DISPLAY[tag] || tag;
        html += `<span class="key-pill ${cls}">${label}</span>`;
        if (tag === "ENTER") html += "\n";
        i += m[0].length;
        continue;
      }
    }
    html += escapeHtml(raw[i]);
    i++;
  }
  return html;
}

// --- Helpers ---

function formatNumber(n) {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + "M";
  if (n >= 1_000) return (n / 1_000).toFixed(1) + "K";
  return n.toString();
}

function formatDuration(secs) {
  if (secs < 60) return secs + "s";
  if (secs < 3600) return Math.floor(secs / 60) + "m " + (secs % 60) + "s";
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  return h + "h " + m + "m";
}

function formatDistance(inches) {
  if (inches < 1) return inches.toFixed(1) + " in";
  if (inches < 12) return inches.toFixed(1) + " in";
  const feet = inches / 12;
  if (feet < 5280) return feet.toFixed(1) + " ft";
  const miles = feet / 5280;
  return miles.toFixed(2) + " mi";
}

function escapeHtml(str) {
  const div = document.createElement("div");
  div.textContent = str;
  return div.innerHTML;
}
