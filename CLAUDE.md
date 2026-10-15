# Working on Text Explainer

Offline desktop app: select text anywhere, press a hotkey, get a readable version next to
the cursor. Runs a small local model (llama.cpp) on an 8 GB CPU-only laptop.
The plan is `docs/IMPLEMENTATION_PLAN.md` (sources, translation and Definer parity: `docs/SOURCES_PLAN.md`); progress and the next step are in
`docs/PROGRESS.md`. Read both before starting work, and update `PROGRESS.md` as you go.
`docs/GUIDE.md` explains how everything works (keep it true when you change how things work).

## Conventions

- Commit as the owner, with no AI co-author lines and no model names:
  `git -c user.name=omikhan4901 -c user.email=mehboobehsankhan@gmail.com commit ...`
- Work on and push to `main` only. Small conventional commits (`feat:`, `fix:`, `test:`,
  `docs:`, `chore:`, `refactor:`).
- **Run `scripts/check.sh` before every push** and gate the push on its exit code
  (`E2E=1` adds the Playwright journeys and accessibility checks; locally use
  `PW_CHROMIUM_PATH=/opt/pw-browsers/chromium` if Playwright can't find Chromium). It also
  lints the Windows app (`src-tauri`) for the Windows target. The push-triggered CI
  workflow was removed on 15 October 2026 at the owner's request while the redesign is in
  progress (it is in git history as `.github/workflows/ci.yml`). Every push to `main` builds the
  Windows installer, smoke-tests it and publishes it to the public `latest-build`
  pre-release (`installer.yml`, the owner's decision, 15 October 2026: the README's
  Download link points there), so a broken `main` breaks the download. Numbered releases
  come from the manual "Release (draft)" workflow. Never leave `main` broken.
- Never spend the owner's money: standard GitHub runners only (the repo is public, so they
  are free), no paid services, no code signing purchases. Ask first if something costs.
- Never commit secrets, model files (`*.gguf`) or generated binaries.

## Layout

- `crates/core` (`te-core`): all logic that isn't Windows UI. Text cleanup and
  classification, readability scores, the meaning guard, the language guard (GBNF grammar),
  prompts, the llama-server client and process manager, the dictionary, model catalog and
  downloads, settings. Pure Rust, tested on Linux and Windows.
- `scripts/build-dictionary.py`: builds `dictionary.sqlite` from WordNet 3.1 (npm
  `wordnet-db`, irregular forms from `wink-lexicon`); `scripts/fetch-llama.py` fetches the
  pinned llama.cpp build. Both run in CI; neither output is committed.
- `crates/eval`: the model evaluation harness (`docs/models.md` has the results; run the
  "Model evaluation" workflow after changing prompts, guards or llama.cpp).
- `src-tauri`: the Tauri 2 app. Tray, global hotkey, selection capture (UI Automation, then
  a clipboard-safe Ctrl+C fallback), the popup window that never takes focus, IPC commands.
- `src/`: React + TypeScript + Tailwind v4 + Radix front end. `popup.html` is the reading
  card, `index.html` the main window (settings, models, first run). Design tokens in
  `src/styles/tokens.css` (`docs/GUIDE.md`, "The interface").

## Rules that are easy to break

- The popup must never take focus from the app the person is reading in.
- Model output always goes through the language guard (grammar on the request, script check
  on the result) so a model can't answer in the wrong language.
- The clipboard fallback restores the person's clipboard exactly, every time, even on error.
- No network requests except downloads the person starts (models) and the card's web
  tabs (`docs/SOURCES_PLAN.md`): the card opens on the first web search (the owner's
  decision, 15 October 2026) and loads only the open tab. AI and dictionary requests stay
  on the machine; "Offline only" (Sources page, and asked at first run) turns every web tab
  off. The model runs only when "Explain with AI" is opened, or at once when the card has
  nothing else to show.
- The card's web view is remote content: it gets no app permissions, never focuses on its
  own, opens new windows in the person's browser, and is closed with the card.
- Desktop software only: never a browser extension.
- Every UI string goes through `src/i18n` (English now; Bangla later).

## Working with the owner

- Look and feel: its own quiet design, not the HR app's (the owner rejected the Atlas
  look on 15 October 2026). Neutral surfaces, near-black controls, one accent that only
  marks where you are (Ink default; Plum / Saffron / Garnet); never blue or neon. Geist,
  Phosphor icons, Radix + Tailwind. No hero banners, pill clutter, gradient washes, glows,
  em dashes or illustrated mock-ups (the owner reads them as "AI-looking"). Calm, minimal,
  little text; one clear action at a time. The text being read is the loudest thing.
- Windows first: build the MVP for Windows only. macOS and Linux come after the MVP.
- This is a portfolio project: keep `docs/marketing/` current, and only claim what is built
  and measured.
- Tests are rigorous and edge-case heavy but fast.
- Never `pkill -f`/`pgrep -f` a pattern that appears in your own command line (it kills the
  shell).
