---
name: dev
description: Build and run the aiHelper Tauri app in dev mode
disable-model-invocation: true
allowed-tools: Bash
argument-hint: [--release]
---

## Start aiHelper Dev Server

Run the Tauri development server. Ensure cargo is in PATH first.

```bash
cd /c/code/onexdata/aiHelper && export PATH="$HOME/.cargo/bin:$PATH" && cargo tauri dev
```

If `$ARGUMENTS` contains `--release`, run a release build instead:

```bash
cd /c/code/onexdata/aiHelper && export PATH="$HOME/.cargo/bin:$PATH" && cargo tauri build
```

Run the command in the background so the user can keep chatting.
