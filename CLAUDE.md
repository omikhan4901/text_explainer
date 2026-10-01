# Working on Text Explainer

Offline desktop app: select text anywhere, press a hotkey, get a readable version next to
the cursor. Runs a small local model (llama.cpp) on an 8 GB CPU-only laptop.
The plan is `docs/IMPLEMENTATION_PLAN.md`; progress and the next step are in
`docs/PROGRESS.md`. Read both before starting work, and update `PROGRESS.md` as you go.
`docs/GUIDE.md` explains how everything works (keep it true when you change how things work).

## Conventions

- Commit as the owner, with no AI co-author lines and no model names:
  `git -c user.name=omikhan4901 -c user.email=mehboobehsankhan@gmail.com commit ...`
- Work on and push to `main` only. Small conventional commits (`feat:`, `fix:`, `test:`,
  `docs:`, `chore:`, `refactor:`).
- **Run `scripts/check.sh` before every push** and gate the push on its exit code. It runs
  what the Linux CI job runs. The Windows app (`src-tauri`) only builds on the Windows CI
  job: after pushing, check that job and fix it before moving on. Never leave `main` red.
- Never spend the owner's money: standard GitHub runners only (the repo is public, so they
  are free), no paid services, no code signing purchases. Ask first if something costs.
- Never commit secrets, model files (`*.gguf`) or generated binaries.

## Layout

- `crates/core` (`te-core`): all logic that isn't Windows UI. Text cleanup and
  classification, readability scores, the meaning guard, the language guard (GBNF grammar),
  prompts, the llama-server client and process manager, the dictionary, model catalog and
  downloads, settings. Pure Rust, tested on Linux and Windows.
- `crates/dictgen`: builds `dictionary.sqlite` from WordNet 3.1 (npm `wordnet-db`).
- `crates/eval`: the model eval harness (`docs/models.md` has the results).
- `src-tauri`: the Tauri 2 app. Tray, global hotkey, selection capture (UI Automation, then
  a clipboard-safe Ctrl+C fallback), the popup window that never takes focus, IPC commands.
- `src/`: React + TypeScript + Tailwind v4 + Radix front end. `popup.html` is the reading
  card, `index.html` the main window (settings, models, first run). Atlas design tokens in
  `src/styles/tokens.css`.

## Rules that are easy to break

- The popup must never take focus from the app the person is reading in.
- Model output always goes through the language guard (grammar on the request, script check
  on the result) so a model can't answer in the wrong language.
- The clipboard fallback restores the person's clipboard exactly, every time, even on error.
- Nothing goes over the network except downloads the person starts (models, dictionary).
- Every UI string goes through `src/i18n` (English now; Bangla later).

## Working with the owner

- Look and feel: Atlas design (see `companymgmt/docs/design/atlas`). White by default,
  dark mode follows the system, accents Plum (default) / Saffron / Garnet / Ink; never blue
  or neon. Radix + Tailwind. No gradient washes, glows or illustrated mock-ups (the owner
  reads them as "AI-looking"). Calm, minimal, little text; one clear action at a time.
- Windows first: build the MVP for Windows only. macOS and Linux come after the MVP.
- This is a portfolio project: keep `docs/marketing/` current, and only claim what is built
  and measured.
- Tests are rigorous and edge-case heavy but fast.
