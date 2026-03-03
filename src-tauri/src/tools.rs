use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::db::Database;

pub const MAX_TOOL_ITERATIONS: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolCallResult {
    pub tool_name: String,
    pub summary: String,
}

pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "list_projects".to_string(),
                description: "List all user-defined projects with their id, name, description, and color.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_project_summaries_today".to_string(),
                description: "Get today's activity summary per project: keystroke count and active seconds.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_project_rules".to_string(),
                description: "Get the matching rules (JSONata expressions) for a specific project.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "project_id": {
                            "type": "integer",
                            "description": "The project ID to get rules for."
                        }
                    },
                    "required": ["project_id"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_all_rules".to_string(),
                description: "Get all project matching rules across all projects.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_untagged_summary".to_string(),
                description: "Get a summary of untagged (not assigned to any project) activity, broken down by app.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_project_activities".to_string(),
                description: "Get recent activity entries for a specific project (app name, window title, duration, timestamp).".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "project_id": {
                            "type": "integer",
                            "description": "The project ID."
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Max rows to return (default 50)."
                        },
                        "offset": {
                            "type": "integer",
                            "description": "Row offset for pagination (default 0)."
                        }
                    },
                    "required": ["project_id"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "list_tasks".to_string(),
                description: "List the user's tasks. Set archived=true to see archived tasks, or false/omit for active tasks.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "archived": {
                            "type": "boolean",
                            "description": "Whether to list archived tasks (default false)."
                        }
                    },
                    "required": []
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_top_windows_today".to_string(),
                description: "Get the top window titles by time spent today, with duration in seconds.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_recent_events".to_string(),
                description: "Get recent system events (e.g. UI actions, errors) with type, payload, and timestamp.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Max events to return (default 20)."
                        }
                    },
                    "required": []
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_all_settings".to_string(),
                description: "Get all user settings (api key is redacted for security).".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        },
    ]
}

/// Execute a tool by name against the database. Returns (json_result, human_summary).
pub fn execute_tool(db: &Database, name: &str, arguments: &str) -> Result<(String, String), String> {
    let args: Value = serde_json::from_str(arguments).unwrap_or(json!({}));

    match name {
        "list_projects" => {
            let projects = db.list_projects()?;
            let summary = format!("{} projects", projects.len());
            Ok((serde_json::to_string(&projects).unwrap_or_default(), summary))
        }
        "get_project_summaries_today" => {
            let summaries = db.get_all_project_summaries_today()?;
            let summary = format!("{} project summaries", summaries.len());
            Ok((serde_json::to_string(&summaries).unwrap_or_default(), summary))
        }
        "get_project_rules" => {
            let project_id = args["project_id"].as_i64()
                .ok_or("Missing required parameter: project_id")?;
            let rules = db.get_project_rules(project_id)?;
            let summary = format!("{} rules for project {}", rules.len(), project_id);
            Ok((serde_json::to_string(&rules).unwrap_or_default(), summary))
        }
        "get_all_rules" => {
            let rules = db.get_all_rules()?;
            let summary = format!("{} rules total", rules.len());
            Ok((serde_json::to_string(&rules).unwrap_or_default(), summary))
        }
        "get_untagged_summary" => {
            let (total, by_app) = db.get_untagged_summary()?;
            let result = json!({ "total": total, "by_app": by_app });
            let summary = format!("{} untagged activities", total);
            Ok((result.to_string(), summary))
        }
        "get_project_activities" => {
            let project_id = args["project_id"].as_i64()
                .ok_or("Missing required parameter: project_id")?;
            let limit = args["limit"].as_i64().unwrap_or(50);
            let offset = args["offset"].as_i64().unwrap_or(0);
            let activities = db.get_project_activities(project_id, limit, offset)?;
            let summary = format!("{} activities for project {}", activities.len(), project_id);
            Ok((serde_json::to_string(&activities).unwrap_or_default(), summary))
        }
        "list_tasks" => {
            let archived = args["archived"].as_bool().unwrap_or(false);
            let tasks = db.list_tasks(archived)?;
            let label = if archived { "archived" } else { "active" };
            let summary = format!("{} {} tasks", tasks.len(), label);
            Ok((serde_json::to_string(&tasks).unwrap_or_default(), summary))
        }
        "get_top_windows_today" => {
            let rows = db.get_top_windows_today()?;
            let result: Vec<Value> = rows.iter().map(|(title, secs)| {
                json!({ "title": title, "duration_secs": secs })
            }).collect();
            let summary = format!("{} windows", result.len());
            Ok((serde_json::to_string(&result).unwrap_or_default(), summary))
        }
        "get_recent_events" => {
            let limit = args["limit"].as_i64().unwrap_or(20);
            let events = db.get_recent_events(limit)?;
            let summary = format!("{} events", events.len());
            Ok((serde_json::to_string(&events).unwrap_or_default(), summary))
        }
        "get_all_settings" => {
            let pairs = db.get_all_settings("default")?;
            // Filter out the API key for security
            let filtered: Vec<(String, String)> = pairs.into_iter()
                .map(|(k, v)| {
                    if k == "ai_api_key" {
                        (k, "***REDACTED***".to_string())
                    } else {
                        (k, v)
                    }
                })
                .collect();
            let result: Value = filtered.iter()
                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                .collect::<serde_json::Map<String, Value>>()
                .into();
            let summary = format!("{} settings", filtered.len());
            Ok((result.to_string(), summary))
        }
        _ => {
            let err = json!({ "error": format!("Unknown tool: {}", name) });
            Ok((err.to_string(), format!("unknown tool: {}", name)))
        }
    }
}
