# aiHelper

Cross-platform AI-powered desktop productivity assistant. Lives in your system tray, appears as a floating overlay on `Ctrl+Shift+Space`.

Built with [Tauri v2](https://v2.tauri.app/), Rust, and Vanilla JS.

## Features

- **AI Chat** — Streaming conversations via OpenRouter (OpenAI-compatible)
- **System Tray** — Runs in the background, toggle overlay with a global hotkey
- **Projects** — Organize work with AI-powered project suggestions
- **Tasks** — Quick task management with auto-archive
- **Tips** — AI-generated productivity tips on a timer
- **Stats** — Live WPM, keystroke counts, mouse distance, top windows
- **Settings** — Configure AI provider, model, hotkey, appearance

## Prerequisites

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://rustup.rs/) stable
- Platform dependencies:
  - **Windows**: No extra deps
  - **macOS**: Xcode Command Line Tools
  - **Linux**: `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`

## Getting Started

```bash
# Install frontend dependencies
npm install

# Run in development mode (hot reload)
npm run tauri dev

# Build for production
npm run tauri build
```

## Configuration

On first launch, open **Settings** to configure:

| Setting | Example |
|---------|---------|
| Provider | `openrouter` |
| API Key | `sk-or-...` |
| Base URL | `https://openrouter.ai/api/v1` |
| Default Model | `openai/gpt-4o-mini` |

Config is stored at:
- **Windows**: `%APPDATA%/com.onexdata.aihelper/config.toml`
- **macOS**: `~/Library/Application Support/com.onexdata.aihelper/config.toml`
- **Linux**: `~/.config/com.onexdata.aihelper/config.toml`

## Downloads

Pre-built installers are available on the [Releases](https://github.com/onexdata/aiHelper/releases) page:

| Platform | Formats |
|----------|---------|
| Windows | `.msi`, `.exe` (NSIS) |
| macOS | `.dmg` (universal — Intel + Apple Silicon) |
| Linux | `.deb`, `.rpm`, `.AppImage` |

## Roadmap

See [documentation/todo.md](documentation/todo.md) for the full roadmap.

## License

MIT
