import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { marked } from "marked";

marked.setOptions({ breaks: true, gfm: true });

// --- State ---
let tree = new Map();       // id -> node { id, parentId, role, content, model, children: [id], activeChildId }
let rootChildren = [];      // ids of root-level messages (parentId === null)
let conversationId = null;
let conversationData = null;
let isStreaming = false;

// DOM refs
let messagesEl, inputEl, sendBtn, titleEl;

// Temp streaming state
let currentTempEl = null;
let currentTempText = "";
let streamingParentId = null;
let currentToolsUsed = null;

// --- Init ---

export function initChat() {
  messagesEl = document.getElementById("chat-messages");
  inputEl = document.getElementById("chat-input");
  sendBtn = document.getElementById("send-btn");
  titleEl = document.getElementById("chat-title");

  sendBtn.addEventListener("click", handleSend);
  inputEl.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  });

  listen("chat-tools-used", (event) => {
    const tools = event.payload?.tools;
    if (tools && tools.length > 0) {
      currentToolsUsed = tools;
      // Insert tool indicator above the temp streaming bubble
      if (currentTempEl && currentTempEl.parentElement) {
        const indicator = buildToolIndicator(tools);
        messagesEl.insertBefore(indicator, currentTempEl.parentElement);
        messagesEl.scrollTop = messagesEl.scrollHeight;
      }
    }
  });

  listen("chat-chunk", (event) => {
    if (currentTempEl) {
      currentTempText += event.payload;
      currentTempEl.innerHTML = marked.parse(currentTempText);
      messagesEl.scrollTop = messagesEl.scrollHeight;
    }
  });

  listen("chat-done", (event) => {
    const payload = event.payload;
    if (payload && payload.id != null) {
      // Add assistant node to tree using the tracked parent
      const parentId = streamingParentId;
      addNodeToTree({
        id: payload.id,
        parentId,
        role: "assistant",
        content: payload.content,
        model: payload.model,
        activeChildId: null,
      });
      // Attach tool indicators to the tree node
      if (currentToolsUsed) {
        const node = tree.get(payload.id);
        if (node) node.toolsUsed = currentToolsUsed;
      }
      // Update parent's activeChildId
      if (parentId != null) {
        const parent = tree.get(parentId);
        if (parent) parent.activeChildId = payload.id;
      }
    }
    streamingParentId = null;
    currentToolsUsed = null;
    removeTempBubble();
    renderActiveThread();
    setStreaming(false);
  });

  listen("chat-error", (event) => {
    if (currentTempEl) {
      currentTempEl.textContent = event.payload;
      currentTempEl.classList.add("error");
    }
    currentTempEl = null;
    currentTempText = "";
    streamingParentId = null;
    currentToolsUsed = null;
    setStreaming(false);
  });
}

export function getIsStreaming() {
  return isStreaming;
}

// --- Open / New ---

export async function openConversation(id) {
  try {
    const resp = await invoke("load_conversation", { conversationId: id });
    conversationId = id;
    conversationData = resp.conversation;
    buildTree(resp.messages);
    titleEl.textContent = conversationData.title;
    renderActiveThread();
    inputEl.focus();
  } catch (e) {
    console.error("Failed to load conversation:", e);
  }
}

export async function startNewConversation() {
  try {
    const conv = await invoke("create_conversation", { title: null });
    conversationId = conv.id;
    conversationData = conv;
    tree.clear();
    rootChildren = [];
    titleEl.textContent = "New conversation";
    messagesEl.innerHTML = "";
    inputEl.focus();
  } catch (e) {
    console.error("Failed to create conversation:", e);
  }
}

// --- Tree operations ---

function buildTree(messages) {
  tree.clear();
  rootChildren = [];

  // First pass: create all nodes
  for (const m of messages) {
    tree.set(m.id, {
      id: m.id,
      parentId: m.parent_id,
      role: m.role,
      content: m.content,
      model: m.model,
      activeChildId: m.active_child_id,
      children: [],
    });
  }

  // Second pass: populate children arrays
  for (const node of tree.values()) {
    if (node.parentId == null) {
      rootChildren.push(node.id);
    } else {
      const parent = tree.get(node.parentId);
      if (parent) parent.children.push(node.id);
    }
  }
}

function addNodeToTree(data) {
  const node = {
    id: data.id,
    parentId: data.parentId,
    role: data.role,
    content: data.content,
    model: data.model,
    activeChildId: data.activeChildId,
    children: [],
  };
  tree.set(node.id, node);

  if (node.parentId == null) {
    rootChildren.push(node.id);
  } else {
    const parent = tree.get(node.parentId);
    if (parent) parent.children.push(node.id);
  }
}

function getActiveThread() {
  const thread = [];
  if (rootChildren.length === 0) return thread;

  // Find active root
  let currentId = conversationData?.active_root_message_id;
  if (currentId == null || !tree.has(currentId)) {
    currentId = rootChildren[rootChildren.length - 1];
  }

  while (currentId != null) {
    const node = tree.get(currentId);
    if (!node) break;
    thread.push(node);
    currentId = node.activeChildId;
  }

  return thread;
}

function getSiblings(node) {
  if (node.parentId == null) {
    return rootChildren.map((id) => tree.get(id)).filter(Boolean);
  }
  const parent = tree.get(node.parentId);
  if (!parent) return [node];
  return parent.children.map((id) => tree.get(id)).filter(Boolean);
}

// --- Rendering ---

function renderActiveThread() {
  messagesEl.innerHTML = "";
  const thread = getActiveThread();

  if (thread.length === 0) {
    const empty = document.createElement("div");
    empty.className = "empty-state";
    empty.textContent = "Start a conversation...";
    messagesEl.appendChild(empty);
    return;
  }

  for (const node of thread) {
    messagesEl.appendChild(renderMessageNode(node));
  }
  messagesEl.scrollTop = messagesEl.scrollHeight;
}

function renderMessageNode(node) {
  const wrapper = document.createElement("div");
  wrapper.className = `msg-wrapper ${node.role}`;
  wrapper.dataset.msgId = node.id;

  // Tool indicator (before the bubble, for assistant messages)
  if (node.role === "assistant" && node.toolsUsed && node.toolsUsed.length > 0) {
    wrapper.appendChild(buildToolIndicator(node.toolsUsed));
  }

  // Bubble
  const bubble = document.createElement("div");
  bubble.className = `msg ${node.role}`;
  if (node.role === "assistant") {
    bubble.innerHTML = marked.parse(node.content);
  } else {
    bubble.textContent = node.content;
  }
  wrapper.appendChild(bubble);

  // Footer row: action button + branch nav, always visible on one line
  const footer = document.createElement("div");
  footer.className = "msg-footer";

  if (node.role === "user") {
    const editBtn = document.createElement("button");
    editBtn.className = "msg-action-btn";
    editBtn.textContent = "\u270F";
    editBtn.title = "Edit";
    editBtn.addEventListener("click", () => handleEdit(node.id));
    footer.appendChild(editBtn);
  }

  if (node.role === "assistant") {
    const rerollBtn = document.createElement("button");
    rerollBtn.className = "msg-action-btn";
    rerollBtn.textContent = "\u21BB";
    rerollBtn.title = "Regenerate";
    rerollBtn.addEventListener("click", () => handleReroll(node.id));
    footer.appendChild(rerollBtn);
  }

  const siblings = getSiblings(node);
  if (siblings.length > 1) {
    const idx = siblings.findIndex((s) => s.id === node.id);

    const prevBtn = document.createElement("button");
    prevBtn.className = "branch-nav-btn";
    prevBtn.textContent = "\u25C0";
    prevBtn.disabled = idx <= 0;
    prevBtn.addEventListener("click", () => navigateBranch(node, siblings, idx - 1));

    const label = document.createElement("span");
    label.className = "branch-nav-label";
    label.textContent = `${idx + 1}/${siblings.length}`;

    const nextBtn = document.createElement("button");
    nextBtn.className = "branch-nav-btn";
    nextBtn.textContent = "\u25B6";
    nextBtn.disabled = idx >= siblings.length - 1;
    nextBtn.addEventListener("click", () => navigateBranch(node, siblings, idx + 1));

    footer.appendChild(prevBtn);
    footer.appendChild(label);
    footer.appendChild(nextBtn);
  }

  wrapper.appendChild(footer);

  return wrapper;
}

// --- Send ---

async function handleSend() {
  const text = inputEl.value.trim();
  if (!text || isStreaming) return;

  inputEl.value = "";

  // Determine parent: last message in active thread, or null
  const thread = getActiveThread();
  const parentId = thread.length > 0 ? thread[thread.length - 1].id : null;

  try {
    const savedMsg = await invoke("save_user_message", {
      conversationId,
      parentId,
      content: text,
    });

    // Add to local tree
    addNodeToTree({
      id: savedMsg.id,
      parentId: savedMsg.parent_id,
      role: "user",
      content: savedMsg.content,
      model: null,
      activeChildId: null,
    });

    // Update parent's activeChildId locally
    if (parentId != null) {
      const parent = tree.get(parentId);
      if (parent) parent.activeChildId = savedMsg.id;
    } else {
      // Update conversation active root
      if (conversationData) conversationData.active_root_message_id = savedMsg.id;
    }

    // Update title if it was auto-set
    if (conversationData && conversationData.title === "New conversation") {
      conversationData.title = text.substring(0, 50);
      titleEl.textContent = conversationData.title;
    }

    renderActiveThread();
    startStreaming(savedMsg.id);
  } catch (e) {
    console.error("Failed to save user message:", e);
  }
}

function startStreaming(parentMessageId) {
  setStreaming(true);
  streamingParentId = parentMessageId;

  // Create temp assistant bubble
  currentTempText = "";
  currentTempEl = document.createElement("div");
  currentTempEl.className = "msg assistant";
  currentTempEl.textContent = "";

  const wrapper = document.createElement("div");
  wrapper.className = "msg-wrapper assistant";
  wrapper.appendChild(currentTempEl);
  messagesEl.appendChild(wrapper);
  messagesEl.scrollTop = messagesEl.scrollHeight;

  // Build messages array for the API: walk from root to parentMessageId
  const apiMessages = buildMessagesUpTo(parentMessageId);

  invoke("send_chat_message", {
    conversationId,
    parentMessageId,
    messages: apiMessages,
    model: null,
  }).catch((e) => {
    if (currentTempEl) {
      currentTempEl.textContent = e;
      currentTempEl.classList.add("error");
    }
    currentTempEl = null;
    currentTempText = "";
    setStreaming(false);
  });
}

function buildMessagesUpTo(targetId) {
  // Walk up from target to root, then reverse
  const chain = [];
  let currentId = targetId;
  while (currentId != null) {
    const node = tree.get(currentId);
    if (!node) break;
    chain.push({ role: node.role, content: node.content });
    currentId = node.parentId;
  }
  chain.reverse();
  return chain;
}

function removeTempBubble() {
  if (currentTempEl && currentTempEl.parentElement) {
    currentTempEl.parentElement.remove();
  }
  currentTempEl = null;
  currentTempText = "";
}

// --- Edit ---

function handleEdit(messageId) {
  if (isStreaming) return;
  const node = tree.get(messageId);
  if (!node) return;

  const wrapper = messagesEl.querySelector(`[data-msg-id="${messageId}"]`);
  if (!wrapper) return;

  wrapper.innerHTML = "";

  const textarea = document.createElement("textarea");
  textarea.className = "msg-edit-textarea";
  textarea.value = node.content;
  textarea.rows = 3;
  wrapper.appendChild(textarea);

  const btns = document.createElement("div");
  btns.className = "msg-edit-actions";

  const cancelBtn = document.createElement("button");
  cancelBtn.textContent = "Cancel";
  cancelBtn.addEventListener("click", () => renderActiveThread());

  const saveBtn = document.createElement("button");
  saveBtn.className = "save-btn";
  saveBtn.textContent = "Save & Send";
  saveBtn.addEventListener("click", async () => {
    const newText = textarea.value.trim();
    if (!newText) return;

    try {
      // Create a sibling of the original message (same parent)
      const savedMsg = await invoke("save_user_message", {
        conversationId,
        parentId: node.parentId,
        content: newText,
      });

      addNodeToTree({
        id: savedMsg.id,
        parentId: savedMsg.parent_id,
        role: "user",
        content: savedMsg.content,
        model: null,
        activeChildId: null,
      });

      // Update parent's activeChildId or conversation root
      if (node.parentId != null) {
        const parent = tree.get(node.parentId);
        if (parent) parent.activeChildId = savedMsg.id;
      } else {
        if (conversationData) conversationData.active_root_message_id = savedMsg.id;
      }

      renderActiveThread();
      startStreaming(savedMsg.id);
    } catch (e) {
      console.error("Failed to save edited message:", e);
    }
  });

  btns.appendChild(cancelBtn);
  btns.appendChild(saveBtn);
  wrapper.appendChild(btns);

  textarea.focus();
  textarea.setSelectionRange(textarea.value.length, textarea.value.length);
}

// --- Reroll ---

function handleReroll(messageId) {
  if (isStreaming) return;
  const node = tree.get(messageId);
  if (!node || node.role !== "assistant") return;

  // The parent user message
  const userParentId = node.parentId;
  if (userParentId == null) return;

  startStreaming(userParentId);
}

// --- Branch navigation ---

async function navigateBranch(node, siblings, newIdx) {
  if (isStreaming) return;
  const target = siblings[newIdx];
  if (!target) return;

  try {
    if (node.parentId == null) {
      await invoke("set_conversation_active_root", {
        conversationId,
        messageId: target.id,
      });
      if (conversationData) conversationData.active_root_message_id = target.id;
    } else {
      await invoke("set_active_child", {
        messageId: node.parentId,
        childId: target.id,
      });
      const parent = tree.get(node.parentId);
      if (parent) parent.activeChildId = target.id;
    }
    renderActiveThread();
  } catch (e) {
    console.error("Failed to navigate branch:", e);
  }
}

// --- Helpers ---

function setStreaming(value) {
  isStreaming = value;
  sendBtn.disabled = value;
  inputEl.disabled = value;
  if (!value) inputEl.focus();
}

function escapeHtml(text) {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}

function buildToolIndicator(tools) {
  const container = document.createElement("div");
  container.className = "tool-indicator";

  const header = document.createElement("button");
  header.className = "tool-indicator-header";
  header.textContent = `Used ${tools.length} tool${tools.length !== 1 ? "s" : ""}`;
  header.addEventListener("click", () => {
    container.classList.toggle("expanded");
  });
  container.appendChild(header);

  const details = document.createElement("div");
  details.className = "tool-indicator-details";

  for (const t of tools) {
    const row = document.createElement("div");
    row.className = "tool-indicator-row";

    const name = document.createElement("span");
    name.className = "tool-indicator-name";
    name.textContent = t.tool_name;

    const summary = document.createElement("span");
    summary.className = "tool-indicator-summary";
    summary.textContent = t.summary;

    row.appendChild(name);
    row.appendChild(summary);
    details.appendChild(row);
  }

  container.appendChild(details);
  return container;
}
