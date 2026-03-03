use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

use crate::ai::{self, ChatMessage};
use crate::db::{
    ActivityTag, ChatMessageRow, Conversation, Database, Project, ProjectActivity, ProjectRule,
    ProjectSuggestion, ProjectSummary, Task, UntaggedActivity, UntaggedSummaryRow,
};
use crate::tools::{self, ToolCallResult};

pub struct DbState {
    pub db: Mutex<Database>,
}

pub struct ForegroundTitleState {
    pub title: Mutex<String>,
}

#[derive(serde::Serialize)]
pub struct LoadConversationResponse {
    pub conversation: Conversation,
    pub messages: Vec<ChatMessageRow>,
}

#[derive(Clone, serde::Serialize)]
pub struct ChatDonePayload {
    pub id: i64,
    pub content: String,
    pub model: String,
}

#[derive(Clone, serde::Serialize)]
pub struct ToolUsedPayload {
    pub tools: Vec<ToolCallResult>,
}

// --- Settings commands ---

#[tauri::command]
pub fn get_settings(db_state: State<DbState>) -> Result<HashMap<String, String>, String> {
    let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    let pairs = db.get_all_settings("default")?;
    Ok(pairs.into_iter().collect())
}

#[tauri::command]
pub fn get_setting(db_state: State<DbState>, key: String) -> Result<Option<String>, String> {
    let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.get_setting(&key, "default")
}

#[tauri::command]
pub fn update_setting(db_state: State<DbState>, key: String, value: String) -> Result<(), String> {
    if key == "hotkey" {
        return Err("Use update_hotkey command to change the hotkey".to_string());
    }
    let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.set_setting(&key, &value, "default")
}

#[tauri::command]
pub fn update_hotkey(
    app: AppHandle,
    db_state: State<DbState>,
    new_hotkey: String,
) -> Result<(), String> {
    // Read the old hotkey from DB
    let old_hotkey = {
        let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
        db.get_setting("hotkey", "default")?
            .unwrap_or_else(|| "Ctrl+Shift+Space".to_string())
    };

    // Unregister old hotkey
    let _ = app.global_shortcut().unregister(old_hotkey.as_str());

    // Try to register new hotkey
    let register_result = app.global_shortcut().on_shortcut(
        new_hotkey.as_str(),
        |app_handle, _shortcut, event| {
            if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                crate::toggle_overlay(app_handle);
            }
        },
    );

    match register_result {
        Ok(()) => {
            // Save new hotkey to DB
            let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
            db.set_setting("hotkey", &new_hotkey, "default")?;
            Ok(())
        }
        Err(e) => {
            // Rollback: re-register old hotkey
            let _ = app.global_shortcut().on_shortcut(
                old_hotkey.as_str(),
                |app_handle, _shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        crate::toggle_overlay(app_handle);
                    }
                },
            );
            Err(format!("Invalid hotkey '{}': {}", new_hotkey, e))
        }
    }
}

// --- Event commands ---

#[tauri::command]
pub fn insert_event(
    state: State<DbState>,
    event_type: String,
    payload: String,
) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.insert_event(&event_type, &payload)
}

#[tauri::command]
pub fn get_recent_events(
    state: State<DbState>,
    limit: i64,
) -> Result<Vec<crate::db::Event>, String> {
    let db = state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.get_recent_events(limit)
}

// --- Conversation commands ---

#[tauri::command]
pub fn create_conversation(
    state: State<DbState>,
    title: Option<String>,
) -> Result<Conversation, String> {
    let db = state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.create_conversation(title.as_deref())
}

#[tauri::command]
pub fn list_conversations(state: State<DbState>) -> Result<Vec<Conversation>, String> {
    let db = state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.list_conversations()
}

#[tauri::command]
pub fn delete_conversation(
    state: State<DbState>,
    conversation_id: i64,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.delete_conversation(conversation_id)
}

#[tauri::command]
pub fn load_conversation(
    state: State<DbState>,
    conversation_id: i64,
) -> Result<LoadConversationResponse, String> {
    let db = state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    let conversation = db.get_conversation(conversation_id)?;
    let messages = db.get_conversation_messages(conversation_id)?;
    Ok(LoadConversationResponse {
        conversation,
        messages,
    })
}

#[tauri::command]
pub fn save_user_message(
    state: State<DbState>,
    conversation_id: i64,
    parent_id: Option<i64>,
    content: String,
) -> Result<ChatMessageRow, String> {
    let db = state.db.lock().map_err(|e| format!("Lock error: {e}"))?;

    let msg = db.insert_chat_message(conversation_id, parent_id, "user", &content, None)?;

    // Update parent's active_child or conversation's active_root
    if let Some(pid) = parent_id {
        db.set_active_child(pid, msg.id)?;
    } else {
        db.set_conversation_active_root(conversation_id, msg.id)?;
    }

    // Auto-title from first user message (first 50 chars)
    let conv = db.get_conversation(conversation_id)?;
    if conv.title == "New conversation" {
        let title: String = content.chars().take(50).collect();
        db.update_conversation_title(conversation_id, &title)?;
    }

    db.touch_conversation(conversation_id)?;
    // Re-fetch with updated active_child_id state
    db.get_chat_message(msg.id)
}

#[tauri::command]
pub fn set_active_child(
    state: State<DbState>,
    message_id: i64,
    child_id: i64,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.set_active_child(message_id, child_id)
}

#[tauri::command]
pub fn set_conversation_active_root(
    state: State<DbState>,
    conversation_id: i64,
    message_id: i64,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.set_conversation_active_root(conversation_id, message_id)
}

// --- send_chat_message: reads AI config from DB, with tool-call loop ---

#[tauri::command]
pub async fn send_chat_message(
    app: AppHandle,
    db_state: State<'_, DbState>,
    conversation_id: i64,
    parent_message_id: i64,
    messages: Vec<ChatMessage>,
    model: Option<String>,
) -> Result<(), String> {
    // Read AI config from DB in a scoped block (drop lock before .await)
    let (api_key, base_url, default_model, system_prompt) = {
        let db = db_state
            .db
            .lock()
            .map_err(|e| format!("Lock error: {e}"))?;
        let key = db
            .get_setting("ai_api_key", "default")?
            .filter(|k| !k.is_empty())
            .ok_or_else(|| "No API key configured. Set it in Settings.".to_string())?;
        let url = db
            .get_setting("ai_base_url", "default")?
            .unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());
        let dm = db
            .get_setting("ai_default_model", "default")?
            .unwrap_or_else(|| "openai/gpt-4o-mini".to_string());
        let mut sp = db
            .get_setting("ai_system_prompt", "default")?
            .unwrap_or_else(|| "You are a helpful assistant.".to_string());
        // Inject activity context so the AI knows about the user's projects/data
        if let Ok(context) = db.build_chat_context() {
            sp.push_str(&context);
        }
        (key, url, dm, sp)
    };

    let model = model.unwrap_or(default_model);
    let client = reqwest::Client::new();

    // Prepend system prompt to messages
    let mut full_messages = vec![ChatMessage::text("system", &system_prompt)];
    full_messages.extend(messages);

    // --- Tool-call loop (non-streaming) ---
    let tool_defs = tools::get_tool_definitions();
    let mut all_tool_indicators: Vec<ToolCallResult> = Vec::new();

    for _ in 0..tools::MAX_TOOL_ITERATIONS {
        let completion = ai::chat_completion_with_tools(
            &client,
            &base_url,
            &api_key,
            &model,
            full_messages.clone(),
            tool_defs.clone(),
        )
        .await;

        let completion = match completion {
            Ok(c) => c,
            Err(_) => break, // Model doesn't support tools or API error — fall through
        };

        let tool_calls = match completion.tool_calls {
            Some(tc) if !tc.is_empty() => tc,
            _ => break, // No tool calls — done with loop
        };

        // Append the assistant message (with tool_calls) to the conversation
        full_messages.push(ChatMessage {
            role: "assistant".to_string(),
            content: completion.content.clone(),
            tool_calls: Some(tool_calls.clone()),
            tool_call_id: None,
            name: None,
        });

        // Execute each tool call and append results
        for tc in &tool_calls {
            let (json_result, summary) = {
                let db = db_state
                    .db
                    .lock()
                    .map_err(|e| format!("Lock error: {e}"))?;
                match tools::execute_tool(&db, &tc.function.name, &tc.function.arguments) {
                    Ok(r) => r,
                    Err(e) => {
                        let err_json = serde_json::json!({"error": e}).to_string();
                        (err_json, format!("error: {}", e))
                    }
                }
            };

            all_tool_indicators.push(ToolCallResult {
                tool_name: tc.function.name.clone(),
                summary: summary.clone(),
            });

            full_messages.push(ChatMessage::tool_result(
                &tc.id,
                &tc.function.name,
                &json_result,
            ));
        }
    }

    // Emit tool indicators if any tools were called
    if !all_tool_indicators.is_empty() {
        let _ = app.emit(
            "chat-tools-used",
            ToolUsedPayload {
                tools: all_tool_indicators,
            },
        );
    }

    // --- Stream final answer (no tools param) ---
    let mut full_response = String::new();
    let app_handle = app.clone();
    let result = crate::ai::stream_chat(
        &client,
        &base_url,
        &api_key,
        &model,
        full_messages,
        |content| {
            full_response.push_str(&content);
            let _ = app_handle.emit("chat-chunk", &content);
        },
    )
    .await;

    match result {
        Ok(()) => {
            // Persist assistant message to DB
            let payload = {
                let db = db_state
                    .db
                    .lock()
                    .map_err(|e| format!("Lock error: {e}"))?;
                let msg = db.insert_chat_message(
                    conversation_id,
                    Some(parent_message_id),
                    "assistant",
                    &full_response,
                    Some(&model),
                )?;
                db.set_active_child(parent_message_id, msg.id)?;
                db.touch_conversation(conversation_id)?;
                ChatDonePayload {
                    id: msg.id,
                    content: full_response,
                    model,
                }
            };
            let _ = app.emit("chat-done", payload);
            Ok(())
        }
        Err(e) => {
            let _ = app.emit("chat-error", &e);
            Err(e)
        }
    }
}

// --- Task commands ---

#[tauri::command]
pub fn create_task(
    db_state: State<DbState>,
    fg_state: State<ForegroundTitleState>,
    content: String,
) -> Result<Task, String> {
    let window_title = fg_state
        .title
        .lock()
        .map_err(|e| format!("Lock error: {e}"))?
        .clone();
    let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.create_task(&content, &window_title)
}

#[tauri::command]
pub fn list_tasks(state: State<DbState>, archived: bool) -> Result<Vec<Task>, String> {
    let db = state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.list_tasks(archived)
}

#[tauri::command]
pub fn update_task_completed(
    state: State<DbState>,
    task_id: i64,
    completed: bool,
) -> Result<Task, String> {
    let db = state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.update_task_completed(task_id, completed)
}

#[tauri::command]
pub fn archive_task(state: State<DbState>, task_id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.archive_task(task_id)
}

#[tauri::command]
pub fn delete_task(state: State<DbState>, task_id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.delete_task(task_id)
}

#[tauri::command]
pub fn get_foreground_title(state: State<ForegroundTitleState>) -> Result<String, String> {
    let title = state
        .title
        .lock()
        .map_err(|e| format!("Lock error: {e}"))?;
    Ok(title.clone())
}

// --- Stats commands ---

#[derive(serde::Serialize)]
pub struct InputStats {
    pub keystroke_counts: std::collections::HashMap<String, i64>,
    pub mouse_distances_px: std::collections::HashMap<String, f64>,
    pub mouse_distances_physical: std::collections::HashMap<String, f64>,
    pub wpm: f64,
    pub screen_resolution: (i32, i32),
    pub screen_size_inches: f64,
}

#[derive(serde::Serialize)]
pub struct InputGroup {
    pub app_name: String,
    pub text: String,
}

#[derive(serde::Serialize)]
pub struct WindowStat {
    pub title: String,
    pub duration_secs: i64,
}

#[tauri::command]
pub fn get_input_stats(db_state: State<DbState>) -> Result<InputStats, String> {
    let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;

    let screen_size_inches: f64 = db
        .get_setting("screen_size_inches", "default")?
        .and_then(|s| s.parse().ok())
        .unwrap_or(24.0);

    let resolution = crate::input_monitor::get_screen_resolution();
    let diagonal_px = ((resolution.0 as f64).powi(2) + (resolution.1 as f64).powi(2)).sqrt();
    let px_per_inch = if screen_size_inches > 0.0 {
        diagonal_px / screen_size_inches
    } else {
        96.0
    };

    let periods: &[(&str, &str)] = &[
        ("second", "-1 second"),
        ("minute", "-1 minute"),
        ("hour", "-1 hour"),
        ("day", "-1 day"),
        ("week", "-7 days"),
        ("month", "-30 days"),
        ("year", "-365 days"),
    ];

    let mut keystroke_counts = std::collections::HashMap::new();
    let mut mouse_distances_px = std::collections::HashMap::new();
    let mut mouse_distances_physical = std::collections::HashMap::new();

    for (label, offset) in periods {
        let ks: i64 = db
            .conn_query_keystroke_count_since_raw(offset)
            .unwrap_or(0);
        let md: f64 = db
            .conn_query_mouse_distance_since_raw(offset)
            .unwrap_or(0.0);

        keystroke_counts.insert(label.to_string(), ks);
        mouse_distances_px.insert(label.to_string(), md);
        mouse_distances_physical.insert(label.to_string(), md / px_per_inch);
    }

    let chars_last_minute = db.get_keystroke_count_last_minute().unwrap_or(0);
    let wpm = chars_last_minute as f64 / 5.0;

    Ok(InputStats {
        keystroke_counts,
        mouse_distances_px,
        mouse_distances_physical,
        wpm,
        screen_resolution: resolution,
        screen_size_inches,
    })
}

#[tauri::command]
pub fn get_recent_input(db_state: State<DbState>, limit_bytes: i64) -> Result<Vec<InputGroup>, String> {
    let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    let rows = db.get_recent_keystrokes(limit_bytes)?;

    // Group consecutive same-app entries
    let mut groups: Vec<InputGroup> = Vec::new();
    for (app_name, chars) in rows {
        if let Some(last) = groups.last_mut() {
            if last.app_name == app_name {
                last.text.push_str(&chars);
                continue;
            }
        }
        groups.push(InputGroup {
            app_name,
            text: chars,
        });
    }

    Ok(groups)
}

#[tauri::command]
pub fn get_top_windows(db_state: State<DbState>) -> Result<Vec<WindowStat>, String> {
    let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    let rows = db.get_top_windows_today()?;
    Ok(rows
        .into_iter()
        .map(|(title, duration_secs)| WindowStat {
            title,
            duration_secs,
        })
        .collect())
}

// --- Project commands ---

#[tauri::command]
pub fn create_project(
    db_state: State<DbState>,
    name: String,
    description: String,
    color: String,
) -> Result<Project, String> {
    let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.create_project(&name, &description, &color)
}

#[tauri::command]
pub fn update_project(
    db_state: State<DbState>,
    id: i64,
    name: String,
    description: String,
    color: String,
) -> Result<(), String> {
    let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.update_project(id, &name, &description, &color)
}

#[tauri::command]
pub fn delete_project(db_state: State<DbState>, id: i64) -> Result<(), String> {
    let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.delete_project(id)
}

#[tauri::command]
pub fn list_projects(db_state: State<DbState>) -> Result<Vec<Project>, String> {
    let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.list_projects()
}

#[tauri::command]
pub fn add_project_rule(
    db_state: State<DbState>,
    project_id: i64,
    expression: String,
    priority: i32,
) -> Result<ProjectRule, String> {
    let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.add_project_rule(project_id, &expression, priority)
}

#[tauri::command]
pub fn delete_project_rule(db_state: State<DbState>, id: i64) -> Result<(), String> {
    let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.delete_project_rule(id)
}

#[tauri::command]
pub fn get_project_rules(
    db_state: State<DbState>,
    project_id: i64,
) -> Result<Vec<ProjectRule>, String> {
    let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.get_project_rules(project_id)
}

#[tauri::command]
pub fn get_all_rules(db_state: State<DbState>) -> Result<Vec<ProjectRule>, String> {
    let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.get_all_rules()
}

#[tauri::command]
pub fn get_untagged_activity(
    db_state: State<DbState>,
    limit: i64,
) -> Result<Vec<UntaggedActivity>, String> {
    let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.get_untagged_activity(limit)
}

#[tauri::command]
pub fn tag_activities(
    db_state: State<DbState>,
    tags: Vec<ActivityTag>,
) -> Result<(), String> {
    let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.tag_activities(&tags)
}

#[tauri::command]
pub fn clear_project_tags(db_state: State<DbState>, project_id: i64) -> Result<(), String> {
    let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.clear_project_tags(project_id)
}

#[tauri::command]
pub fn suggest_projects(db_state: State<DbState>) -> Result<Vec<ProjectSuggestion>, String> {
    let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.suggest_projects()
}

#[tauri::command]
pub fn get_all_project_summaries_today(
    db_state: State<DbState>,
) -> Result<Vec<ProjectSummary>, String> {
    let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.get_all_project_summaries_today()
}

#[derive(serde::Serialize)]
pub struct UntaggedSummaryResponse {
    pub total: i64,
    pub by_app: Vec<UntaggedSummaryRow>,
}

#[tauri::command]
pub fn get_untagged_summary(db_state: State<DbState>) -> Result<UntaggedSummaryResponse, String> {
    let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    let (total, by_app) = db.get_untagged_summary()?;
    Ok(UntaggedSummaryResponse { total, by_app })
}

#[tauri::command]
pub fn get_project_activities(
    db_state: State<DbState>,
    project_id: i64,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<ProjectActivity>, String> {
    let db = db_state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
    db.get_project_activities(project_id, limit.unwrap_or(50), offset.unwrap_or(0))
}

// --- Smart Suggest (AI-powered) ---

#[tauri::command]
pub async fn smart_suggest_projects(
    db_state: State<'_, DbState>,
    days: Option<i32>,
) -> Result<Vec<ProjectSuggestion>, String> {
    let days = days.unwrap_or(7);

    // Read AI config + activity summary + existing project names under lock
    let (api_key, base_url, model, activity_rows, existing_names) = {
        let db = db_state
            .db
            .lock()
            .map_err(|e| format!("Lock error: {e}"))?;

        let key = db
            .get_setting("ai_api_key", "default")?
            .filter(|k| !k.is_empty())
            .ok_or_else(|| "No API key configured. Set it in Settings.".to_string())?;
        let url = db
            .get_setting("ai_base_url", "default")?
            .unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());
        let m = db
            .get_setting("ai_default_model", "default")?
            .unwrap_or_else(|| "openai/gpt-4o-mini".to_string());

        let rows = db.get_activity_summary_for_ai(days)?;
        let existing: Vec<String> = db
            .list_projects()?
            .into_iter()
            .map(|p| p.name.to_lowercase())
            .collect();

        (key, url, m, rows, existing)
    };
    // Lock is dropped here

    if activity_rows.is_empty() {
        return Err("No activity data found. Use your computer for a while first.".to_string());
    }

    // Build activity summary table for the prompt
    let mut activity_table = String::from("app_name | window_title | duration_secs\n---|---|---\n");
    for row in &activity_rows {
        activity_table.push_str(&format!(
            "{} | {} | {}\n",
            row.app_name, row.window_title, row.total_duration_secs
        ));
    }

    let system_prompt = r#"You analyze desktop activity data and suggest project groupings.

You will receive a table of window activity data with columns: app_name, window_title, duration_secs.

Your task is to identify cross-app projects. A project is a logical grouping of activities across multiple apps. For example, working on "aiHelper" might involve VS Code (editing code), Chrome (GitHub, docs), Terminal (cargo, git), and Slack (team channel).

Group cross-app activity by project when you see the same project/repo/topic name appearing across different apps.

Return a JSON array of project objects. Each object has:
- "name": string — short project name (e.g., "aiHelper", "Documentation", "Email")
- "rules": array of JSONata expression strings that match against {app_name, window_title} objects

JSONata syntax reference:
- Equality: app_name = "Code"
- Contains: $contains(window_title, "aiHelper")
- Case-insensitive: $contains($lowercase(window_title), "github")
- OR multiple conditions: app_name = "Code" or app_name = "Terminal"
- AND: app_name = "Code" and $contains(window_title, "myProject")

Each rule should match one pattern. Use multiple rules per project to capture different apps.

Example output:
```json
[
  {
    "name": "aiHelper",
    "rules": [
      "app_name = \"Code\" and $contains(window_title, \"aiHelper\")",
      "$contains($lowercase(window_title), \"aihelper\")"
    ]
  }
]
```

IMPORTANT:
- Return ONLY the JSON array, no other text
- Do not wrap in markdown code fences
- Keep project names concise
- Create 2-8 projects maximum
- Each rule must be a valid JSONata expression"#;

    let messages = vec![
        ChatMessage::text("system", system_prompt),
        ChatMessage::text(
            "user",
            &format!(
                "Here is my desktop activity from the last {} days. Suggest project groupings:\n\n{}",
                days, activity_table
            ),
        ),
    ];

    let client = reqwest::Client::new();
    let response = ai::chat_completion(&client, &base_url, &api_key, &model, messages).await?;

    // Parse JSON response — strip markdown code fences if present
    let json_str = extract_json_array(&response)?;
    let suggestions: Vec<ProjectSuggestion> =
        serde_json::from_str(&json_str).map_err(|e| format!("Failed to parse AI response as project suggestions: {e}"))?;

    // Filter out duplicates of existing project names
    let filtered: Vec<ProjectSuggestion> = suggestions
        .into_iter()
        .filter(|s| !existing_names.contains(&s.name.to_lowercase()))
        .collect();

    Ok(filtered)
}

// --- Generate Tip (AI-powered) ---

#[tauri::command]
pub async fn generate_tip(
    db_state: State<'_, DbState>,
) -> Result<String, String> {
    // Read AI config + activity context under lock, then drop before await
    let (api_key, base_url, model, context, keystrokes) = {
        let db = db_state
            .db
            .lock()
            .map_err(|e| format!("Lock error: {e}"))?;

        let key = db
            .get_setting("ai_api_key", "default")?
            .filter(|k| !k.is_empty())
            .ok_or_else(|| "No API key configured. Set it in Settings.".to_string())?;
        let url = db
            .get_setting("ai_base_url", "default")?
            .unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());
        let m = db
            .get_setting("ai_default_model", "default")?
            .unwrap_or_else(|| "openai/gpt-4o-mini".to_string());

        let ctx = db.build_chat_context().unwrap_or_default();

        // Get recent keystrokes for specificity
        let ks_rows = db.get_recent_keystrokes(2048).unwrap_or_default();
        let ks_text: String = ks_rows
            .into_iter()
            .map(|(app, chars)| format!("[{}] {}", app, chars))
            .collect::<Vec<_>>()
            .join("\n");

        (key, url, m, ctx, ks_text)
    };
    // Lock is dropped here

    let mut system_prompt = String::from(
        "You are a concise productivity assistant. Based on the activity data below, \
         generate ONE short actionable tip. Under 3 sentences. Be specific to what the user \
         is actually doing right now.",
    );
    system_prompt.push_str(&context);
    if !keystrokes.is_empty() {
        system_prompt.push_str("\n\n--- Recent Keystrokes ---\n");
        system_prompt.push_str(&keystrokes);
    }

    let messages = vec![
        ChatMessage::text("system", &system_prompt),
        ChatMessage::text("user", "Give me a productivity tip based on my current activity."),
    ];

    let client = reqwest::Client::new();
    ai::chat_completion(&client, &base_url, &api_key, &model, messages).await
}

/// Extract a JSON array from text that may contain markdown code fences or surrounding text.
fn extract_json_array(text: &str) -> Result<String, String> {
    let trimmed = text.trim();

    // Try direct parse first
    if trimmed.starts_with('[') {
        return Ok(trimmed.to_string());
    }

    // Strip markdown code fences: ```json ... ``` or ``` ... ```
    if let Some(start) = trimmed.find("```") {
        let after_fence = &trimmed[start + 3..];
        // Skip optional language tag (e.g., "json")
        let content_start = after_fence.find('\n').unwrap_or(0) + 1;
        let content = &after_fence[content_start..];
        if let Some(end) = content.find("```") {
            let inner = content[..end].trim();
            if inner.starts_with('[') {
                return Ok(inner.to_string());
            }
        }
    }

    // Try to find a JSON array with bracket matching
    if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed.rfind(']') {
            if end > start {
                return Ok(trimmed[start..=end].to_string());
            }
        }
    }

    Err("Could not find JSON array in AI response".to_string())
}
