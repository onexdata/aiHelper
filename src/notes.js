import { invoke } from "@tauri-apps/api/core";
import { formatRelativeTime } from "./history.js";

let currentNoteId = null;

export function initNotes() {
  const input = document.getElementById("note-title-input");
  const saveBtn = document.getElementById("note-save-btn");
  const backBtn = document.getElementById("note-back-btn");

  input.addEventListener("keydown", async (e) => {
    if (e.key === "Enter") {
      e.preventDefault();
      const title = input.value.trim();
      if (!title) return;
      input.value = "";
      try {
        const note = await invoke("create_note", { title, content: "" });
        openEditor(note);
      } catch (err) {
        console.error("Failed to create note:", err);
      }
    }
  });

  saveBtn.addEventListener("click", saveCurrentNote);
  backBtn.addEventListener("click", closeEditor);
}

export async function loadNoteList() {
  const list = document.getElementById("note-list");
  try {
    const notes = await invoke("list_notes");
    if (notes.length === 0) {
      list.innerHTML = '<div class="empty-state">No notes yet</div>';
      return;
    }
    list.innerHTML = "";
    for (const note of notes) {
      list.appendChild(createNoteItem(note));
    }
  } catch (err) {
    console.error("Failed to load notes:", err);
    list.innerHTML = '<div class="empty-state">Failed to load notes</div>';
  }
}

function createNoteItem(note) {
  const item = document.createElement("div");
  item.className = "note-item";
  item.addEventListener("click", () => openEditor(note));

  const title = document.createElement("div");
  title.className = "note-item-title";
  title.textContent = note.title || "Untitled";

  const preview = document.createElement("div");
  preview.className = "note-item-preview";
  const previewText = (note.content || "").replace(/\n/g, " ").slice(0, 80);
  preview.textContent = previewText || "No content";

  const meta = document.createElement("div");
  meta.className = "note-item-meta";
  meta.textContent = formatRelativeTime(note.updated_at);

  const deleteBtn = document.createElement("button");
  deleteBtn.className = "note-delete-btn";
  deleteBtn.innerHTML = "&times;";
  deleteBtn.title = "Delete note";
  deleteBtn.addEventListener("click", async (e) => {
    e.stopPropagation();
    try {
      await invoke("delete_note", { id: note.id });
      loadNoteList();
    } catch (err) {
      console.error("Failed to delete note:", err);
    }
  });

  const row = document.createElement("div");
  row.className = "note-item-row";
  row.appendChild(title);
  row.appendChild(deleteBtn);

  item.appendChild(row);
  item.appendChild(preview);
  item.appendChild(meta);
  return item;
}

function openEditor(note) {
  currentNoteId = note.id;
  const editor = document.getElementById("note-editor");
  const list = document.getElementById("note-list");
  const inputRow = document.getElementById("note-input-row");
  const titleInput = document.getElementById("note-edit-title");
  const contentArea = document.getElementById("note-edit-content");

  titleInput.value = note.title;
  contentArea.value = note.content;

  inputRow.style.display = "none";
  list.style.display = "none";
  editor.style.display = "flex";

  contentArea.focus();
}

function closeEditor() {
  currentNoteId = null;
  const editor = document.getElementById("note-editor");
  const list = document.getElementById("note-list");
  const inputRow = document.getElementById("note-input-row");

  editor.style.display = "none";
  inputRow.style.display = "flex";
  list.style.display = "flex";

  loadNoteList();
}

async function saveCurrentNote() {
  if (currentNoteId === null) return;
  const title = document.getElementById("note-edit-title").value.trim();
  const content = document.getElementById("note-edit-content").value;
  try {
    await invoke("update_note", { id: currentNoteId, title, content });
    closeEditor();
  } catch (err) {
    console.error("Failed to save note:", err);
  }
}
