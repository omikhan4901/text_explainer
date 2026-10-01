# Progress

Current milestone: **M0, foundations**.

## Windows MVP checklist

- [x] Plan approved (Atlas, Windows first, select-to-show off, balanced model)
- [ ] M0 Foundations: scaffold, docs, check script, CI, Atlas tokens, tray app
- [ ] M1 Core: text, readability, guards, prompts, engine
- [ ] M2 Capture + popup
- [ ] M3 Models + first run
- [ ] M4 Words (dictionary)
- [ ] M5 Eval and default model
- [ ] M6 Polish + release

## Next step

Scaffold the workspace (Cargo workspace, Tauri app, Vite front end) and CI.

## Numbers (for the marketing kit; measured, not estimated)

None yet.

## Blocked on the owner

Nothing.

## Decisions made while building

- Tauri 2 (stable), not Tauri 3 (alpha in Oct 2026).
- This build container can't reach Hugging Face or GitHub release downloads, so models are
  only run in CI (the eval workflow) and on the owner's machine.
