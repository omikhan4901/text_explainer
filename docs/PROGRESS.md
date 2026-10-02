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
  Gemma 4 E2B 17.3 tok/s, 5.8 s median per answer (first words after 2.2 s), 3.9 GB
  peak memory, reading grade down 9.2 levels (Plain) and 12.2 (Simpler), facts flagged
  in 1 to 3 of 56 answers across runs.
- On a Windows runner, with the engine build the installer bundles: Gemma 4 E2B 15.4 tok/s,
  6.7 s median per answer (first words 2.5 s), 1 of 56 answers flagged, no wrong
  language; Qwen3.5 2B 15.8 tok/s, 4.8 s.
- Prompt cache: capped at 1024 MiB after measuring off / 256 / 1024 / 2048 MiB (turning it
  off made the first words 4 to 7 s slower; 256 MiB was too small for Qwen3.5 4B).
- Dictionary lookup: about 60 microseconds (budget 50 ms).
- Memory without a model (Windows runner, app plus 8 WebView2 processes): 146 MB
  private, about 385 MB working set summed over the processes (shared WebView2 pages
  count once per process), the same with the main window open and when started in the
  tray at login (the hidden settings window is still loaded). Budget 150 MB: within it
  by private memory, narrowly. Breakdown: the app itself 6 MB; WebView2's browser 45 MB,
  GPU 15, utilities 20, crash handler 2; one renderer per window (card, settings) about
  30 MB each. Creating the settings window only when opened would save about 30 MB; not
  worth the risk before the first real-machine test.
- First words (budget 2 s, default model, 4-core runner): 2.2 s median on Linux, 2.5 s
  on Windows, so just over budget; a paragraph's instructions dominate on a CPU.
- The Windows CI job builds and uploads the NSIS installer on every push to main, then
  smoke-tests it: silent install, the bundled llama-server runs, the app starts and finds
  its engine and dictionary.

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
  best). Qwen3.5 4B was the first guess; it generates about a third slower on a CPU and
  needs the most memory.
- LFM2.5 1.2B stays in the catalog, labelled as fast but unreliable with facts.
- Rust pinned to 1.99.0 (`rust-toolchain.toml`) so local and CI lints match.
