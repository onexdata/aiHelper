import { invoke } from "@tauri-apps/api/core";

let tipTimer = null;
let isGenerating = false;
let intervalMinutes = 5;
let backgroundEnabled = false;

const slider = () => document.getElementById("tips-interval-slider");
const sliderValue = () => document.getElementById("tips-interval-value");
const checkbox = () => document.getElementById("tips-background-checkbox");
const generateBtn = () => document.getElementById("tips-generate-now-btn");
const feed = () => document.getElementById("tips-feed");
const loading = () => document.getElementById("tips-loading");

export async function initTips() {
  // Wire slider
  const s = slider();
  if (s) {
    s.addEventListener("input", () => {
      sliderValue().textContent = `${s.value} min`;
    });
    s.addEventListener("change", () => {
      intervalMinutes = parseInt(s.value, 10);
      invoke("update_setting", { key: "tips_interval_minutes", value: String(intervalMinutes) });
      // Restart timer if running
      if (tipTimer) {
        stopTimer();
        startTimer();
      }
    });
  }

  // Wire checkbox
  const cb = checkbox();
  if (cb) {
    cb.addEventListener("change", () => {
      backgroundEnabled = cb.checked;
      invoke("update_setting", { key: "tips_background_enabled", value: String(backgroundEnabled) });
    });
  }

  // Wire Generate Now button
  const btn = generateBtn();
  if (btn) {
    btn.addEventListener("click", () => generateTip());
  }

  // Load persisted settings
  try {
    const saved = await invoke("get_setting", { key: "tips_interval_minutes" });
    if (saved) {
      intervalMinutes = parseInt(saved, 10) || 5;
      if (s) {
        s.value = intervalMinutes;
        sliderValue().textContent = `${intervalMinutes} min`;
      }
    }
  } catch (_) {}

  try {
    const bgSaved = await invoke("get_setting", { key: "tips_background_enabled" });
    if (bgSaved === "true") {
      backgroundEnabled = true;
      if (cb) cb.checked = true;
      startTimer();
    }
  } catch (_) {}
}

export function loadTips() {
  if (!tipTimer) {
    startTimer();
  }
}

export function stopTips() {
  if (!backgroundEnabled) {
    stopTimer();
  }
}

function startTimer() {
  if (tipTimer) return;
  generateTip();
  tipTimer = setInterval(() => generateTip(), intervalMinutes * 60000);
}

function stopTimer() {
  if (tipTimer) {
    clearInterval(tipTimer);
    tipTimer = null;
  }
}

async function generateTip() {
  if (isGenerating) return;
  isGenerating = true;

  const btn = generateBtn();
  if (btn) btn.disabled = true;
  const ld = loading();
  if (ld) ld.style.display = "flex";

  try {
    const tip = await invoke("generate_tip");
    appendTipCard(tip, false);
  } catch (err) {
    appendTipCard(String(err), true);
  } finally {
    isGenerating = false;
    if (btn) btn.disabled = false;
    if (ld) ld.style.display = "none";
  }
}

function appendTipCard(text, isError) {
  const f = feed();
  if (!f) return;

  const card = document.createElement("div");
  card.className = "tip-card" + (isError ? " tip-card-error" : "");

  const time = document.createElement("div");
  time.className = "tip-card-time";
  time.textContent = new Date().toLocaleTimeString();

  const body = document.createElement("div");
  body.className = "tip-card-body";
  body.textContent = text;

  card.appendChild(time);
  card.appendChild(body);
  f.appendChild(card);
  f.scrollTop = f.scrollHeight;
}
