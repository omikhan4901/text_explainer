# Progress

Current milestone: **M3, models + first run** (wiring done; needs testing on Windows).

## Windows MVP checklist

- [x] Plan approved (Atlas, Windows first, select-to-show off, balanced model)
- [x] M0 Foundations: scaffold, docs, check script, CI (Linux + Windows installer), Atlas tokens, tray app
- [x] M1 Core: text repair, readability, language guard (GBNF), meaning guard, prompts, output filter, engine, pipeline
- [x] M2 Capture + popup: UI Automation, clipboard fallback, double Ctrl+C, non-activating card, Esc / click outside, level switch, copy, pin
- [ ] M3 Models + first run: catalog, downloads, own GGUF / local server, settings pages, first run (built; first Windows run pending)
- [ ] M4 Words (WordNet dictionary for single words)
- [ ] M5 Eval and default model (`crates/eval`, manual CI workflow, `docs/models.md`)
- [ ] M6 Polish + release (performance budgets, accessibility pass, installer release, README screenshots of the real app)

## Next step

Get the Windows CI job green end to end (installer artifact), then M5 eval workflow so the
default model and real speed numbers come from measurements, then M4.

## Numbers (for the marketing kit; measured, not estimated)

- Tests: 99 Rust (core and Windows crate, including 13 end-to-end engine tests against a
  stand-in llama-server and 6 download tests against a flaky local server), 9 front-end.
- Model files pinned with SHA-256 from Hugging Face; llama.cpp b11323 Windows CPU build
  pinned with SHA-256.

## Blocked on the owner

- Testing on a real Windows machine once the installer artifact is available (checklist
  will be in `docs/testing/windows.md`).

## Decisions made while building

- Tauri 2 (stable), not Tauri 3 (alpha in Oct 2026).
- This build container can't reach Hugging Face or GitHub release downloads, so models are
  only run in CI (the eval workflow) and on the owner's machine.
- Default shortcut Ctrl+Shift+Space: Alt combinations make many apps open their menu bar
  when Alt is released, and Ctrl+Alt+letter types characters on some keyboard layouts.
- Double Ctrl+C is on by default: it is the "copy from anywhere" path and needs no
  simulated keys. The hotkey is the fallback the owner asked for.
- Gemma 4 E4B is in the "Best" tier, not "Balanced": its file is 4.2 GB (per-layer
  embeddings), too heavy alongside a browser on 8 GB. Balanced candidates: Qwen3.5 4B
  (2.7 GB) and Gemma 4 E2B (2.6 GB); the eval decides between them.
- Rust pinned to 1.99.0 (`rust-toolchain.toml`) so local and CI lints match.
