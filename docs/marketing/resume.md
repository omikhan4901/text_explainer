# Resume bullets

Pick 3–4. Every number is measured (see `docs/PROGRESS.md`). Don't claim users or
downloads until they exist.

**Text Explainer**: offline "make this readable" desktop app (Rust, Tauri, React/TypeScript, llama.cpp) · 2026

- Built a Windows desktop app that rewrites any selected text at a chosen reading level
  using a small language model running locally through llama.cpp, with no data leaving
  the device.
- Eliminated language drift in small-model output by generating a GBNF grammar per
  request that blocks unexpected writing systems at sampling time, with a verified
  retry path for servers without grammar support.
- Designed a deterministic meaning check that compares numbers, dates, acronyms and names
  between source and rewrite and flags omissions or invented facts in the UI.
- Implemented selection capture through Windows UI Automation with a clipboard fallback
  that snapshots and restores every clipboard format, and a non-activating popup that
  never steals focus.
- Wrote 99 Rust tests and 9 front-end tests, including end-to-end engine tests against a
  stand-in llama.cpp server, gated in GitHub Actions on Linux and Windows.

**Keywords (ATS):** Rust, Tauri, TypeScript, React, Tailwind CSS, Radix UI, llama.cpp,
GGUF, local LLM inference, prompt engineering, constrained decoding (GBNF), Win32 API,
UI Automation, SQLite, Tokio, async Rust, GitHub Actions, CI/CD, accessibility (WCAG),
Hugging Face.
