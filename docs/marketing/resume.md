# Resume bullets

Pick 3–4. Every number is measured (see `docs/PROGRESS.md`). Don't claim users or
downloads until they exist.

**Plainly**: offline "make this readable" desktop app (Rust, Tauri, React/TypeScript, llama.cpp) · 2026

- Built a Windows desktop app that rewrites any selected text at a chosen reading level
  using a small language model running locally through llama.cpp, with no data leaving
  the device.
- Eliminated language drift in small-model output by generating a GBNF grammar per
  request that blocks unexpected writing systems at sampling time, with a verified
  retry path for servers without grammar support.
- Designed a deterministic meaning check that compares numbers, dates, acronyms and names
  between source and rewrite and flags omissions or invented facts in the UI.
- Implemented selection capture through Windows UI Automation with a clipboard fallback
  that snapshots and restores every clipboard format, and a popup that opens beside the
  text without stealing focus from the app being read.
- Built an evaluation harness that runs a 28-passage corpus through candidate models in
  parallel GitHub Actions jobs; chose Gemma 4 E2B as default (about 6 s median per rewrite
  on a 4-core CPU, 3.9 GB, fewest dropped facts), replacing a slower first choice, and
  sized the engine's prompt cache by measurement (first words 4x faster than with it off).
- Wrote 201 Rust tests, 37 front-end unit tests and 118 Playwright journey and WCAG 2.2 AA
  checks, run before every push; each push builds, smoke-tests and publishes a Windows
  installer on a GitHub runner.
- Shipped signed self-updates (minisign, Tauri updater): installed copies check hourly,
  ask before installing, show download progress and refuse an unsigned build.

**Keywords (ATS):** Rust, Tauri, TypeScript, React, Tailwind CSS, Radix UI, llama.cpp,
GGUF, local LLM inference, prompt engineering, constrained decoding (GBNF), Win32 API,
UI Automation, SQLite, Tokio, async Rust, GitHub Actions, CI/CD, accessibility (WCAG),
Hugging Face.
