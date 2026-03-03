import { invoke } from "@tauri-apps/api/core";
import { runTagger } from "./tagger.js";

let projectsBody;
let projectsToolbar;
let projectsUntagged;

export function initProjects() {
  const newBtn = document.getElementById("projects-new-btn");
  const suggestBtn = document.getElementById("projects-suggest-btn");
  const smartSuggestBtn = document.getElementById("projects-smart-suggest-btn");

  if (newBtn) newBtn.addEventListener("click", handleCreateProject);
  if (suggestBtn) suggestBtn.addEventListener("click", handleSuggestProjects);
  if (smartSuggestBtn) smartSuggestBtn.addEventListener("click", handleSmartSuggest);

  projectsBody = document.getElementById("projects-body");
  projectsUntagged = document.getElementById("projects-untagged");
}

export async function loadProjects() {
  // Fetch projects and summaries independently so one failure doesn't block the other
  let projects = [];
  let summaries = [];

  try {
    projects = await invoke("list_projects");
  } catch (e) {
    console.error("Failed to list projects:", e);
  }

  try {
    summaries = await invoke("get_all_project_summaries_today");
  } catch (e) {
    console.error("Failed to get project summaries:", e);
  }

  renderProjectList(projects, summaries);

  // Untagged summary with app breakdown
  try {
    const summary = await invoke("get_untagged_summary");
    if (projectsUntagged) {
      if (summary.total > 0) {
        projectsUntagged.style.display = "block";
        projectsUntagged.innerHTML = "";

        const label = document.createElement("span");
        label.className = "untagged-label";
        label.textContent = `Untagged: ${formatNumber(summary.total)} activities`;
        projectsUntagged.appendChild(label);

        if (summary.by_app.length > 0) {
          const pills = document.createElement("span");
          pills.className = "untagged-breakdown";
          const maxPills = 4;
          const shown = summary.by_app.slice(0, maxPills);
          for (const app of shown) {
            const pill = document.createElement("span");
            pill.className = "untagged-pill";
            pill.textContent = `${app.app_name}: ${formatNumber(app.total)}`;
            pills.appendChild(pill);
          }
          if (summary.by_app.length > maxPills) {
            const more = document.createElement("span");
            more.className = "untagged-pill untagged-pill-more";
            more.textContent = `+${summary.by_app.length - maxPills} more`;
            pills.appendChild(more);
          }
          projectsUntagged.appendChild(pills);
        }
      } else {
        projectsUntagged.style.display = "none";
      }
    }
  } catch (e) {
    console.error("Failed to get untagged summary:", e);
    if (projectsUntagged) projectsUntagged.style.display = "none";
  }
}

function renderProjectList(projects, summaries) {
  if (!projectsBody) return;

  // Clear existing cards
  const existing = projectsBody.querySelectorAll(".project-card, .empty-state");
  existing.forEach((el) => el.remove());

  if (projects.length === 0) {
    const empty = document.createElement("div");
    empty.className = "empty-state";
    empty.textContent = "No projects yet";
    projectsBody.appendChild(empty);
    return;
  }

  // Build summary map keyed by project_id
  const summaryMap = new Map();
  for (const s of summaries) {
    if (s.project_id != null) summaryMap.set(s.project_id, s);
  }

  for (const project of projects) {
    const summary = summaryMap.get(project.id) || {
      keystroke_count: 0,
      active_secs: 0,
    };
    projectsBody.appendChild(renderProjectCard(project, summary));
  }
}

function renderProjectCard(project, summary) {
  const card = document.createElement("div");
  card.className = "project-card";
  card.style.borderLeftColor = project.color;
  card.dataset.projectId = project.id;

  // Header row
  const header = document.createElement("div");
  header.className = "project-card-header";

  const name = document.createElement("span");
  name.className = "project-card-name";
  name.textContent = project.name;

  const duration = document.createElement("span");
  duration.className = "project-card-duration";
  duration.textContent = formatDuration(summary.active_secs);

  const menuBtn = document.createElement("button");
  menuBtn.className = "project-menu-btn";
  menuBtn.textContent = "\u22EE"; // vertical ellipsis
  menuBtn.title = "Options";
  menuBtn.addEventListener("click", (e) => {
    e.stopPropagation();
    showProjectMenu(project, menuBtn);
  });

  header.appendChild(name);
  header.appendChild(duration);
  header.appendChild(menuBtn);

  // Stats row
  const stats = document.createElement("div");
  stats.className = "project-card-stats";
  stats.textContent = `${formatNumber(summary.keystroke_count)} keys  \u00B7  ${formatDuration(summary.active_secs)} active`;

  // Collapsible rules section (hidden by default)
  const rulesSection = document.createElement("div");
  rulesSection.className = "project-card-rules";
  rulesSection.style.display = "none";
  rulesSection.dataset.projectId = project.id;

  card.appendChild(header);
  card.appendChild(stats);
  card.appendChild(rulesSection);

  // Click to expand/collapse
  card.addEventListener("click", (e) => {
    if (e.target.closest(".project-menu-btn, .project-rule-delete, .project-rule-input, .project-rule-add-btn, .project-activity-section")) return;
    toggleRulesSection(project.id, rulesSection);
  });

  return card;
}

async function toggleRulesSection(projectId, section) {
  if (section.style.display === "none") {
    section.style.display = "block";
    await loadRulesForProject(projectId, section);
  } else {
    section.style.display = "none";
  }
}

async function loadRulesForProject(projectId, section) {
  try {
    const rules = await invoke("get_project_rules", { projectId });
    section.innerHTML = "";

    // Description (not editable for now, just show)
    const ruleLabel = document.createElement("div");
    ruleLabel.className = "project-rules-label";
    ruleLabel.textContent = `Rules (${rules.length})`;
    section.appendChild(ruleLabel);

    for (const rule of rules) {
      const row = document.createElement("div");
      row.className = "project-rule-row";

      const expr = document.createElement("code");
      expr.className = "project-rule-expr";
      expr.textContent = rule.expression;

      const delBtn = document.createElement("button");
      delBtn.className = "project-rule-delete";
      delBtn.textContent = "\u00D7";
      delBtn.title = "Delete rule";
      delBtn.addEventListener("click", async (e) => {
        e.stopPropagation();
        try {
          await invoke("delete_project_rule", { id: rule.id });
          await loadRulesForProject(projectId, section);
        } catch (err) {
          console.error("Failed to delete rule:", err);
        }
      });

      row.appendChild(expr);
      row.appendChild(delBtn);
      section.appendChild(row);
    }

    // Add rule input
    const addRow = document.createElement("div");
    addRow.className = "project-rule-add-row";

    const input = document.createElement("input");
    input.className = "project-rule-input";
    input.type = "text";
    input.placeholder = 'JSONata expression, e.g. app_name = "Code"';

    const addBtn = document.createElement("button");
    addBtn.className = "project-rule-add-btn";
    addBtn.textContent = "+";
    addBtn.title = "Add rule";
    addBtn.addEventListener("click", async () => {
      const expression = input.value.trim();
      if (!expression) return;
      try {
        await invoke("add_project_rule", { projectId, expression, priority: 0 });
        input.value = "";
        await loadRulesForProject(projectId, section);
      } catch (err) {
        console.error("Failed to add rule:", err);
      }
    });

    input.addEventListener("keydown", async (e) => {
      if (e.key === "Enter") {
        e.preventDefault();
        e.stopPropagation();
        addBtn.click();
      }
    });

    addRow.appendChild(input);
    addRow.appendChild(addBtn);
    section.appendChild(addRow);

    // Recent Activity section
    try {
      const activities = await invoke("get_project_activities", { projectId, limit: 20 });
      const actSection = document.createElement("div");
      actSection.className = "project-activity-section";

      const actLabel = document.createElement("div");
      actLabel.className = "project-rules-label";
      actLabel.textContent = "Recent Activity";
      actSection.appendChild(actLabel);

      if (activities.length === 0) {
        const empty = document.createElement("div");
        empty.className = "project-activity-empty";
        empty.textContent = "No tagged activity yet";
        actSection.appendChild(empty);
      } else {
        const list = document.createElement("div");
        list.className = "project-activity-list";
        for (const act of activities) {
          const row = document.createElement("div");
          row.className = "project-activity-row";

          const app = document.createElement("span");
          app.className = "project-activity-app";
          app.textContent = act.app_name;

          const title = document.createElement("span");
          title.className = "project-activity-title";
          title.textContent = act.window_title;
          title.title = act.window_title;

          const dur = document.createElement("span");
          dur.className = "project-activity-duration";
          dur.textContent = formatDuration(act.duration_secs);

          row.appendChild(app);
          row.appendChild(title);
          row.appendChild(dur);
          list.appendChild(row);
        }
        actSection.appendChild(list);
      }
      section.appendChild(actSection);
    } catch (err) {
      console.error("Failed to load project activities:", err);
    }
  } catch (e) {
    console.error("Failed to load rules:", e);
  }
}

function showProjectMenu(project, anchor) {
  // Remove any existing menu
  document.querySelectorAll(".project-context-menu").forEach((m) => m.remove());

  const menu = document.createElement("div");
  menu.className = "project-context-menu";

  const deleteItem = document.createElement("button");
  deleteItem.className = "project-context-item danger";
  deleteItem.textContent = "Delete project";
  deleteItem.addEventListener("click", async () => {
    menu.remove();
    await handleDeleteProject(project.id);
  });

  const clearItem = document.createElement("button");
  clearItem.className = "project-context-item";
  clearItem.textContent = "Clear tags & re-tag";
  clearItem.addEventListener("click", async () => {
    menu.remove();
    try {
      await invoke("clear_project_tags", { projectId: project.id });
      await loadProjects();
    } catch (err) {
      console.error("Failed to clear tags:", err);
    }
  });

  menu.appendChild(clearItem);
  menu.appendChild(deleteItem);

  // Position near anchor
  anchor.style.position = "relative";
  anchor.parentElement.style.position = "relative";
  anchor.parentElement.appendChild(menu);

  // Close on click outside
  const close = (e) => {
    if (!menu.contains(e.target)) {
      menu.remove();
      document.removeEventListener("click", close);
    }
  };
  setTimeout(() => document.addEventListener("click", close), 0);
}

async function handleCreateProject() {
  const name = prompt("Project name:");
  if (!name || !name.trim()) return;
  try {
    await invoke("create_project", {
      name: name.trim(),
      description: "",
      color: "#93bbfc",
    });
    await loadProjects();
  } catch (e) {
    console.error("Failed to create project:", e);
  }
}

async function handleSuggestProjects() {
  let suggestions;
  try {
    suggestions = await invoke("suggest_projects");
  } catch (e) {
    console.error("Failed to get suggestions:", e);
    return;
  }

  if (suggestions.length === 0) {
    alert("No suggestions available. Use more apps to generate data first.");
    return;
  }

  // Show each suggestion and let user accept/reject
  for (const suggestion of suggestions) {
    const accept = confirm(
      `Create project "${suggestion.name}"?\n\nRules:\n${suggestion.rules.join("\n")}`
    );
    if (accept) {
      try {
        const project = await invoke("create_project", {
          name: suggestion.name,
          description: "Auto-suggested project",
          color: getRandomColor(),
        });
        for (const expr of suggestion.rules) {
          try {
            await invoke("add_project_rule", {
              projectId: project.id,
              expression: expr,
              priority: 0,
            });
          } catch (err) {
            console.error("Failed to add rule:", err);
          }
        }
      } catch (e) {
        console.error("Failed to create suggested project:", e);
      }
    }
  }
  // Tag activity immediately with progress, then reload
  await runTaggerWithProgress();
  await loadProjects();
}

async function handleSmartSuggest() {
  const btn = document.getElementById("projects-smart-suggest-btn");
  if (!btn) return;

  // Show loading state
  const originalText = btn.textContent;
  btn.textContent = "Analyzing...";
  btn.disabled = true;

  let suggestions;
  try {
    suggestions = await invoke("smart_suggest_projects", { days: 7 });
  } catch (e) {
    btn.textContent = originalText;
    btn.disabled = false;
    const msg = typeof e === "string" ? e : e.message || String(e);
    if (msg.includes("API key")) {
      alert("No AI provider configured. Go to Settings to set your API key and base URL.");
    } else {
      alert("Smart Suggest failed: " + msg);
    }
    return;
  }

  btn.textContent = originalText;
  btn.disabled = false;

  if (!suggestions || suggestions.length === 0) {
    alert("No suggestions from AI. Try using more apps or increasing the analysis window.");
    return;
  }

  showSuggestionCards(suggestions);
}

function showSuggestionCards(suggestions) {
  if (!projectsBody) return;

  // Clear existing content and show suggestion cards
  const existing = projectsBody.querySelectorAll(".project-card, .empty-state, .suggestion-container");
  existing.forEach((el) => el.remove());

  const container = document.createElement("div");
  container.className = "suggestion-container";

  // Header with Accept All / Skip All
  const header = document.createElement("div");
  header.className = "suggestion-header";

  const title = document.createElement("span");
  title.className = "suggestion-header-title";
  title.textContent = `${suggestions.length} suggestion${suggestions.length !== 1 ? "s" : ""}`;

  const btnRow = document.createElement("div");
  btnRow.className = "suggestion-header-actions";

  const acceptAllBtn = document.createElement("button");
  acceptAllBtn.className = "suggestion-btn accept";
  acceptAllBtn.textContent = "Accept All";
  acceptAllBtn.addEventListener("click", async () => {
    await acceptSuggestions(suggestions, container);
  });

  const skipAllBtn = document.createElement("button");
  skipAllBtn.className = "suggestion-btn skip";
  skipAllBtn.textContent = "Skip All";
  skipAllBtn.addEventListener("click", () => {
    container.remove();
    loadProjects();
  });

  btnRow.appendChild(acceptAllBtn);
  btnRow.appendChild(skipAllBtn);
  header.appendChild(title);
  header.appendChild(btnRow);
  container.appendChild(header);

  // Render each suggestion card
  for (const suggestion of suggestions) {
    container.appendChild(renderSuggestionCard(suggestion, container, suggestions));
  }

  projectsBody.prepend(container);
}

function renderSuggestionCard(suggestion, container, allSuggestions) {
  const card = document.createElement("div");
  card.className = "suggestion-card";
  card.dataset.name = suggestion.name;

  const cardHeader = document.createElement("div");
  cardHeader.className = "suggestion-card-header";

  const name = document.createElement("span");
  name.className = "suggestion-card-name";
  name.textContent = suggestion.name;

  cardHeader.appendChild(name);

  // Rules list
  const rulesList = document.createElement("div");
  rulesList.className = "suggestion-card-rules";
  for (const rule of suggestion.rules) {
    const ruleEl = document.createElement("code");
    ruleEl.className = "suggestion-card-rule";
    ruleEl.textContent = rule;
    rulesList.appendChild(ruleEl);
  }

  // Action buttons
  const actions = document.createElement("div");
  actions.className = "suggestion-card-actions";

  const acceptBtn = document.createElement("button");
  acceptBtn.className = "suggestion-btn accept";
  acceptBtn.textContent = "Accept";
  acceptBtn.addEventListener("click", async () => {
    await acceptSuggestions([suggestion], null);
    card.remove();
    // If no more cards, remove container and reload
    if (container.querySelectorAll(".suggestion-card").length === 0) {
      container.remove();
      await loadProjects();
    }
  });

  const skipBtn = document.createElement("button");
  skipBtn.className = "suggestion-btn skip";
  skipBtn.textContent = "Skip";
  skipBtn.addEventListener("click", () => {
    card.remove();
    if (container.querySelectorAll(".suggestion-card").length === 0) {
      container.remove();
      loadProjects();
    }
  });

  actions.appendChild(acceptBtn);
  actions.appendChild(skipBtn);

  card.appendChild(cardHeader);
  card.appendChild(rulesList);
  card.appendChild(actions);

  return card;
}

async function acceptSuggestions(suggestions, container) {
  for (const suggestion of suggestions) {
    try {
      const project = await invoke("create_project", {
        name: suggestion.name,
        description: "AI-suggested project",
        color: getRandomColor(),
      });
      for (const expr of suggestion.rules) {
        try {
          await invoke("add_project_rule", {
            projectId: project.id,
            expression: expr,
            priority: 0,
          });
        } catch (err) {
          console.error("Failed to add rule:", err);
        }
      }
    } catch (e) {
      console.error("Failed to create suggested project:", e);
    }
  }
  if (container) {
    container.remove();
  }
  await runTaggerWithProgress();
  await loadProjects();
}

function showTaggerStatus(text, type) {
  if (!projectsUntagged) return;
  projectsUntagged.style.display = "block";
  projectsUntagged.textContent = text;
  projectsUntagged.className = "projects-untagged" + (type ? " tagger-" + type : "");
}

async function runTaggerWithProgress() {
  showTaggerStatus("Tagging: loading rules...", "active");
  await runTagger((status) => {
    switch (status.phase) {
      case "compiling":
        showTaggerStatus(`Tagging: ${status.ruleCount} rules compiled`, "active");
        break;
      case "batch":
        showTaggerStatus(
          `Tagging: batch ${status.batchNum} \u2014 ${status.tagged} tagged, ${status.skipped} skipped (${status.totalTagged} total)`,
          "active"
        );
        break;
      case "done":
        if (status.totalTagged > 0) {
          showTaggerStatus(`Tagged ${status.totalTagged} activities (${status.totalSkipped} unmatched)`, "done");
        } else {
          showTaggerStatus("No new activities matched any rules", "done");
        }
        break;
      case "idle":
        showTaggerStatus("No rules configured", "done");
        break;
      case "error":
        showTaggerStatus(`Tagger error: ${status.message}`, "error");
        break;
    }
  });
}

async function handleDeleteProject(id) {
  if (!confirm("Delete this project? Activity tags will be cleared.")) return;
  try {
    await invoke("delete_project", { id });
    await loadProjects();
  } catch (e) {
    console.error("Failed to delete project:", e);
  }
}

// --- Helpers ---

const PROJECT_COLORS = [
  "#93bbfc", "#86efac", "#fca5a5", "#fcd34d", "#c4b5fd",
  "#f9a8d4", "#67e8f9", "#fdba74", "#a5b4fc", "#6ee7b7",
];

function getRandomColor() {
  return PROJECT_COLORS[Math.floor(Math.random() * PROJECT_COLORS.length)];
}

function formatNumber(n) {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + "M";
  if (n >= 1_000) return (n / 1_000).toFixed(1) + "K";
  return n.toString();
}

function formatDuration(secs) {
  if (secs < 60) return secs + "s";
  if (secs < 3600) return Math.floor(secs / 60) + "m";
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  return h + "h " + m + "m";
}
