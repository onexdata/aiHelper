import { getCurrentWindow } from "@tauri-apps/api/window";
import { loadConversationList } from "./history.js";
import { loadTaskList } from "./tasks.js";
import { loadSettings } from "./settings.js";
import { startStatsRefresh, stopStatsRefresh } from "./stats.js";
import { loadProjects } from "./projects.js";
import { loadTips, stopTips } from "./tips.js";
import { loadNoteList } from "./notes.js";

let activeTab = "conversations";

export function initTabs() {
  const tabBar = document.getElementById("tab-bar");
  if (!tabBar) return;

  tabBar.addEventListener("click", (e) => {
    const btn = e.target.closest(".tab-btn");
    if (!btn) return;
    const tabId = btn.dataset.tab;
    if (tabId) switchTab(tabId);
  });

  // Wire close buttons on placeholder panels
  document.querySelectorAll(".close-tab-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      getCurrentWindow().hide();
    });
  });
}

export function switchTab(tabId) {
  if (tabId === activeTab) return;

  // Hide all panels, show target
  document.querySelectorAll(".tab-panel").forEach((panel) => {
    panel.style.display = "none";
  });
  const target = document.getElementById(`${tabId}-panel`);
  if (target) target.style.display = "flex";

  // Toggle active class on tab buttons
  document.querySelectorAll(".tab-btn").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.tab === tabId);
  });

  // Stop stats refresh when leaving stats tab
  if (activeTab === "stats") {
    stopStatsRefresh();
  }

  // Stop tips timer when leaving tips tab (unless background enabled)
  if (activeTab === "tips") {
    stopTips();
  }

  activeTab = tabId;

  // Refresh conversation list when switching back to conversations
  if (tabId === "conversations") {
    loadConversationList();
  }

  if (tabId === "tasks") {
    loadTaskList();
  }

  if (tabId === "projects") {
    loadProjects();
  }

  if (tabId === "settings") {
    loadSettings();
  }

  if (tabId === "stats") {
    startStatsRefresh();
  }

  if (tabId === "notes") {
    loadNoteList();
  }

  if (tabId === "tips") {
    loadTips();
  }
}

export function getActiveTab() {
  return activeTab;
}
