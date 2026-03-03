# ADR-001: AI System Choice

**Status:** Accepted
**Date:** 2026-03-01

## Context

aiHelper needs an AI backend that supports:
1. **LLM chat** — text generation with streaming
2. **Multimodal** — vision (send images/screenshots to AI)
3. **Diffusers** — image generation (Stable Diffusion, FLUX, DALL-E)
4. **Vocoders** — text-to-speech / speech synthesis
5. **MCP** — Model Context Protocol for tool use (filesystem, web, databases, etc.)

These are fundamentally different capabilities. No single provider handles all five. The question is: what architecture unifies them?

## Decision

**Use OpenRouter as the primary AI gateway, with an OpenAI-compatible client in Rust.**

### Why OpenRouter

- **Single API, 300+ models** — Claude, GPT-4o, Gemini, Llama, Mistral, DeepSeek, etc.
- **Fully OpenAI-compatible** — same endpoint format (`/v1/chat/completions`), same SDKs
- **Multimodal built-in** — vision models accept images natively
- **Image generation** — supports DALL-E, Stable Diffusion, FLUX via `/v1/images/generations`
- **Pay-per-token** — no subscriptions, use any model on demand
- **Fallback routing** — if one provider is down, OpenRouter routes to another
- **`ai_base_url` override** — can point to local Ollama, LM Studio, or any OpenAI-compatible server instead

### Capability Mapping

| Capability | How | Endpoint |
|------------|-----|----------|
| **LLM chat** | OpenRouter → any model | `POST /v1/chat/completions` |
| **Multimodal** | OpenRouter → vision models (Claude, GPT-4o, Gemini) | Same endpoint, image in message content |
| **Diffusers** | OpenRouter → image models, or separate endpoint | `POST /v1/images/generations` |
| **Vocoders/TTS** | OpenAI TTS API or local piper-tts | `POST /v1/audio/speech` |
| **MCP** | Client-side — independent of LLM provider | Rust MCP client SDK |

### Why Rust (not frontend JS)

1. **API key security** — keys never leave the Rust process, never exposed to webview
2. **MCP client** — MCP servers are spawned as subprocesses, needs native process management
3. **Streaming control** — SSE parsing in Rust, forwarded to frontend via Tauri events
4. **Local model support** — can shell out to Ollama/whisper.cpp from Rust
5. **Platform APIs** — future TTS/STT will use native OS APIs from Rust

### Implementation Plan

**Rust side (`src-tauri/src/ai.rs`):**
- `reqwest` for HTTP + SSE streaming to OpenRouter
- OpenAI-compatible request/response structs (serde)
- Stream chunks forwarded to frontend via `app.emit("chat-chunk", ...)`
- Config reads `ai_provider`, `ai_api_key`, `ai_base_url` from AppConfig

**Frontend side:**
- Chat UI in overlay (message list + input)
- Listens for `chat-chunk` events, appends to current message
- Sends user messages via `invoke("send_chat_message", { message, model })`

**MCP (later, Phase 6):**
- Rust MCP client using `rmcp` crate or custom implementation
- MCP servers configured in config.toml
- Tools exposed to LLM via OpenAI function calling format

## Alternatives Considered

### Direct provider SDKs (Anthropic SDK, OpenAI SDK, etc.)
- **Rejected:** Lock-in to one provider. User wants model flexibility.

### Frontend-only (call APIs from JS)
- **Rejected:** API keys exposed in webview. No MCP support. Can't access platform APIs.

### LangChain / LlamaIndex (Python)
- **Rejected:** Adds Python runtime dependency. Tauri is Rust-native. Unnecessary abstraction.

### Ollama-only (local models)
- **Rejected as default:** Great for local use but limits model quality. Supported via `ai_base_url` override — user can point to `http://localhost:11434/v1` for Ollama.

## Consequences

- All AI calls go through a single OpenAI-compatible Rust client
- Switching models is a config change, not a code change
- MCP is a separate subsystem that feeds tools into the LLM's function calling
- Image gen and TTS are separate endpoints but same auth pattern
- Users can self-host by changing `ai_base_url` to Ollama/LM Studio
