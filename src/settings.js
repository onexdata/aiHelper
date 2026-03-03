import { invoke } from "@tauri-apps/api/core";

let saveBtn;
let toastEl;

export function initSettings() {
  saveBtn = document.getElementById("settings-save-btn");
  toastEl = document.getElementById("settings-toast");

  // Wire save button
  saveBtn.addEventListener("click", saveSettings);

  // Wire Show/Hide toggle for password fields
  document.querySelectorAll(".settings-toggle-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      const key = btn.dataset.toggle;
      const input = document.querySelector(`[data-key="${key}"]`);
      if (!input) return;
      if (input.type === "password") {
        input.type = "text";
        btn.textContent = "Hide";
      } else {
        input.type = "password";
        btn.textContent = "Show";
      }
    });
  });

  // Wire range sliders for live preview
  document.querySelectorAll(".settings-range").forEach((range) => {
    range.addEventListener("input", () => {
      const key = range.dataset.key;
      const label = document.querySelector(`[data-range-for="${key}"]`);
      if (label) label.textContent = range.value;

      // Live CSS preview
      if (key === "overlay_opacity") {
        document.documentElement.style.setProperty("--overlay-opacity", range.value);
      } else if (key === "tab_icon_size") {
        document.documentElement.style.setProperty("--tab-icon-size", range.value + "px");
      }
    });
  });
}

export async function loadSettings() {
  try {
    const settings = await invoke("get_settings");
    // Populate all inputs with data-key
    document.querySelectorAll("[data-key]").forEach((input) => {
      const key = input.dataset.key;
      if (key in settings) {
        input.value = settings[key];
      }
    });
    // Update range labels
    document.querySelectorAll(".settings-range-value").forEach((label) => {
      const key = label.dataset.rangeFor;
      if (key in settings) {
        label.textContent = settings[key];
      }
    });
    // Apply appearance settings
    applyAppearanceSettings(settings);
  } catch (e) {
    console.error("Failed to load settings:", e);
  }
}

function applyAppearanceSettings(settings) {
  if (settings.overlay_opacity) {
    document.documentElement.style.setProperty("--overlay-opacity", settings.overlay_opacity);
  }
  if (settings.tab_icon_size) {
    document.documentElement.style.setProperty("--tab-icon-size", settings.tab_icon_size + "px");
  }
}

async function saveSettings() {
  saveBtn.disabled = true;
  const errors = [];

  const inputs = document.querySelectorAll("[data-key]");
  for (const input of inputs) {
    const key = input.dataset.key;
    const value = input.value;

    try {
      if (key === "hotkey") {
        await invoke("update_hotkey", { newHotkey: value });
      } else {
        await invoke("update_setting", { key, value });
      }
    } catch (e) {
      errors.push(`${key}: ${e}`);
    }
  }

  // Apply appearance after save
  const opacity = document.querySelector('[data-key="overlay_opacity"]');
  const iconSize = document.querySelector('[data-key="tab_icon_size"]');
  if (opacity) {
    document.documentElement.style.setProperty("--overlay-opacity", opacity.value);
  }
  if (iconSize) {
    document.documentElement.style.setProperty("--tab-icon-size", iconSize.value + "px");
  }

  saveBtn.disabled = false;

  if (errors.length > 0) {
    showToast("Error: " + errors.join("; "), true);
  } else {
    showToast("Settings saved");
  }
}

function showToast(message, isError = false) {
  toastEl.textContent = message;
  toastEl.className = "settings-toast" + (isError ? " error" : " success");
  toastEl.style.opacity = "1";
  setTimeout(() => {
    toastEl.style.opacity = "0";
  }, 3000);
}
