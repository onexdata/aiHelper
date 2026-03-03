# aiHelper — Roadmap / TODO

## Phase 0 — Scaffold [DONE]
- [x] Tauri v2 app with system tray icon + context menu
- [x] Global hotkey (Ctrl+Shift+Space) toggles transparent overlay
- [x] Hot reload via Vite

## Phase 1 — Foundation
- [x] Centralized config system (AppConfig struct + TOML)
- [x] SQLite database (rusqlite bundled, WAL mode, migrations)
- [x] Tauri commands for config + event CRUD
- [ ] AI chatbot — OpenRouter integration (OpenAI-compatible, streaming)
- [ ] Chat UI in overlay (message input, streamed response display)

## Phase 2 — Activity Tracking
- [ ] Window focus tracking (Win32 API → SQLite log)
- [ ] Keyboard activity logging (low-level hooks → SQLite log)

## Phase 3 — Screen Intelligence
- [ ] OCR / screen text extraction (WinRT OCR API on Windows)

## Phase 4 — Voice
- [ ] Voice-to-text / mic transcription (whisper-rs, local Whisper model)
- [ ] Voice-to-keyboard injection (transcription → simulated keystrokes)

## Phase 5 — Advanced
- [ ] Network traffic logging (libpcap/npcap)
- [ ] Screen recording (DXGI Desktop Duplication on Windows)
- [ ] Recording scrubber/playback (HTML5 video + timeline sync)

## Phase 6 — AI Expansion
- [ ] Image generation via OpenRouter diffusion models
- [ ] Text-to-speech via OpenAI-compatible TTS endpoint
- [ ] MCP client — connect to MCP servers for tool use (filesystem, web, etc.)
- [ ] MCP server — expose aiHelper capabilities to other AI tools

---

## Architecture Decisions
See [adrs/](./adrs/) for Architecture Decision Records.

| ADR | Title | Status |
|-----|-------|--------|
| [001](./adrs/001-ai-system-choice.md) | AI System Choice — OpenRouter | Accepted |
