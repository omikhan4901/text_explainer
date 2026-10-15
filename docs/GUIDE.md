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
3. **Tabs** (`trigger.rs`, `crates/core/src/sources.rs`). The `open` event lists the
   card's web tabs (`sources::tabs_for`: enabled sources for words or for passages, none
   while Offline only is on) and the selection's kind. For one word or a short term the
   WordNet entry (`crates/core/src/dictionary.rs`) follows at once. The model doesn't
   start (`ai: "deferred"`) unless there is nothing else to show (`"running"`); with no
   model it's `"off"`. Opening "Explain with AI" makes the card call `explain_now`, which
   runs step 4 for that request if it's still the current one; a new reading level calls
   `explain_again` (same request, the card stays on its tab).
4. **Web tabs** (`popup.rs`). The card measures its content area and calls `web_show`
   (async: creating a web view from a sync command can deadlock on Windows), which builds
   the page address from the source's template (`sources::build_url`: the selection is
   percent-encoded and can't change the host) and shows one child web view (`popup-web`,
   Tauri's `unstable` multi-webview) over that area: incognito, never focused on its own,
   http(s) navigation only, new windows sent to the default browser. It gets no app
   permissions (it's remote content). Other tabs call `web_hide`; closing the card closes
   the web view to free its memory. The main window's Sources page edits the list
   (`settings.sources`, `offline_only`, `translate_to`).
5. **Pipeline** (`crates/core/src/explain.rs`): repair the text (`text.rs`), classify it,
   detect its language (`language.rs`), decide the allowed scripts and build the GBNF
   grammar, split long text into parts, then for each part: build the prompt
   (`prompt.rs`), stream from llama-server (`engine/`), clean the stream (`output.rs`),
   check the script; on drift, retry once with a stricter instruction. Finally the
   meaning check (`meaning.rs`) and reading grade (`readability.rs`, English only).
6. **Card updates** (`src/popup/state.ts` reduces the events and the tab choice;
   `Popup.tsx` renders them). Tabs (`tabs()` in `state.ts`): web searches, then the
   dictionary for words, then "Explain with AI", pinned at the right of the tab row. With
   web tabs every tab keeps the page's height so switching doesn't resize the card;
   otherwise the window is resized to the card as text streams in. The level switch and
   grade sit in the AI tab's footer on passages.

## The interface

Design tokens are in `src/styles/tokens.css`: neutral surfaces, near-black controls
(`--control`) and one accent (`--accent`, from Settings: Ink by default, or Plum, Saffron,
Garnet) that marks only where you are (open tab, links, switches, focus). Type is Geist
(UI and the default reading font) with Geist Mono for small figures; the reading font can
be Atkinson Hyperlegible or Source Serif. Icons are Phosphor. Shared pieces are in
`src/components/ui/` (`Section` and `Group` for settings pages, `Field` and `Switch` rows,
`Segmented`, `Kbd`, `Button`). Every pair of colours meets WCAG 2.2 AA in light and dark,
checked by `e2e/a11y.spec.ts`.

## The model engine

`crates/core/src/engine/server.rs` starts the bundled `llama-server.exe` on 127.0.0.1
with a random port and a random API key, from its own folder (for its DLLs), with no
console window, and logs to `%LOCALAPPDATA%\dev.omikhan.textexplainer\logs`. A Windows job
object (`te_win::kill_with_app`) ensures it dies with the app. It's started on the first
request, restarted if it crashes, and stopped after the idle time in Settings.

## Models already on the computer

`crates/core/src/discover.rs` looks for GGUF files the person already has: LM Studio
(`~/.lmstudio/models`, `~/.cache/lm-studio/models`), Jan, GPT4All, the Hugging Face cache,
Downloads (top level), and Ollama (its manifests name each model; the model layer is a
GGUF blob, checked to be a 64-hex-digit digest inside `blobs`). Files must start with the
GGUF magic bytes and be at least 50 MB; vision projectors and later parts of split models
are skipped, and the walk is bounded in depth and entries. Each result gets the same
memory fit as the catalog (size plus half). The Model page and first run list them
(`find_models`); "Use" sets a `file` model.

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
`popup.html?demo=passage|word|word_offline|check|no_model|loading|error`, plus
`&tab=explain` (or `dictionary`, `web:bing`) to open a tab and `&offline=1` for Offline
only; a web tab shows its loading placeholder there (the page itself needs the app). And
`index.html?state=ready&page=model`. `scripts/wincheck.sh clippy -p text-explainer`
type-checks the Windows app from Linux.
