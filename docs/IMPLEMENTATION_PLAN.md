# Text Explainer: implementation plan

Approved by the owner on 2026-10-01. Decisions: Atlas design, Windows first (MVP before
other platforms), "select to show" off by default, a balanced model (between the fastest
and the best) with a strict language guard.

## 1. The product

Select text you're reading anywhere, press one key, and a clearer version appears next to
the cursor. Fully offline, on an 8 GB CPU-only laptop.

The main job is **making text readable**, not dictionary lookups.

| Selection | What you get |
|---|---|
| A sentence, a paragraph, a few pages | A readable rewrite that keeps the meaning: short sentences, plain words, acronyms spelled out, structure kept. Streams in. |
| Text full of jargon | The rewrite, plus the hard terms with a short meaning that fits this context |
| One word or a short phrase | An instant dictionary entry (local SQLite WordNet) and a one-line "here it means…" from the model |

- **Reading levels:** *Simpler* (about grade 6), *Plain* (default, about grade 8–9),
  *Clearer* (same level, better structure). A badge shows the grade before and after.
- **Meaning guard:** a deterministic check that numbers, dates, money, percentages and names
  in the source still appear in the rewrite. Missing ones are flagged in the card.
- **Language guard:** small models (Qwen especially) sometimes switch to Chinese. Every
  request carries a GBNF grammar that forbids scripts that are neither in the source nor in
  the chosen output language, so it can't happen at the token level. The result is checked
  again (for endpoints without grammar support) and retried once with a stricter prompt.
- **PDF cleanup:** text copied from PDFs is repaired before anything else (hyphenation at
  line ends, hard-wrapped lines, ligatures, soft hyphens).

Later (after the MVP): snip + OCR for text you can't select, history and topic word lists,
"select to show" dot (off by default), macOS and Linux, Bangla UI.

## 2. Model

Balanced tier by default. Candidates, all GGUF Q4_K_M through llama.cpp, thinking off:

| Tier | Model | RAM (approx.) |
|---|---|---|
| **Balanced (default candidates)** | Qwen3.5-4B, Gemma 4 E4B | 3–5 GB |
| Fast | Qwen3.5-2B, Gemma 4 E2B | ~2 GB |
| Best | Qwen3.5-9B | ~6.5 GB (16 GB machines) |
| Own | any GGUF file, or a local OpenAI-compatible endpoint (Ollama, LM Studio) | |

The default is picked by measurement, not blog posts: `crates/eval` runs a corpus of
passages (legal, medical, academic, technical, news, ESL) through each model and scores
reading-grade drop, meaning preservation (guard), language drift, preamble/chatter,
time to first token, tokens per second and peak RAM. It runs in a manual GitHub Actions
workflow (standard runners are free for public repos). Results go in `docs/models.md`.

People can take full control in Settings: the catalog (with "fits your RAM" hints),
resumable downloads checked against SHA-256, importing any GGUF, temperature, top-p,
context size, threads, idle unload, editable prompts per level, and a speed test.

## 3. Architecture (Windows MVP)

```
Tauri 2 app (tray only, no taskbar button)
├─ crates/core (te-core): text, readability, guard, language guard, prompts,
│   engine (llama-server process + streaming client), dictionary, models, settings
├─ src-tauri: tray, global hotkey, selection capture, popup window, IPC
├─ src (React + TS + Tailwind v4 + Radix, Atlas tokens)
│   ├─ popup.html  reading card (transparent, always on top, never focused)
│   └─ index.html  settings, models, first run
└─ llama.cpp llama-server (Windows CPU build) bundled as a resource,
    started on 127.0.0.1 with a random port and a per-launch API key,
    no console window, unloaded after idle
```

- **Selection capture (Windows):** UI Automation `TextPattern.GetSelection` on the focused
  element first. Fallback: wait for the hotkey's modifiers to be released, snapshot the
  clipboard (all formats), send Ctrl+C, wait for the clipboard sequence number to change,
  read the text, restore the clipboard exactly.
- **Popup:** `WS_EX_NOACTIVATE` + `SW_SHOWNOACTIVATE`, so the reading app keeps focus and the
  caret. Placed by the selection's bounding rectangle (UIA) or the cursor, clamped to the
  monitor's work area with DPI scaling. Esc (a global shortcut registered only while the
  card is shown) and a click outside (low-level mouse hook while shown) close it, unless
  pinned.
- **Long text:** split at paragraph boundaries, streamed chunk by chunk; the system prompt
  is cached by llama-server across requests.
- **Privacy:** no telemetry, no network except downloads the person starts. Strict CSP.

## 4. Milestones (Windows MVP)

Each milestone ships on its own, passes `scripts/check.sh` and the Windows CI job, and is
pushed to `main`.

| # | Milestone | Done when |
|---|---|---|
| 0 | Foundations | Scaffold, docs, check script, CI (Linux checks + Windows build with installer artifact), Atlas tokens, tray app |
| 1 | Core | Text cleanup and classification, readability, meaning guard, language guard grammar, prompts, llama-server client and process manager (tested against a fake server) |
| 2 | Capture + popup | Hotkey, UIA capture with clipboard-safe fallback, non-focusing popup with streaming rewrite, levels, copy, pin, Esc / click outside |
| 3 | Models + first run | Catalog, downloads with resume and SHA-256, import GGUF, settings, first-run flow (RAM check, download, hotkey check, "try it") |
| 4 | Words | WordNet SQLite dictionary, lemmatizer, single-word card, terms with in-context meanings |
| 5 | Eval | Harness + corpus + manual CI workflow; default model and prompts chosen from results; `docs/models.md` |
| 6 | Polish + release | Performance budgets, accessibility, sidecar crash recovery, NSIS installer from CI, README with screenshots, marketing kit refreshed |

The marketing kit (`docs/marketing/`) starts at milestone 1 and is updated at every
milestone, claiming only what is built and measured.

**Performance budgets:** popup visible ≤ 100 ms after the hotkey, dictionary ≤ 50 ms,
app ≤ 150 MB RAM without a model loaded, first token ≤ 2 s for a paragraph on a 4-core
laptop with the default model.

## 5. Testing

- Rust unit tests for every core module; engine tests against a fake llama-server.
- Vitest + Testing Library for the front end, with Tauri IPC mocked.
- Windows-only code (capture, popup window styles) is built, linted and unit-tested on the
  Windows CI runner; the owner checks real-world behaviour with `docs/testing/windows.md`.
- The real-model eval runs on demand in CI, never on every push.

## 6. Costs

None. Public repo on standard GitHub runners, free model downloads from Hugging Face,
unsigned installers (Windows SmartScreen will warn until the owner decides to buy signing).
