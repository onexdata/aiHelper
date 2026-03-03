import { invoke } from "@tauri-apps/api/core";

let listEl;
let callbacks = {};

export function initHistory({ onSelectConversation, onNewChatClick }) {
  callbacks = { onSelectConversation, onNewChatClick };
  listEl = document.getElementById("conversation-list");
  document.getElementById("new-chat-btn").addEventListener("click", () => {
    callbacks.onNewChatClick();
  });
}

export async function loadConversationList() {
  try {
    const conversations = await invoke("list_conversations");
    renderList(conversations);
  } catch (e) {
    console.error("Failed to load conversations:", e);
    listEl.innerHTML = "";
  }
}

function renderList(conversations) {
  listEl.innerHTML = "";

  if (conversations.length === 0) {
    const empty = document.createElement("div");
    empty.className = "empty-state";
    empty.textContent = "No conversations yet";
    listEl.appendChild(empty);
    return;
  }

  for (const conv of conversations) {
    const item = document.createElement("div");
    item.className = "conversation-item";

    const info = document.createElement("div");
    info.className = "conversation-item-info";

    const title = document.createElement("div");
    title.className = "conversation-item-title";
    title.textContent = conv.title;

    const time = document.createElement("div");
    time.className = "conversation-item-time";
    time.textContent = formatRelativeTime(conv.updated_at);

    info.appendChild(title);
    info.appendChild(time);

    const deleteBtn = document.createElement("button");
    deleteBtn.className = "conversation-item-delete";
    deleteBtn.title = "Delete";
    deleteBtn.textContent = "\u00d7";
    deleteBtn.addEventListener("click", async (e) => {
      e.stopPropagation();
      try {
        await invoke("delete_conversation", { conversationId: conv.id });
        await loadConversationList();
      } catch (err) {
        console.error("Failed to delete conversation:", err);
      }
    });

    item.appendChild(info);
    item.appendChild(deleteBtn);
    item.addEventListener("click", () => {
      callbacks.onSelectConversation(conv.id);
    });

    listEl.appendChild(item);
  }
}

export function formatRelativeTime(dateStr) {
  const date = new Date(dateStr + "Z"); // SQLite stores UTC without Z suffix
  const now = new Date();
  const diffMs = now - date;
  const diffMin = Math.floor(diffMs / 60000);

  if (diffMin < 1) return "just now";
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `${diffHr}h ago`;
  const diffDay = Math.floor(diffHr / 24);
  if (diffDay < 30) return `${diffDay}d ago`;
  return date.toLocaleDateString();
}
