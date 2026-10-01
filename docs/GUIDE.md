# How Text Explainer works

A guide to the code for anyone changing it. Keep it true when you change how things work.

## The pieces

| Folder | What it is | Runs on |
|---|---|---|
| `crates/core` (`te-core`) | All logic: text repair, language and meaning guards, prompts, the model engine, dictionary, downloads, settings | Anywhere (tested on Linux and Windows) |
| `crates/win` (`te-win`) | Windows integration: reading the selection, the clipboard, the popup window, mouse hook, job object | Windows (stubs elsewhere) |
| `crates/eval` (`te-eval`) | The model evaluation (see `docs/models.md`) | Linux CI |
| `src-tauri` | The app: tray, hotkey, popup placement, commands | Windows |
| `src` | The interface: `popup/` (reading card) and `main/` (settings, first run) | WebView2 |

## One request, step by step

1. **Trigger** (`src-tauri/src/trigger.rs`). The hotkey runs `te_win::capture_selection`
   on a background thread: UI Automation first (`crates/win/src/imp/uia.rs`, 450 ms
   limit), then, if allowed, the clipboard fallback (`clipboard.rs`: snapshot every
   format, release the hotkey's modifiers, send Ctrl+C, wait for the clipboard sequence
   number to change, read, restore). Double Ctrl+C comes from a clipboard listener in the
   same file and hands over the copied text directly.
2. **Card opens** (`src-tauri/src/popup.rs`). Placed by the selection's screen rectangle
   (or the mouse), on the right monitor, on the side with room (`crates/win/src/placement.rs`),
   shown with `SWP_NOACTIVATE` so the reading app keeps focus. Esc is registered as a
   global shortcut only while it's open; a low-level mouse hook closes it on clicks
   elsewhere.
3. **Dictionary** (`crates/core/src/dictionary.rs`). For one word or a short term, the
   WordNet entry is sent to the card at once.
4. **Pipeline** (`crates/core/src/explain.rs`): repair the text (`text.rs`), classify it,
   detect its language (`language.rs`), decide the allowed scripts and build the GBNF
   grammar, split long text into parts, then for each part: build the prompt
   (`prompt.rs`), stream from llama-server (`engine/`), clean the stream (`output.rs`),
   check the script; on drift, retry once with a stricter instruction. Finally the
   meaning check (`meaning.rs`) and reading grade (`readability.rs`, English only).
5. **Card updates** (`src/popup/state.ts` reduces the events; `Popup.tsx` renders them).
   The window is resized to the card as text streams in.

## The model engine

`crates/core/src/engine/server.rs` starts the bundled `llama-server.exe` on 127.0.0.1
with a random port and a random API key, from its own folder (for its DLLs), with no
console window, and logs to `%LOCALAPPDATA%\dev.omikhan.textexplainer\logs`. A Windows job
object (`te_win::kill_with_app`) ensures it dies with the app. It's started on the first
request, restarted if it crashes, and stopped after the idle time in Settings.

## Data on disk

- Settings: `%APPDATA%\dev.omikhan.textexplainer\settings.json` (saved atomically; an
  unreadable file is kept as `settings.broken.json`).
- Models: `%LOCALAPPDATA%\dev.omikhan.textexplainer\models` (downloads resume from
  `.part` files and are checked against SHA-256 before use).
- Logs: `%LOCALAPPDATA%\dev.omikhan.textexplainer\logs` (`app.log`, `llama-server.log`).

## Updating pinned things

- **llama.cpp**: run the probe workflow, update `VERSION` and the hashes in
  `scripts/fetch-llama.py`, then run the evaluation workflow before releasing.
- **Models**: the probe lists file sizes and SHA-256; update `crates/core/src/models.rs`.
- **Dictionary**: `npm` dev dependencies `wordnet-db` and `wink-lexicon`;
  `scripts/build-dictionary.py` rebuilds it.

## Developing without Windows

`npx vite` serves the interface in a browser against a mock (`src/lib/mock.ts`):
`popup.html?demo=passage|word|word_offline|check|no_model|loading|error`, and
`index.html?state=ready&page=model`. `scripts/wincheck.sh clippy -p text-explainer`
type-checks the Windows app from Linux.
