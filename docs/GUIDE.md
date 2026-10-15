# How Text Explainer works

A guide to the code for anyone changing it. Keep it true when you change how things work.

## The pieces

| Folder | What it is | Runs on |
|---|---|---|
| `crates/core` (`te-core`) | All logic: text repair, language and meaning guards, prompts, the model engine, dictionary, downloads, settings, the Word book and history | Anywhere (tested on Linux and Windows) |
| `crates/win` (`te-win`) | Windows integration: reading the selection, the clipboard, the popup window, mouse hook, job object, Listen (SAPI voice) | Windows (stubs elsewhere) |
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

## The Word book, history and Listen

- **Saving** (`src/popup/Popup.tsx`, `WordActions`). The card's header has Save (a
  bookmark, `aria-pressed`) for any selection whose header isn't cut short (the header
  holds 120 characters, the book takes terms up to 200), and Listen for words and short
  terms. On open the card asks `is_saved`. Saving stores the term (a word without its
  punctuation, `bareTerm`), its kind, the first dictionary sense and the AI answer if it
  has finished; the app adds the paragraph around the selection as context when UI
  Automation reported one. If the AI answer finishes after saving, the card saves again
  and the answer becomes the explanation. Pressing Save again removes the word.
- **History** (`trigger.rs` calls `words::record` as the card opens on a selection). Only
  when `keep_history` is on (default); the query is kept as one line of at most 300
  characters. Each insert prunes lookups older than `history_days` (1 to 365, default 30)
  and keeps the newest 2,000.
- **Storage** (`crates/core/src/history.rs`): `history.sqlite`, WAL journal,
  `foreign_keys` on, a 5 second busy timeout. The schema version is `PRAGMA user_version`
  and `migrate` brings it up step by step, each step in one transaction (running it again
  changes nothing). Tables are `STRICT` with `CHECK`s for the kinds and lengths:
  `lookups (id, query, kind, created_at)` indexed on `created_at`, and
  `saved (id, term, term_key UNIQUE, kind, definition, explanation, context, created_at,
  updated_at)`. `term_key` is the term trimmed, lowercased and with whitespace collapsed,
  so "Ubiquitous" and " ubiquitous" are one entry (no Unicode normalisation: composed and
  decomposed accents are different keys). Saving is an upsert on `term_key` that keeps
  `created_at` and replaces only the texts it is given. Search is a bound `LIKE` with
  `%`, `_` and `\` escaped. Limits are applied once, at the boundary: terms up to 200
  characters (longer is refused), definition, explanation and context cut to 8,000.
  A file that isn't a database or fails `quick_check` is renamed to
  `history.broken.sqlite` and a new one starts; a newer schema or an unwritable folder
  leaves the file alone and the book lives in memory for that run. Times are unix
  milliseconds.
- **Commands** (`src-tauri/src/words.rs`): `save_word`, `remove_word`, `is_saved`,
  `list_words` (search, pages of at most 200), `list_history`, `clear_history`,
  `export_words`, `speak`. Each runs on a blocking thread with the book's mutex held only
  inside it.
- **Words page** (`src/main/pages/Words.tsx`): Saved (search, Listen, Remove, export) and
  History (Keep history, how long, the list, Clear history after a confirm; turning
  history off offers to clear what is kept). Lists reload when the window is focused.
- **Export** (`ExportFormat` in `history.rs`). The save dialog picks the file
  (`dialog:allow-save`); the app writes it next to the target and renames it into place.
  CSV is RFC 4180 (every field quoted, CRLF), starts with a UTF-8 byte order mark so
  Excel reads Bangla, and prefixes cells starting with `=`, `+`, `-`, `@`, tab or CR with
  `'` so a spreadsheet never runs them. The Anki file is tab-separated with
  `#separator:tab` and `#html:true` headers: front is the term, back is the definition and
  explanation, HTML-escaped with line breaks as `<br>`.
- **Listen** (`crates/win/src/imp/speech.rs`): one worker thread owns a SAPI `SpVoice`
  (COM initialised on that thread) and speaks each text with `SPF_ASYNC |
  SPF_PURGEBEFORESPEAK | SPF_IS_NOT_XML`, so a new word interrupts the last and text is
  never read as SAPI markup. Windows' own voices, offline; at most 200 characters.

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
- Word book and history: `%LOCALAPPDATA%\dev.omikhan.textexplainer\history.sqlite`
  (an unusable file is kept as `history.broken.sqlite`).

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
`index.html?state=ready&page=model` (or `page=words`, `&tab=history`); the mock
keeps a small Word book in memory (`WordBook` in `src/lib/words.ts`, the same rules as
`history.rs`). `scripts/wincheck.sh clippy -p text-explainer`
type-checks the Windows app from Linux.
