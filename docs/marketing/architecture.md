# Architecture one-pager

```
Tauri 2 app (tray only)
├─ te-core (Rust, tested on every platform)
│   text cleanup → classify → chunk → prompt → engine → output filter
│   readability grade · language guard (GBNF) · meaning guard
│   llama-server process manager · streaming client · downloads · settings
├─ te-win (Rust, Win32)
│   UI Automation capture · clipboard fallback · double Ctrl+C listener
│   non-activating popup · click-outside hook · job object · memory size
├─ src-tauri: hotkey, popup placement, commands, tray
├─ React + TypeScript (Atlas design): reading card, settings, first run
└─ llama.cpp llama-server (pinned Windows CPU build, SHA-256 checked)
    on 127.0.0.1 with a random port and per-launch API key
```

**One request:** the hotkey reads the selection (UI Automation, else a clipboard-safe
copy) → the text is repaired and classified → its language is detected and the allowed
scripts decided → long text is split at paragraph boundaries → each part streams through
llama-server with a grammar that blocks other scripts → the output filter hides chatter
("Sure! Here is…") as it streams → the meaning guard and reading grade are shown on the
card.

**Why a separate llama-server process:** a model crash can't take the app down, models
swap by restarting it, llama.cpp upgrades are a file swap, and the prompt cache makes
repeated instructions nearly free. A Windows job object ensures it never outlives the app.

**Privacy:** no telemetry; the only network access is downloads the person starts.
