# ADR-002: AI-Assisted Project Suggestions

**Status:** Proposed
**Date:** 2026-03-01

## Context

The current Auto-Suggest feature in Projects uses **deterministic heuristics** to propose projects:
- Extract VS Code workspace names from window titles
- Detect Slack usage
- Group remaining apps by raw `app_name` (Chrome, WindowsTerminal, Reolink, etc.)

This produces useful but shallow groupings. The results are per-application, not per-project. A real "project" is a cross-app concept — working on "aiHelper" involves VS Code, Chrome (docs, Stack Overflow, GitHub), Terminal (cargo build, git), and Slack (team channel). The heuristic approach can't discover these relationships.

**What we want:** An LLM analyzes the user's actual activity patterns and proposes meaningful project definitions with JSONata rules that span multiple apps.

## Decision

**Add an AI-assisted suggestion mode that sends anonymized activity summaries to the configured LLM and receives back project definitions with JSONata rules.**

### How It Works

1. **Collect activity summary** — query the last 7 days of `window_activity` data, grouped and aggregated (not raw keystrokes or content):
   ```
   app_name | window_title patterns | total_duration
   Code     | "*.rs - aiHelper - Visual Studio Code" | 4h 30m
   Code     | "*.ts - voyce-web - Visual Studio Code" | 2h 15m
   chrome   | "GitHub - onexdata/aiHelper"            | 45m
   chrome   | "rust book"                             | 30m
   Terminal | "cargo build", "git push"               | 1h 10m
   Slack    | "#aihelper-team"                        | 20m
   ```

2. **Build a prompt** — send the summary to the LLM with instructions:
   - "Given this activity data, suggest meaningful project groupings"
   - "Each project should have a name, description, and one or more JSONata rules"
   - "Rules match against `{app_name, window_title}` objects"
   - "Projects should be cross-app where patterns suggest it (e.g., same repo name across Code + Chrome + Terminal)"
   - Include the JSONata syntax reference and examples of valid expressions

3. **Parse structured response** — the LLM returns JSON with project definitions:
   ```json
   [
     {
       "name": "aiHelper",
       "description": "Desktop productivity tool — Tauri/Rust/JS",
       "rules": [
         "app_name = \"Code\" and $contains(window_title, \"aiHelper\")",
         "$contains($lowercase(window_title), \"onexdata/aihelper\")",
         "app_name = \"Terminal\" and $contains(window_title, \"aiHelper\")"
       ]
     }
   ]
   ```

4. **Present for review** — show each suggestion in the UI with preview of which existing activity it would match (dry-run the JSONata rules against untagged data). User can accept, edit, or reject each one.

### Privacy Design

- **No raw keystrokes** are sent — only `app_name`, window title patterns (deduplicated), and durations
- **Window titles are summarized** — group similar titles, show patterns not exact strings (e.g., `"*.rs - aiHelper - Visual Studio Code"` not every filename)
- **User controls the LLM** — uses their configured provider (OpenRouter, local Ollama, etc.)
- **Optional** — the deterministic Auto-Suggest remains as the default, AI-assisted is an explicit "Smart Suggest" action

### API Design

**Backend (Rust):**
- `get_activity_summary_for_ai(days: i32) → ActivitySummary` — aggregated, anonymized activity data
- `parse_ai_project_suggestions(response: &str) → Vec<ProjectSuggestion>` — parse LLM JSON response

**Frontend (JS):**
- Calls `get_activity_summary_for_ai` to build the prompt
- Sends to LLM via existing `send_chat_message` infrastructure (or a dedicated non-streaming invoke)
- Parses response and shows preview UI with match counts per rule

### UI Flow

```
[Auto-Suggest]  [Smart Suggest ✨]

┌─ Smart Suggest ─────────────────────────┐
│ Analyzing 7 days of activity...         │
│                                          │
│ ┌─ Suggestion: aiHelper ───────────────┐│
│ │ Desktop productivity tool             ││
│ │                                       ││
│ │ Rules:                                ││
│ │  ✓ Code + "aiHelper" (142 matches)   ││
│ │  ✓ Chrome + "onexdata" (38 matches)  ││
│ │  ✓ Terminal + "aiHelper" (67 matches) ││
│ │                                       ││
│ │ [Accept]  [Edit Rules]  [Skip]        ││
│ └───────────────────────────────────────┘│
│                                          │
│ ┌─ Suggestion: VoyceMe ────────────────┐│
│ │ ...                                   ││
│ └───────────────────────────────────────┘│
└──────────────────────────────────────────┘
```

### Prompt Engineering

The system prompt should:
- Explain JSONata syntax with examples
- Specify the input schema (`{app_name: string, window_title: string}`)
- Request structured JSON output
- Encourage cross-app grouping (same project name across Code, browser, terminal)
- Avoid over-grouping (don't merge unrelated browser tabs into one project)
- Handle the "uncategorizable" case (some activity is genuinely miscellaneous)

### Integration with Tips

Long-term, the Tips tab can proactively suggest: *"You've been working in Code on 'platform-factory' for 3 hours but it's not a project yet. Create it?"* — this is a natural extension of AI-assisted suggestions running periodically rather than on-demand.

## Alternatives Considered

### Clustering algorithms (k-means on window titles)
- **Rejected:** Requires NLP embeddings, significant complexity, and the results still need human-readable names. An LLM does clustering + naming + rule generation in one step.

### Pre-built rule templates
- **Rejected as primary:** Too rigid. Every user's workflow is different. Templates could supplement AI suggestions but can't replace them.

### Always-on AI categorization (tag every activity row via LLM)
- **Rejected:** Too expensive (API costs per keystroke batch), too slow, and violates the "rules engine" architecture. The LLM's job is to write the rules, not to evaluate them — JSONata handles evaluation locally at zero cost.

## Consequences

- Smart Suggest requires a configured AI provider (API key) — falls back to deterministic Auto-Suggest if unconfigured
- LLM quality affects suggestion quality — works best with capable models (GPT-4o, Claude, etc.)
- Window title patterns may reveal project/repo names to the AI provider — documented in privacy section
- The JSONata rules generated by the LLM might need user editing — the preview with match counts helps validate before accepting
- This establishes the pattern for other AI-assisted features (Tips, smart task creation, meeting summaries)
