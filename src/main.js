import { getCurrentWindow } from "@tauri-apps/api/window";
import { initTabs } from "./tabs.js";
import { initHistory, loadConversationList } from "./history.js";
import { initChat, openConversation, startNewConversation, getIsStreaming } from "./chat.js";
import { initTasks } from "./tasks.js";
import { initSettings, loadSettings } from "./settings.js";
import { initStats } from "./stats.js";
import { initProjects } from "./projects.js";
import { startTagger } from "./tagger.js";
import { initTips } from "./tips.js";
import { initNotes } from "./notes.js";

const historyView = document.getElementById("history-view");
const chatView = document.getElementById("chat-view");
const backBtn = document.getElementById("back-btn");
const closeBtn = document.getElementById("close-btn");
const chatNewBtn = document.getElementById("chat-new-btn");

function showView(view) {
  if (view === "history") {
    historyView.style.display = "flex";
    chatView.style.display = "none";
    loadConversationList();
  } else {
    historyView.style.display = "none";
    chatView.style.display = "flex";
  }
}

// Init tabs (before history so DOM is wired up)
initTabs();

// Init modules
initHistory({
  onSelectConversation: async (id) => {
    showView("chat");
    await openConversation(id);
  },
  onNewChatClick: async () => {
    showView("chat");
    await startNewConversation();
  },
});

initChat();
initTasks();
initSettings();
loadSettings();
initStats();
initProjects();
initTips();
initNotes();
startTagger();

// Wire header buttons
backBtn.addEventListener("click", () => {
  if (getIsStreaming()) return;
  showView("history");
});

chatNewBtn.addEventListener("click", async () => {
  if (getIsStreaming()) return;
  await startNewConversation();
});

closeBtn.addEventListener("click", () => {
  getCurrentWindow().hide();
});

// --- Drag regions (headers) for decorationless window ---
document.querySelectorAll(".drag-region").forEach((el) => {
  el.addEventListener("mousedown", (e) => {
    // Don't drag when clicking buttons or interactive elements
    if (e.target.closest("button, input, textarea, select, a")) return;
    e.preventDefault();
    getCurrentWindow().startDragging();
  });
});

// --- Resize handles for decorationless window ---
const resizeDirections = {
  n: "North", s: "South", e: "East", w: "West",
  ne: "NorthEast", nw: "NorthWest", se: "SouthEast", sw: "SouthWest",
};
const overlay = document.querySelector(".overlay");
for (const [dir, tauriDir] of Object.entries(resizeDirections)) {
  const handle = document.createElement("div");
  handle.className = `resize-handle resize-${dir}`;
  handle.addEventListener("mousedown", (e) => {
    e.preventDefault();
    getCurrentWindow().startResizeDragging(tauriDir);
  });
  overlay.appendChild(handle);
}

// Start on history view
showView("history");
