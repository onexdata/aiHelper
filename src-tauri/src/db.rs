use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

const CURRENT_VERSION: i32 = 11;

pub struct Database {
    conn: Connection,
}

#[derive(Debug, Serialize)]
pub struct Event {
    pub id: i64,
    pub event_type: String,
    pub payload: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Conversation {
    pub id: i64,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub active_root_message_id: Option<i64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ChatMessageRow {
    pub id: i64,
    pub conversation_id: i64,
    pub parent_id: Option<i64>,
    pub role: String,
    pub content: String,
    pub model: Option<String>,
    pub created_at: String,
    pub active_child_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: i64,
    pub content: String,
    pub completed: bool,
    pub archived: bool,
    pub window_title: String,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub color: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProjectRule {
    pub id: i64,
    pub project_id: i64,
    pub expression: String,
    pub priority: i32,
    pub created_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct UntaggedActivity {
    pub table: String,
    pub id: i64,
    pub app_name: String,
    pub window_title: String,
}

#[derive(Debug, Deserialize)]
pub struct ActivityTag {
    pub table: String,
    pub id: i64,
    pub project_id: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectSuggestion {
    pub name: String,
    pub rules: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ActivitySummaryRow {
    pub app_name: String,
    pub window_title: String,
    pub total_duration_secs: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProjectSummary {
    pub project_id: Option<i64>,
    pub name: String,
    pub color: String,
    pub keystroke_count: i64,
    pub active_secs: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct UntaggedSummaryRow {
    pub app_name: String,
    pub keystroke_rows: i64,
    pub window_rows: i64,
    pub total: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Note {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProjectActivity {
    pub id: i64,
    pub app_name: String,
    pub window_title: String,
    pub duration_secs: i64,
    pub started_at: String,
}

impl Database {
    pub fn initialize(db_path: &str) -> Result<Self, String> {
        if let Some(parent) = Path::new(db_path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create db directory: {e}"))?;
        }

        let conn = Connection::open(db_path)
            .map_err(|e| format!("Failed to open database: {e}"))?;

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| format!("Failed to set pragmas: {e}"))?;

        let mut db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&mut self) -> Result<(), String> {
        self.conn
            .execute(
                "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL)",
                [],
            )
            .map_err(|e| format!("Failed to create schema_version table: {e}"))?;

        let version: i32 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to read schema version: {e}"))?;

        if version < 1 {
            self.migrate_v1()?;
        }

        if version < 2 {
            self.migrate_v2()?;
        }

        if version < 3 {
            self.migrate_v3()?;
        }

        if version < 4 {
            self.migrate_v4()?;
        }

        if version < 5 {
            self.migrate_v5()?;
        }

        if version < 6 {
            self.migrate_v6()?;
        }

        if version < 7 {
            self.migrate_v7()?;
        }

        if version < 9 {
            self.migrate_v8()?;
        }

        if version < 10 {
            self.migrate_v9()?;
        }

        if version < 11 {
            self.migrate_v10()?;
        }

        if version < CURRENT_VERSION {
            self.conn
                .execute(
                    "INSERT INTO schema_version (version) VALUES (?1)",
                    params![CURRENT_VERSION],
                )
                .map_err(|e| format!("Failed to update schema version: {e}"))?;
        }

        Ok(())
    }

    fn migrate_v1(&mut self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS events (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    event_type TEXT NOT NULL,
                    payload TEXT NOT NULL DEFAULT '',
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                );
                CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
                CREATE INDEX IF NOT EXISTS idx_events_created ON events(created_at);",
            )
            .map_err(|e| format!("Migration v1 failed: {e}"))?;
        Ok(())
    }

    fn migrate_v4(&mut self) -> Result<(), String> {
        self.conn
            .execute_batch("ALTER TABLE tasks ADD COLUMN completed_at TEXT;")
            .map_err(|e| format!("Migration v4 failed: {e}"))?;
        Ok(())
    }

    fn migrate_v3(&mut self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS tasks (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    content TEXT NOT NULL,
                    completed INTEGER NOT NULL DEFAULT 0,
                    archived INTEGER NOT NULL DEFAULT 0,
                    window_title TEXT NOT NULL DEFAULT '',
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                );
                CREATE INDEX IF NOT EXISTS idx_tasks_archived ON tasks(archived);",
            )
            .map_err(|e| format!("Migration v3 failed: {e}"))?;
        Ok(())
    }

    fn migrate_v2(&mut self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS conversations (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    title TEXT NOT NULL DEFAULT 'New conversation',
                    created_at TEXT NOT NULL DEFAULT (datetime('now')),
                    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                    active_root_message_id INTEGER
                );

                CREATE TABLE IF NOT EXISTS chat_messages (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
                    parent_id INTEGER REFERENCES chat_messages(id) ON DELETE CASCADE,
                    role TEXT NOT NULL,
                    content TEXT NOT NULL DEFAULT '',
                    model TEXT,
                    created_at TEXT NOT NULL DEFAULT (datetime('now')),
                    active_child_id INTEGER REFERENCES chat_messages(id)
                );

                CREATE INDEX IF NOT EXISTS idx_chat_messages_conversation ON chat_messages(conversation_id);
                CREATE INDEX IF NOT EXISTS idx_chat_messages_parent ON chat_messages(parent_id);",
            )
            .map_err(|e| format!("Migration v2 failed: {e}"))?;
        Ok(())
    }

    // --- Conversation CRUD ---

    pub fn create_conversation(&self, title: Option<&str>) -> Result<Conversation, String> {
        let t = title.unwrap_or("New conversation");
        self.conn
            .execute(
                "INSERT INTO conversations (title) VALUES (?1)",
                params![t],
            )
            .map_err(|e| format!("Failed to create conversation: {e}"))?;
        let id = self.conn.last_insert_rowid();
        self.get_conversation(id)
    }

    pub fn list_conversations(&self) -> Result<Vec<Conversation>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, title, created_at, updated_at, active_root_message_id FROM conversations ORDER BY updated_at DESC")
            .map_err(|e| format!("Failed to prepare list_conversations: {e}"))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Conversation {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                    active_root_message_id: row.get(4)?,
                })
            })
            .map_err(|e| format!("Failed to query conversations: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read conversation row: {e}"))?;

        Ok(rows)
    }

    pub fn get_conversation(&self, id: i64) -> Result<Conversation, String> {
        self.conn
            .query_row(
                "SELECT id, title, created_at, updated_at, active_root_message_id FROM conversations WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Conversation {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        created_at: row.get(2)?,
                        updated_at: row.get(3)?,
                        active_root_message_id: row.get(4)?,
                    })
                },
            )
            .map_err(|e| format!("Conversation not found: {e}"))
    }

    pub fn delete_conversation(&self, id: i64) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM conversations WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to delete conversation: {e}"))?;
        Ok(())
    }

    pub fn update_conversation_title(&self, id: i64, title: &str) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE conversations SET title = ?1 WHERE id = ?2",
                params![title, id],
            )
            .map_err(|e| format!("Failed to update conversation title: {e}"))?;
        Ok(())
    }

    pub fn set_conversation_active_root(&self, conversation_id: i64, message_id: i64) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE conversations SET active_root_message_id = ?1 WHERE id = ?2",
                params![message_id, conversation_id],
            )
            .map_err(|e| format!("Failed to set active root: {e}"))?;
        Ok(())
    }

    pub fn touch_conversation(&self, id: i64) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE conversations SET updated_at = datetime('now') WHERE id = ?1",
                params![id],
            )
            .map_err(|e| format!("Failed to touch conversation: {e}"))?;
        Ok(())
    }

    // --- Chat message CRUD ---

    pub fn insert_chat_message(
        &self,
        conversation_id: i64,
        parent_id: Option<i64>,
        role: &str,
        content: &str,
        model: Option<&str>,
    ) -> Result<ChatMessageRow, String> {
        self.conn
            .execute(
                "INSERT INTO chat_messages (conversation_id, parent_id, role, content, model) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![conversation_id, parent_id, role, content, model],
            )
            .map_err(|e| format!("Failed to insert chat message: {e}"))?;
        let id = self.conn.last_insert_rowid();
        self.get_chat_message(id)
    }

    pub fn get_chat_message(&self, id: i64) -> Result<ChatMessageRow, String> {
        self.conn
            .query_row(
                "SELECT id, conversation_id, parent_id, role, content, model, created_at, active_child_id FROM chat_messages WHERE id = ?1",
                params![id],
                |row| {
                    Ok(ChatMessageRow {
                        id: row.get(0)?,
                        conversation_id: row.get(1)?,
                        parent_id: row.get(2)?,
                        role: row.get(3)?,
                        content: row.get(4)?,
                        model: row.get(5)?,
                        created_at: row.get(6)?,
                        active_child_id: row.get(7)?,
                    })
                },
            )
            .map_err(|e| format!("Chat message not found: {e}"))
    }

    pub fn get_conversation_messages(&self, conversation_id: i64) -> Result<Vec<ChatMessageRow>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, conversation_id, parent_id, role, content, model, created_at, active_child_id FROM chat_messages WHERE conversation_id = ?1 ORDER BY id ASC")
            .map_err(|e| format!("Failed to prepare get_conversation_messages: {e}"))?;

        let rows = stmt
            .query_map(params![conversation_id], |row| {
                Ok(ChatMessageRow {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    parent_id: row.get(2)?,
                    role: row.get(3)?,
                    content: row.get(4)?,
                    model: row.get(5)?,
                    created_at: row.get(6)?,
                    active_child_id: row.get(7)?,
                })
            })
            .map_err(|e| format!("Failed to query messages: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read message row: {e}"))?;

        Ok(rows)
    }

    pub fn set_active_child(&self, message_id: i64, child_id: i64) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE chat_messages SET active_child_id = ?1 WHERE id = ?2",
                params![child_id, message_id],
            )
            .map_err(|e| format!("Failed to set active child: {e}"))?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn update_message_content(&self, message_id: i64, content: &str) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE chat_messages SET content = ?1 WHERE id = ?2",
                params![content, message_id],
            )
            .map_err(|e| format!("Failed to update message content: {e}"))?;
        Ok(())
    }

    pub fn insert_event(&self, event_type: &str, payload: &str) -> Result<i64, String> {
        self.conn
            .execute(
                "INSERT INTO events (event_type, payload) VALUES (?1, ?2)",
                params![event_type, payload],
            )
            .map_err(|e| format!("Failed to insert event: {e}"))?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_recent_events(&self, limit: i64) -> Result<Vec<Event>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, event_type, payload, created_at FROM events ORDER BY id DESC LIMIT ?1")
            .map_err(|e| format!("Failed to prepare query: {e}"))?;

        let events = stmt
            .query_map(params![limit], |row| {
                Ok(Event {
                    id: row.get(0)?,
                    event_type: row.get(1)?,
                    payload: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .map_err(|e| format!("Failed to query events: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read event row: {e}"))?;

        Ok(events)
    }

    // --- Task CRUD ---

    pub fn create_task(&self, content: &str, window_title: &str) -> Result<Task, String> {
        self.conn
            .execute(
                "INSERT INTO tasks (content, window_title) VALUES (?1, ?2)",
                params![content, window_title],
            )
            .map_err(|e| format!("Failed to create task: {e}"))?;
        let id = self.conn.last_insert_rowid();
        self.get_task(id)
    }

    pub fn get_task(&self, id: i64) -> Result<Task, String> {
        self.conn
            .query_row(
                "SELECT id, content, completed, archived, window_title, created_at, completed_at FROM tasks WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Task {
                        id: row.get(0)?,
                        content: row.get(1)?,
                        completed: row.get::<_, i32>(2)? != 0,
                        archived: row.get::<_, i32>(3)? != 0,
                        window_title: row.get(4)?,
                        created_at: row.get(5)?,
                        completed_at: row.get(6)?,
                    })
                },
            )
            .map_err(|e| format!("Task not found: {e}"))
    }

    pub fn list_tasks(&self, archived: bool) -> Result<Vec<Task>, String> {
        let archived_int: i32 = if archived { 1 } else { 0 };
        let mut stmt = self
            .conn
            .prepare("SELECT id, content, completed, archived, window_title, created_at, completed_at FROM tasks WHERE archived = ?1 ORDER BY created_at DESC")
            .map_err(|e| format!("Failed to prepare list_tasks: {e}"))?;

        let rows = stmt
            .query_map(params![archived_int], |row| {
                Ok(Task {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    completed: row.get::<_, i32>(2)? != 0,
                    archived: row.get::<_, i32>(3)? != 0,
                    window_title: row.get(4)?,
                    created_at: row.get(5)?,
                    completed_at: row.get(6)?,
                })
            })
            .map_err(|e| format!("Failed to query tasks: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read task row: {e}"))?;

        Ok(rows)
    }

    pub fn update_task_completed(&self, id: i64, completed: bool) -> Result<Task, String> {
        let completed_int: i32 = if completed { 1 } else { 0 };
        if completed {
            self.conn
                .execute(
                    "UPDATE tasks SET completed = ?1, completed_at = datetime('now') WHERE id = ?2",
                    params![completed_int, id],
                )
                .map_err(|e| format!("Failed to update task: {e}"))?;
        } else {
            // Unchecking also unarchives so the task returns to Active
            self.conn
                .execute(
                    "UPDATE tasks SET completed = 0, completed_at = NULL, archived = 0 WHERE id = ?1",
                    params![id],
                )
                .map_err(|e| format!("Failed to update task: {e}"))?;
        }
        self.get_task(id)
    }

    pub fn archive_task(&self, id: i64) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE tasks SET archived = 1 WHERE id = ?1",
                params![id],
            )
            .map_err(|e| format!("Failed to archive task: {e}"))?;
        Ok(())
    }

    pub fn delete_task(&self, id: i64) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM tasks WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to delete task: {e}"))?;
        Ok(())
    }

    // --- Migration v5: settings table ---

    fn migrate_v5(&mut self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS settings (
                    key TEXT NOT NULL,
                    value TEXT NOT NULL DEFAULT '',
                    component TEXT NOT NULL DEFAULT 'default',
                    PRIMARY KEY (key, component)
                );",
            )
            .map_err(|e| format!("Migration v5 failed: {e}"))?;
        Ok(())
    }

    // --- Settings CRUD ---

    pub fn get_setting(&self, key: &str, component: &str) -> Result<Option<String>, String> {
        match self.conn.query_row(
            "SELECT value FROM settings WHERE key = ?1 AND component = ?2",
            params![key, component],
            |row| row.get(0),
        ) {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("Failed to get setting: {e}")),
        }
    }

    pub fn get_all_settings(&self, component: &str) -> Result<Vec<(String, String)>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT key, value FROM settings WHERE component = ?1")
            .map_err(|e| format!("Failed to prepare get_all_settings: {e}"))?;

        let rows = stmt
            .query_map(params![component], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| format!("Failed to query settings: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read setting row: {e}"))?;

        Ok(rows)
    }

    pub fn set_setting(&self, key: &str, value: &str, component: &str) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO settings (key, value, component) VALUES (?1, ?2, ?3)",
                params![key, value, component],
            )
            .map_err(|e| format!("Failed to set setting: {e}"))?;
        Ok(())
    }

    pub fn seed_setting(&self, key: &str, value: &str, component: &str) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO settings (key, value, component) VALUES (?1, ?2, ?3)",
                params![key, value, component],
            )
            .map_err(|e| format!("Failed to seed setting: {e}"))?;
        Ok(())
    }

    pub fn seed_defaults_from_config(
        &self,
        hotkey: Option<&str>,
        ai_provider: Option<&str>,
        ai_api_key: Option<&str>,
        ai_base_url: Option<&str>,
        task_archive_delay_secs: Option<u32>,
    ) -> Result<(), String> {
        let c = "default";
        self.seed_setting("hotkey", hotkey.unwrap_or("Ctrl+Shift+Space"), c)?;
        self.seed_setting("ai_provider", ai_provider.unwrap_or(""), c)?;
        self.seed_setting("ai_api_key", ai_api_key.unwrap_or(""), c)?;
        self.seed_setting("ai_base_url", ai_base_url.unwrap_or("https://openrouter.ai/api/v1"), c)?;
        self.seed_setting("ai_default_model", "openai/gpt-4o-mini", c)?;
        self.seed_setting(
            "task_archive_delay_secs",
            &task_archive_delay_secs.unwrap_or(5).to_string(),
            c,
        )?;
        self.seed_setting("overlay_opacity", "0.95", c)?;
        self.seed_setting("tab_icon_size", "16", c)?;
        self.seed_setting("ai_system_prompt", "You are a helpful assistant.", c)?;
        self.seed_setting("screen_size_inches", "24", c)?;
        Ok(())
    }

    // --- Migration v6: input monitoring tables ---

    fn migrate_v6(&mut self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS keystrokes (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    chars TEXT NOT NULL DEFAULT '',
                    window_title TEXT NOT NULL DEFAULT '',
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                );
                CREATE INDEX IF NOT EXISTS idx_keystrokes_created ON keystrokes(created_at);

                CREATE TABLE IF NOT EXISTS mouse_distance (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    distance_px REAL NOT NULL DEFAULT 0,
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                );
                CREATE INDEX IF NOT EXISTS idx_mouse_distance_created ON mouse_distance(created_at);

                CREATE TABLE IF NOT EXISTS window_activity (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    window_title TEXT NOT NULL DEFAULT '',
                    started_at TEXT NOT NULL DEFAULT (datetime('now')),
                    duration_secs INTEGER NOT NULL DEFAULT 0
                );
                CREATE INDEX IF NOT EXISTS idx_window_activity_started ON window_activity(started_at);",
            )
            .map_err(|e| format!("Migration v6 failed: {e}"))?;
        Ok(())
    }

    // --- Migration v7: add app_name column to keystrokes + window_activity ---

    fn migrate_v7(&mut self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "ALTER TABLE keystrokes ADD COLUMN app_name TEXT NOT NULL DEFAULT '';
                 ALTER TABLE window_activity ADD COLUMN app_name TEXT NOT NULL DEFAULT '';",
            )
            .map_err(|e| format!("Migration v7 failed: {e}"))?;
        Ok(())
    }

    // --- Input monitoring inserts ---

    pub fn insert_keystrokes(&self, chars: &str, app_name: &str, window_title: &str) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO keystrokes (chars, app_name, window_title) VALUES (?1, ?2, ?3)",
                params![chars, app_name, window_title],
            )
            .map_err(|e| format!("Failed to insert keystrokes: {e}"))?;
        Ok(())
    }

    pub fn insert_mouse_distance(&self, distance_px: f64) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO mouse_distance (distance_px) VALUES (?1)",
                params![distance_px],
            )
            .map_err(|e| format!("Failed to insert mouse distance: {e}"))?;
        Ok(())
    }

    pub fn insert_window_activity(&self, app_name: &str, window_title: &str, duration_secs: i64) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO window_activity (app_name, window_title, duration_secs) VALUES (?1, ?2, ?3)",
                params![app_name, window_title, duration_secs],
            )
            .map_err(|e| format!("Failed to insert window activity: {e}"))?;
        Ok(())
    }

    // --- Input monitoring queries ---

    #[allow(dead_code)]
    pub fn get_keystroke_count_since(&self, since: &str) -> Result<i64, String> {
        self.conn
            .query_row(
                "SELECT COALESCE(SUM(LENGTH(chars)), 0) FROM keystrokes WHERE created_at >= ?1",
                params![since],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to get keystroke count: {e}"))
    }

    #[allow(dead_code)]
    pub fn get_mouse_distance_since(&self, since: &str) -> Result<f64, String> {
        self.conn
            .query_row(
                "SELECT COALESCE(SUM(distance_px), 0.0) FROM mouse_distance WHERE created_at >= ?1",
                params![since],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to get mouse distance: {e}"))
    }

    pub fn get_keystroke_count_last_minute(&self) -> Result<i64, String> {
        self.conn
            .query_row(
                "SELECT COALESCE(SUM(LENGTH(chars)), 0) FROM keystrokes WHERE created_at >= datetime('now', '-1 minute')",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to get keystroke count last minute: {e}"))
    }

    /// Returns (app_name, chars) tuples, reverse-chronological then reversed for display order.
    pub fn get_recent_keystrokes(&self, limit_chars: i64) -> Result<Vec<(String, String)>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT app_name, chars FROM keystrokes WHERE app_name != '' ORDER BY id DESC")
            .map_err(|e| format!("Failed to prepare recent keystrokes: {e}"))?;

        let mut total = 0i64;
        let mut result = Vec::new();
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| format!("Failed to query keystrokes: {e}"))?;

        for row in rows {
            let (app_name, chars) = row.map_err(|e| format!("Failed to read keystroke row: {e}"))?;
            total += chars.len() as i64;
            result.push((app_name, chars));
            if total >= limit_chars {
                break;
            }
        }
        result.reverse();
        Ok(result)
    }

    pub fn conn_query_keystroke_count_since_raw(&self, offset: &str) -> Result<i64, String> {
        let sql = format!(
            "SELECT COALESCE(SUM(LENGTH(chars)), 0) FROM keystrokes WHERE created_at >= datetime('now', '{}')",
            offset
        );
        self.conn
            .query_row(&sql, [], |row| row.get(0))
            .map_err(|e| format!("Failed to get keystroke count: {e}"))
    }

    pub fn conn_query_mouse_distance_since_raw(&self, offset: &str) -> Result<f64, String> {
        let sql = format!(
            "SELECT COALESCE(SUM(distance_px), 0.0) FROM mouse_distance WHERE created_at >= datetime('now', '{}')",
            offset
        );
        self.conn
            .query_row(&sql, [], |row| row.get(0))
            .map_err(|e| format!("Failed to get mouse distance: {e}"))
    }

    pub fn get_top_windows_today(&self) -> Result<Vec<(String, i64)>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT app_name, SUM(duration_secs) as total
                 FROM window_activity
                 WHERE started_at >= date('now') AND app_name != ''
                 GROUP BY app_name
                 ORDER BY total DESC
                 LIMIT 20",
            )
            .map_err(|e| format!("Failed to prepare top windows: {e}"))?;

        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))
            .map_err(|e| format!("Failed to query top windows: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read window row: {e}"))?;

        Ok(rows)
    }

    // --- Migration v8: projects + project_rules tables, project_id on activity tables ---

    fn migrate_v8(&mut self) -> Result<(), String> {
        // Tables — idempotent via IF NOT EXISTS
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS projects (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    description TEXT NOT NULL DEFAULT '',
                    color TEXT NOT NULL DEFAULT '#93bbfc',
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                );

                CREATE TABLE IF NOT EXISTS project_rules (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                    expression TEXT NOT NULL,
                    priority INTEGER NOT NULL DEFAULT 0,
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                );
                CREATE INDEX IF NOT EXISTS idx_project_rules_project ON project_rules(project_id);",
            )
            .map_err(|e| format!("Migration v8 (tables) failed: {e}"))?;

        // ALTERs — check if column exists first (SQLite has no ADD COLUMN IF NOT EXISTS)
        if !self.column_exists("keystrokes", "project_id") {
            self.conn
                .execute_batch("ALTER TABLE keystrokes ADD COLUMN project_id INTEGER REFERENCES projects(id);")
                .map_err(|e| format!("Migration v8 (keystrokes alter) failed: {e}"))?;
        }
        if !self.column_exists("window_activity", "project_id") {
            self.conn
                .execute_batch("ALTER TABLE window_activity ADD COLUMN project_id INTEGER REFERENCES projects(id);")
                .map_err(|e| format!("Migration v8 (window_activity alter) failed: {e}"))?;
        }

        self.conn
            .execute_batch(
                "CREATE INDEX IF NOT EXISTS idx_keystrokes_project ON keystrokes(project_id);
                 CREATE INDEX IF NOT EXISTS idx_window_activity_project ON window_activity(project_id);",
            )
            .map_err(|e| format!("Migration v8 (indexes) failed: {e}"))?;

        Ok(())
    }

    fn column_exists(&self, table: &str, column: &str) -> bool {
        let sql = format!("PRAGMA table_info({})", table);
        let mut stmt = match self.conn.prepare(&sql) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let rows = match stmt.query_map([], |row| row.get::<_, String>(1)) {
            Ok(r) => r,
            Err(_) => return false,
        };
        for name in rows.flatten() {
            if name == column {
                return true;
            }
        }
        false
    }

    // --- Project CRUD ---

    pub fn create_project(&self, name: &str, description: &str, color: &str) -> Result<Project, String> {
        self.conn
            .execute(
                "INSERT INTO projects (name, description, color) VALUES (?1, ?2, ?3)",
                params![name, description, color],
            )
            .map_err(|e| format!("Failed to create project: {e}"))?;
        let id = self.conn.last_insert_rowid();
        self.get_project(id)
    }

    pub fn get_project(&self, id: i64) -> Result<Project, String> {
        self.conn
            .query_row(
                "SELECT id, name, description, color, created_at FROM projects WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Project {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        description: row.get(2)?,
                        color: row.get(3)?,
                        created_at: row.get(4)?,
                    })
                },
            )
            .map_err(|e| format!("Project not found: {e}"))
    }

    pub fn update_project(&self, id: i64, name: &str, description: &str, color: &str) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE projects SET name = ?1, description = ?2, color = ?3 WHERE id = ?4",
                params![name, description, color, id],
            )
            .map_err(|e| format!("Failed to update project: {e}"))?;
        Ok(())
    }

    pub fn delete_project(&self, id: i64) -> Result<(), String> {
        // Clear project_id on tagged activity before deleting
        self.clear_project_tags(id)?;
        self.conn
            .execute("DELETE FROM projects WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to delete project: {e}"))?;
        Ok(())
    }

    pub fn list_projects(&self) -> Result<Vec<Project>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, description, color, created_at FROM projects ORDER BY name ASC")
            .map_err(|e| format!("Failed to prepare list_projects: {e}"))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    color: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| format!("Failed to query projects: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read project row: {e}"))?;

        Ok(rows)
    }

    // --- Project Rule CRUD ---

    pub fn add_project_rule(&self, project_id: i64, expression: &str, priority: i32) -> Result<ProjectRule, String> {
        self.conn
            .execute(
                "INSERT INTO project_rules (project_id, expression, priority) VALUES (?1, ?2, ?3)",
                params![project_id, expression, priority],
            )
            .map_err(|e| format!("Failed to add project rule: {e}"))?;
        let id = self.conn.last_insert_rowid();
        self.conn
            .query_row(
                "SELECT id, project_id, expression, priority, created_at FROM project_rules WHERE id = ?1",
                params![id],
                |row| {
                    Ok(ProjectRule {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        expression: row.get(2)?,
                        priority: row.get(3)?,
                        created_at: row.get(4)?,
                    })
                },
            )
            .map_err(|e| format!("Project rule not found: {e}"))
    }

    pub fn delete_project_rule(&self, id: i64) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM project_rules WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to delete project rule: {e}"))?;
        Ok(())
    }

    pub fn get_project_rules(&self, project_id: i64) -> Result<Vec<ProjectRule>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, project_id, expression, priority, created_at FROM project_rules WHERE project_id = ?1 ORDER BY priority DESC")
            .map_err(|e| format!("Failed to prepare get_project_rules: {e}"))?;

        let rows = stmt
            .query_map(params![project_id], |row| {
                Ok(ProjectRule {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    expression: row.get(2)?,
                    priority: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| format!("Failed to query project rules: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read project rule row: {e}"))?;

        Ok(rows)
    }

    pub fn get_all_rules(&self) -> Result<Vec<ProjectRule>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, project_id, expression, priority, created_at FROM project_rules ORDER BY priority DESC")
            .map_err(|e| format!("Failed to prepare get_all_rules: {e}"))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ProjectRule {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    expression: row.get(2)?,
                    priority: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| format!("Failed to query all rules: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read rule row: {e}"))?;

        Ok(rows)
    }

    // --- Tagging methods ---

    pub fn get_untagged_activity(&self, limit: i64) -> Result<Vec<UntaggedActivity>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT * FROM (SELECT 'keystrokes' AS tbl, id, app_name, window_title FROM keystrokes WHERE project_id IS NULL AND app_name != '' LIMIT ?1)
                 UNION ALL
                 SELECT * FROM (SELECT 'window_activity' AS tbl, id, app_name, window_title FROM window_activity WHERE project_id IS NULL AND app_name != '' LIMIT ?1)",
            )
            .map_err(|e| format!("Failed to prepare get_untagged_activity: {e}"))?;

        let rows = stmt
            .query_map(params![limit], |row| {
                Ok(UntaggedActivity {
                    table: row.get(0)?,
                    id: row.get(1)?,
                    app_name: row.get(2)?,
                    window_title: row.get(3)?,
                })
            })
            .map_err(|e| format!("Failed to query untagged activity: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read untagged row: {e}"))?;

        Ok(rows)
    }

    pub fn tag_activities(&self, tags: &[ActivityTag]) -> Result<(), String> {
        let tx = self.conn.unchecked_transaction()
            .map_err(|e| format!("Failed to begin transaction: {e}"))?;

        for tag in tags {
            let sql = match tag.table.as_str() {
                "keystrokes" => "UPDATE keystrokes SET project_id = ?1 WHERE id = ?2",
                "window_activity" => "UPDATE window_activity SET project_id = ?1 WHERE id = ?2",
                other => return Err(format!("Unknown table: {other}")),
            };
            tx.execute(sql, params![tag.project_id, tag.id])
                .map_err(|e| format!("Failed to tag activity: {e}"))?;
        }

        tx.commit().map_err(|e| format!("Failed to commit tags: {e}"))?;
        Ok(())
    }

    pub fn clear_project_tags(&self, project_id: i64) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE keystrokes SET project_id = NULL WHERE project_id = ?1",
                params![project_id],
            )
            .map_err(|e| format!("Failed to clear keystroke tags: {e}"))?;
        self.conn
            .execute(
                "UPDATE window_activity SET project_id = NULL WHERE project_id = ?1",
                params![project_id],
            )
            .map_err(|e| format!("Failed to clear window activity tags: {e}"))?;
        Ok(())
    }

    // --- Project query methods ---

    pub fn get_all_project_summaries_today(&self) -> Result<Vec<ProjectSummary>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT p.id, p.name, p.color,
                    COALESCE(k.cnt, 0) as keystroke_count,
                    COALESCE(w.secs, 0) as active_secs
                 FROM projects p
                 LEFT JOIN (
                    SELECT project_id, SUM(LENGTH(chars)) as cnt
                    FROM keystrokes
                    WHERE project_id IS NOT NULL AND created_at >= date('now')
                    GROUP BY project_id
                 ) k ON k.project_id = p.id
                 LEFT JOIN (
                    SELECT project_id, SUM(duration_secs) as secs
                    FROM window_activity
                    WHERE project_id IS NOT NULL AND started_at >= date('now')
                    GROUP BY project_id
                 ) w ON w.project_id = p.id
                 ORDER BY active_secs DESC, keystroke_count DESC",
            )
            .map_err(|e| format!("Failed to prepare project summaries: {e}"))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ProjectSummary {
                    project_id: Some(row.get(0)?),
                    name: row.get(1)?,
                    color: row.get(2)?,
                    keystroke_count: row.get(3)?,
                    active_secs: row.get(4)?,
                })
            })
            .map_err(|e| format!("Failed to query project summaries: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read summary row: {e}"))?;

        Ok(rows)
    }

    // --- Activity summary for AI ---

    pub fn get_activity_summary_for_ai(&self, days: i32) -> Result<Vec<ActivitySummaryRow>, String> {
        let offset = format!("-{} days", days);
        let mut stmt = self
            .conn
            .prepare(
                "SELECT app_name, window_title, SUM(duration_secs) as total
                 FROM window_activity
                 WHERE started_at >= datetime('now', ?1)
                   AND app_name != ''
                 GROUP BY app_name, window_title
                 HAVING total >= 30
                 ORDER BY total DESC
                 LIMIT 100",
            )
            .map_err(|e| format!("Failed to prepare activity summary query: {e}"))?;

        let rows = stmt
            .query_map(params![offset], |row| {
                Ok(ActivitySummaryRow {
                    app_name: row.get(0)?,
                    window_title: row.get(1)?,
                    total_duration_secs: row.get(2)?,
                })
            })
            .map_err(|e| format!("Failed to query activity summary: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read activity summary row: {e}"))?;

        Ok(rows)
    }

    // --- Auto-suggest projects ---

    pub fn suggest_projects(&self) -> Result<Vec<ProjectSuggestion>, String> {
        use std::collections::HashMap;

        let mut suggestions: Vec<ProjectSuggestion> = Vec::new();
        let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Pre-seed seen_names with existing project names (case-insensitive) to avoid duplicates
        let existing = self.list_projects()?;
        for p in &existing {
            seen_names.insert(p.name.to_lowercase());
        }

        // 1. Extract VS Code workspace names
        let mut stmt = self
            .conn
            .prepare(
                "SELECT window_title FROM window_activity
                 WHERE app_name LIKE '%Code%' AND window_title LIKE '%Visual Studio Code%'
                 GROUP BY window_title",
            )
            .map_err(|e| format!("Failed to prepare VS Code suggestion query: {e}"))?;

        let vscode_titles: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| format!("Failed to query VS Code titles: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read VS Code title: {e}"))?;

        // Regex: extract workspace name before " - Visual Studio Code"
        for title in &vscode_titles {
            // Pattern: "filename - workspace - Visual Studio Code"
            if let Some(pos) = title.rfind(" - Visual Studio Code") {
                let prefix = &title[..pos];
                if let Some(dash_pos) = prefix.rfind(" - ") {
                    let workspace = prefix[dash_pos + 3..].trim();
                    let ws_lower = workspace.to_lowercase();
                    if !workspace.is_empty() && !seen_names.contains(&ws_lower) {
                        seen_names.insert(ws_lower);
                        suggestions.push(ProjectSuggestion {
                            name: workspace.to_string(),
                            rules: vec![format!(
                                "app_name = \"Code\" and $contains(window_title, \"{}\")",
                                workspace.replace('"', "\\\"")
                            )],
                        });
                    }
                }
            }
        }

        // 2. Extract Slack team/channel names
        let mut stmt2 = self
            .conn
            .prepare(
                "SELECT window_title FROM window_activity
                 WHERE LOWER(app_name) LIKE '%slack%'
                 GROUP BY window_title",
            )
            .map_err(|e| format!("Failed to prepare Slack suggestion query: {e}"))?;

        let slack_titles: Vec<String> = stmt2
            .query_map([], |row| row.get(0))
            .map_err(|e| format!("Failed to query Slack titles: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read Slack title: {e}"))?;

        // Group Slack as a single project
        if !slack_titles.is_empty() && !seen_names.contains("slack") {
            seen_names.insert("slack".to_string());
            suggestions.push(ProjectSuggestion {
                name: "Slack".to_string(),
                rules: vec!["$contains($lowercase(app_name), \"slack\")".to_string()],
            });
        }

        // 3. Group remaining by high-duration app_name
        let mut stmt3 = self
            .conn
            .prepare(
                "SELECT app_name, SUM(duration_secs) as total
                 FROM window_activity
                 WHERE app_name != '' AND started_at >= datetime('now', '-7 days')
                 GROUP BY app_name
                 HAVING total >= 300
                 ORDER BY total DESC
                 LIMIT 20",
            )
            .map_err(|e| format!("Failed to prepare app suggestion query: {e}"))?;

        let app_rows: Vec<(String, i64)> = stmt3
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))
            .map_err(|e| format!("Failed to query app suggestions: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read app suggestion: {e}"))?;

        // Build a map of app_name -> already suggested (skip Code/Slack if already suggested)
        let mut app_map: HashMap<String, bool> = HashMap::new();
        for (app_name, _) in &app_rows {
            let lower = app_name.to_lowercase();
            if lower.contains("code") || lower.contains("slack") {
                continue; // Already handled above
            }
            if seen_names.contains(&app_name.to_lowercase()) {
                continue;
            }
            app_map.entry(app_name.clone()).or_insert(true);
        }

        for (app_name, _) in app_map {
            seen_names.insert(app_name.to_lowercase());
            suggestions.push(ProjectSuggestion {
                name: app_name.clone(),
                rules: vec![format!("app_name = \"{}\"", app_name.replace('"', "\\\""))],
            });
        }

        Ok(suggestions)
    }

    // --- Untagged summary ---

    pub fn get_untagged_summary(&self) -> Result<(i64, Vec<UntaggedSummaryRow>), String> {
        // Total untagged count across both tables
        let total: i64 = self
            .conn
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM keystrokes WHERE project_id IS NULL AND app_name != '') +
                    (SELECT COUNT(*) FROM window_activity WHERE project_id IS NULL AND app_name != '')",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count untagged: {e}"))?;

        // Breakdown by app_name (top 20)
        let mut stmt = self
            .conn
            .prepare(
                "SELECT app_name,
                        SUM(CASE WHEN tbl = 'keystrokes' THEN cnt ELSE 0 END) as keystroke_rows,
                        SUM(CASE WHEN tbl = 'window_activity' THEN cnt ELSE 0 END) as window_rows,
                        SUM(cnt) as total
                 FROM (
                     SELECT 'keystrokes' AS tbl, app_name, COUNT(*) as cnt
                     FROM keystrokes WHERE project_id IS NULL AND app_name != ''
                     GROUP BY app_name
                     UNION ALL
                     SELECT 'window_activity' AS tbl, app_name, COUNT(*) as cnt
                     FROM window_activity WHERE project_id IS NULL AND app_name != ''
                     GROUP BY app_name
                 )
                 GROUP BY app_name
                 ORDER BY total DESC
                 LIMIT 20",
            )
            .map_err(|e| format!("Failed to prepare untagged summary: {e}"))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(UntaggedSummaryRow {
                    app_name: row.get(0)?,
                    keystroke_rows: row.get(1)?,
                    window_rows: row.get(2)?,
                    total: row.get(3)?,
                })
            })
            .map_err(|e| format!("Failed to query untagged summary: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read untagged summary row: {e}"))?;

        Ok((total, rows))
    }

    // --- Project activities ---

    pub fn get_project_activities(
        &self,
        project_id: i64,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ProjectActivity>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, app_name, window_title, duration_secs, started_at
                 FROM window_activity
                 WHERE project_id = ?1
                 ORDER BY started_at DESC
                 LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| format!("Failed to prepare project activities: {e}"))?;

        let rows = stmt
            .query_map(params![project_id, limit, offset], |row| {
                Ok(ProjectActivity {
                    id: row.get(0)?,
                    app_name: row.get(1)?,
                    window_title: row.get(2)?,
                    duration_secs: row.get(3)?,
                    started_at: row.get(4)?,
                })
            })
            .map_err(|e| format!("Failed to query project activities: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read project activity row: {e}"))?;

        Ok(rows)
    }

    // --- Migration v9: tips table ---

    fn migrate_v9(&mut self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS tips (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    content TEXT NOT NULL,
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                );
                CREATE INDEX IF NOT EXISTS idx_tips_created ON tips(created_at);",
            )
            .map_err(|e| format!("Migration v9 failed: {e}"))?;
        Ok(())
    }

    // --- Tips CRUD ---

    pub fn insert_tip(&self, content: &str) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO tips (content) VALUES (?1)",
                params![content],
            )
            .map_err(|e| format!("Failed to insert tip: {e}"))?;
        Ok(())
    }

    pub fn get_recent_tips(&self, limit: i64) -> Result<Vec<(String, String)>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT content, created_at FROM tips ORDER BY id DESC LIMIT ?1",
            )
            .map_err(|e| format!("Failed to prepare get_recent_tips: {e}"))?;

        let rows = stmt
            .query_map(params![limit], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| format!("Failed to query tips: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read tip row: {e}"))?;

        Ok(rows)
    }

    /// Aggregates recent keystrokes into a human-readable summary (per-app char counts + app switches).
    pub fn summarize_recent_keystrokes(&self, limit_chars: i64) -> Result<String, String> {
        let rows = self.get_recent_keystrokes(limit_chars)?;
        if rows.is_empty() {
            return Ok("No recent keystrokes recorded.".to_string());
        }

        let mut app_chars: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut app_switches = 0usize;
        let mut last_app: Option<String> = None;

        for (app_name, chars) in &rows {
            *app_chars.entry(app_name.clone()).or_insert(0) += chars.len();
            if let Some(ref prev) = last_app {
                if prev != app_name {
                    app_switches += 1;
                }
            }
            last_app = Some(app_name.clone());
        }

        // Sort by char count descending
        let mut sorted: Vec<_> = app_chars.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        let mut summary = String::new();
        for (app, count) in &sorted {
            summary.push_str(&format!("- {} chars in {}\n", count, app));
        }
        summary.push_str(&format!("- {} app switches", app_switches));

        Ok(summary)
    }

    // --- Time query methods ---

    pub fn get_total_active_secs_today(&self) -> Result<i64, String> {
        self.conn
            .query_row(
                "SELECT COALESCE(SUM(duration_secs), 0) FROM window_activity WHERE started_at >= date('now')",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to get total active secs today: {e}"))
    }

    pub fn get_current_app_time_today(&self, app_name: &str) -> Result<i64, String> {
        self.conn
            .query_row(
                "SELECT COALESCE(SUM(duration_secs), 0) FROM window_activity WHERE started_at >= date('now') AND app_name = ?1",
                params![app_name],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to get app time today: {e}"))
    }

    // --- Build chat context for AI ---

    pub fn build_chat_context(&self) -> Result<String, String> {
        let mut ctx = String::from("\n\n--- User's Activity Data ---\n");

        // Project summaries
        let summaries = self.get_all_project_summaries_today()?;
        if !summaries.is_empty() {
            ctx.push_str("\nProjects (today's stats):\n");
            for s in &summaries {
                ctx.push_str(&format!(
                    "- {} — {} keys, {} active\n",
                    s.name, s.keystroke_count, format_secs(s.active_secs)
                ));
            }
        } else {
            ctx.push_str("\nNo projects configured yet.\n");
        }

        // Tagging rules grouped by project
        let projects = self.list_projects()?;
        let rules = self.get_all_rules()?;
        if !rules.is_empty() {
            ctx.push_str("\nTagging rules:\n");
            for p in &projects {
                let project_rules: Vec<&str> = rules
                    .iter()
                    .filter(|r| r.project_id == p.id)
                    .map(|r| r.expression.as_str())
                    .collect();
                if !project_rules.is_empty() {
                    ctx.push_str(&format!("- {}: {}\n", p.name, project_rules.join(" | ")));
                }
            }
        }

        // Untagged summary
        let (untagged_total, untagged_apps) = self.get_untagged_summary()?;
        if untagged_total > 0 {
            ctx.push_str(&format!("\nUntagged activities: {} total\n", untagged_total));
            let top_apps: Vec<String> = untagged_apps
                .iter()
                .take(10)
                .map(|r| format!("{} ({})", r.app_name, r.total))
                .collect();
            ctx.push_str(&format!("Top untagged apps: {}\n", top_apps.join(", ")));
        }

        // Top windows today
        let top_windows = self.get_top_windows_today()?;
        if !top_windows.is_empty() {
            ctx.push_str("\nTop apps today (by time):\n");
            for (app, secs) in top_windows.iter().take(10) {
                ctx.push_str(&format!("- {} — {}\n", app, format_secs(*secs)));
            }
        }

        ctx.push_str("--- End Activity Data ---\n");
        Ok(ctx)
    }

    // --- Migration v10: notes table ---

    fn migrate_v10(&mut self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS notes (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    title TEXT NOT NULL DEFAULT '',
                    content TEXT NOT NULL DEFAULT '',
                    created_at TEXT NOT NULL DEFAULT (datetime('now')),
                    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
                );
                CREATE INDEX IF NOT EXISTS idx_notes_updated ON notes(updated_at);",
            )
            .map_err(|e| format!("Migration v10 failed: {e}"))?;
        Ok(())
    }

    // --- Notes CRUD ---

    pub fn create_note(&self, title: &str, content: &str) -> Result<Note, String> {
        self.conn
            .execute(
                "INSERT INTO notes (title, content) VALUES (?1, ?2)",
                params![title, content],
            )
            .map_err(|e| format!("Failed to create note: {e}"))?;
        let id = self.conn.last_insert_rowid();
        self.get_note(id)
    }

    pub fn get_note(&self, id: i64) -> Result<Note, String> {
        self.conn
            .query_row(
                "SELECT id, title, content, created_at, updated_at FROM notes WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Note {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        content: row.get(2)?,
                        created_at: row.get(3)?,
                        updated_at: row.get(4)?,
                    })
                },
            )
            .map_err(|e| format!("Note not found: {e}"))
    }

    pub fn list_notes(&self) -> Result<Vec<Note>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, title, content, created_at, updated_at FROM notes ORDER BY updated_at DESC")
            .map_err(|e| format!("Failed to prepare list_notes: {e}"))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Note {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })
            .map_err(|e| format!("Failed to query notes: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read note row: {e}"))?;

        Ok(rows)
    }

    pub fn update_note(&self, id: i64, title: &str, content: &str) -> Result<Note, String> {
        self.conn
            .execute(
                "UPDATE notes SET title = ?1, content = ?2, updated_at = datetime('now') WHERE id = ?3",
                params![title, content, id],
            )
            .map_err(|e| format!("Failed to update note: {e}"))?;
        self.get_note(id)
    }

    pub fn delete_note(&self, id: i64) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM notes WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to delete note: {e}"))?;
        Ok(())
    }
}

fn format_secs(secs: i64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        format!("{}h {}m", h, m)
    }
}
