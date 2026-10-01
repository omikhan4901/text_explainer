# Progress

Current milestone: **M6, polish and release**. The Windows CI job builds the installer;
next is the owner's first test on a real Windows machine (`docs/testing/windows.md`).

## Windows MVP checklist

- [x] Plan approved (Atlas, Windows first, select-to-show off, balanced model)
- [x] M0 Foundations: scaffold, docs, check script, CI (Linux + Windows installer), Atlas tokens, tray app
- [x] M1 Core: text repair, readability, language guard (GBNF), meaning guard, prompts, output filter, engine, pipeline
- [x] M2 Capture + popup: UI Automation, clipboard fallback, double Ctrl+C, non-activating card, Esc / click outside, level switch, copy, pin
- [x] M3 Models + first run: catalog, downloads, own GGUF / local server, settings pages, first run
- [x] M4 Words: WordNet dictionary (inflections, irregular forms, British spellings, proper-noun flag)
- [x] M5 Eval and default model: `crates/eval`, workflow, `docs/models.md`; default is Gemma 4 E2B
- [ ] M6 Polish + release: accessibility checks done (WCAG 2.2 AA, light and dark); draft-release
      workflow done; remaining: the owner's Windows test, fixes from it, screenshots of the
      real app, first draft release

## Next step

Wait for the owner's Windows test results; meanwhile keep polishing (performance on the
real machine, any issue in `docs/testing/windows.md`).

## Numbers (for the marketing kit; measured, not estimated)

- Tests: 109 Rust (core, Windows crate and real-dictionary checks, including 13 end-to-end
  engine tests against a stand-in llama-server and 6 download tests against a flaky local
  server), 10 front-end unit tests, 38 Playwright checks (19 journeys and WCAG 2.2 AA
  scans, each in light and dark).
- Model evaluation (4-vCPU GitHub runner, no GPU, 56 rewrites per model; `docs/models.md`):
  Gemma 4 E2B 16.9 tok/s, 6.0 s median per answer, 3.9 GB peak memory, reading grade
  down 9.1 levels (Plain) and 12.7 (Simpler), facts flagged in 4 of 56 answers.
- Dictionary lookup: about 60 microseconds (budget 50 ms).
- The Windows CI job builds and uploads the NSIS installer on every push to main.

## Blocked on the owner

- Testing on a real Windows machine: the installer is in the latest CI run's artifacts;
  the checklist is `docs/testing/windows.md`.
- Publishing the first release: run "Release (draft)" and press Publish when happy.

## Decisions made while building

- Tauri 2 (stable), not Tauri 3 (alpha in Oct 2026).
- This build container can't reach Hugging Face or GitHub release downloads, so models are
  only run in CI (the eval workflow) and on the owner's machine.
- Default shortcut Ctrl+Shift+Space: Alt combinations make many apps open their menu bar
  when Alt is released, and Ctrl+Alt+letter types characters on some keyboard layouts.
- Double Ctrl+C is on by default: it is the "copy from anywhere" path and needs no
  simulated keys. The hotkey is the fallback the owner asked for.
- Gemma 4 E4B is in the "Best" tier, not "Balanced": its file is 4.2 GB (per-layer
  embeddings), too heavy alongside a browser on 8 GB.
- Default model: Gemma 4 E2B, by the evaluation (as fast as the 2B models, kept facts
  best). Qwen3.5 4B was the first guess; it is half as fast on a CPU.
- LFM2.5 1.2B stays in the catalog, labelled as fast but unreliable with facts.
- Rust pinned to 1.99.0 (`rust-toolchain.toml`) so local and CI lints match.
