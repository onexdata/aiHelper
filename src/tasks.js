import { invoke } from "@tauri-apps/api/core";
import { formatRelativeTime } from "./history.js";

let taskListEl;
let taskInputEl;
let taskInputRow;
let taskContextEl;
let activeSubTab = "active";
let archiveDelaySecs = 5;

// Map of taskId -> timeoutId for pending archive timers
const archiveTimers = new Map();

export function initTasks() {
  taskListEl = document.getElementById("task-list");
  taskInputEl = document.getElementById("task-input");
  taskInputRow = document.getElementById("task-input-row");
  taskContextEl = document.getElementById("task-context");

  // Enter to create task
  taskInputEl.addEventListener("keydown", async (e) => {
    if (e.key === "Enter") {
      e.preventDefault();
      const content = taskInputEl.value.trim();
      if (!content) return;
      taskInputEl.value = "";
      try {
        await invoke("create_task", { content });
        await loadTaskList();
      } catch (err) {
        console.error("Failed to create task:", err);
      }
    }
  });

  // Sub-tab switching
  document.querySelectorAll(".tasks-sub-tab").forEach((btn) => {
    btn.addEventListener("click", () => {
      const subtab = btn.dataset.subtab;
      if (subtab === activeSubTab) return;
      activeSubTab = subtab;
      document.querySelectorAll(".tasks-sub-tab").forEach((b) => {
        b.classList.toggle("active", b.dataset.subtab === subtab);
      });
      // Show/hide input row
      taskInputRow.style.display = subtab === "active" ? "flex" : "none";
      loadTaskList();
    });
  });

  // Load config for archive delay
  loadArchiveDelay();
}

async function loadArchiveDelay() {
  try {
    const val = await invoke("get_setting", { key: "task_archive_delay_secs" });
    archiveDelaySecs = val ? parseInt(val, 10) || 5 : 5;
  } catch (e) {
    console.error("Failed to load archive delay setting:", e);
  }
}

export async function loadTaskList() {
  try {
    const archived = activeSubTab === "archived";
    const tasks = await invoke("list_tasks", { archived });
    renderTaskList(tasks);
    // Update context display
    await updateContext();
  } catch (e) {
    console.error("Failed to load tasks:", e);
    taskListEl.innerHTML = "";
  }
}

async function updateContext() {
  try {
    const title = await invoke("get_foreground_title");
    if (title) {
      taskContextEl.textContent = `From: ${title}`;
      taskContextEl.style.display = "block";
    } else {
      taskContextEl.style.display = "none";
    }
  } catch {
    taskContextEl.style.display = "none";
  }
}

function renderTaskList(tasks) {
  taskListEl.innerHTML = "";

  if (tasks.length === 0) {
    const empty = document.createElement("div");
    empty.className = "empty-state";
    empty.textContent = activeSubTab === "active" ? "No tasks yet" : "No archived tasks";
    taskListEl.appendChild(empty);
    return;
  }

  for (const task of tasks) {
    taskListEl.appendChild(renderTaskItem(task));
  }
}

function renderTaskItem(task) {
  const item = document.createElement("div");
  item.className = "task-item" + (task.completed ? " task-completed" : "");
  item.dataset.taskId = task.id;

  // Top row: checkbox + content + countdown + delete
  const row = document.createElement("div");
  row.className = "task-row";

  const checkbox = document.createElement("input");
  checkbox.type = "checkbox";
  checkbox.className = "task-checkbox";
  checkbox.checked = task.completed;
  checkbox.addEventListener("change", () => {
    handleToggle(task.id, checkbox.checked, item);
  });

  const content = document.createElement("span");
  content.className = "task-content";
  content.textContent = task.content;

  const countdown = document.createElement("span");
  countdown.className = "task-countdown";
  countdown.id = `task-countdown-${task.id}`;
  // If there's an active timer, show indicator
  if (archiveTimers.has(task.id)) {
    countdown.textContent = "archiving...";
  }

  const deleteBtn = document.createElement("button");
  deleteBtn.className = "task-delete-btn";
  deleteBtn.title = "Delete";
  deleteBtn.textContent = "\u00d7";
  deleteBtn.addEventListener("click", async (e) => {
    e.stopPropagation();
    cancelArchiveTimer(task.id);
    try {
      await invoke("delete_task", { taskId: task.id });
      await loadTaskList();
    } catch (err) {
      console.error("Failed to delete task:", err);
    }
  });

  row.appendChild(checkbox);
  row.appendChild(content);
  row.appendChild(countdown);
  row.appendChild(deleteBtn);

  // Meta row: window title + created time + completed time
  const meta = document.createElement("div");
  meta.className = "task-meta";
  const parts = [];
  if (task.window_title) parts.push(task.window_title);
  parts.push(formatRelativeTime(task.created_at));
  if (task.completed_at) parts.push("done " + formatRelativeTime(task.completed_at));
  meta.textContent = parts.join(" \u2022 ");

  item.appendChild(row);
  item.appendChild(meta);

  return item;
}

async function handleToggle(taskId, completed, itemEl) {
  try {
    await invoke("update_task_completed", { taskId, completed });

    if (completed) {
      itemEl.classList.add("task-completed");
      startArchiveTimer(taskId);
    } else {
      itemEl.classList.remove("task-completed");
      cancelArchiveTimer(taskId);
      // Unchecking also unarchives in the DB, so reload to move it between tabs
      await loadTaskList();
    }
  } catch (err) {
    console.error("Failed to toggle task:", err);
  }
}

function startArchiveTimer(taskId) {
  cancelArchiveTimer(taskId);

  const countdownEl = document.getElementById(`task-countdown-${taskId}`);
  if (countdownEl) countdownEl.textContent = "archiving...";

  const timerId = setTimeout(async () => {
    archiveTimers.delete(taskId);
    try {
      await invoke("archive_task", { taskId });
      await loadTaskList();
    } catch (err) {
      console.error("Failed to archive task:", err);
    }
  }, archiveDelaySecs * 1000);

  archiveTimers.set(taskId, timerId);
}

function cancelArchiveTimer(taskId) {
  if (archiveTimers.has(taskId)) {
    clearTimeout(archiveTimers.get(taskId));
    archiveTimers.delete(taskId);
    const countdownEl = document.getElementById(`task-countdown-${taskId}`);
    if (countdownEl) countdownEl.textContent = "";
  }
}
