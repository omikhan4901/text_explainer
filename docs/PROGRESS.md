# Progress

Current milestone: **M6, polish and release**. Paused on 2 October 2026 as ongoing work:
everything that can be built and checked without a Windows desktop is done and green in
CI. Next is the owner's first test on a real Windows machine (`docs/testing/windows.md`).

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

When work resumes:

1. The owner installs the latest CI installer and goes through `docs/testing/windows.md`;
   fix whatever it finds.
2. Screenshots of the installed app for the README and marketing kit.
3. Run "Release (draft)" and publish the first release; then post the LinkedIn draft and
   record the demo video (`docs/marketing/`).
4. Optional, measured first: create the settings window only when opened (about 30 MB
   less while the app sits in the tray).

The hourly resume routine is switched off; nothing runs on its own while paused.

## After the MVP: sources and translation

Planned on 15 October 2026 (`docs/SOURCES_PLAN.md`): everything Definer does (tabs of
sources, custom sources, a Look up box, triggers, app and site rules) plus local Explain,
AI Translate and the offline dictionary. Desktop software only, never a browser extension.

- [x] S0 Plan
- [x] Redesign (15 October 2026): the owner found the Atlas look (copied from the HR app)
      ugly. New design system: neutral surfaces, near-black controls, one quiet accent
      (Ink default), Geist and Phosphor icons; text sidebar instead of the icon rail; no
      hero banner or pill clutter. The card is now tabbed, and a word opens on its
      Dictionary tab with the model started only if Explain is opened.
- [x] Search first (15 October 2026, owner): the card opens on a web search (Google,
      Bing, DuckDuckGo, Wikipedia, Translate on by default), then the dictionary, then
      "Explain with AI", which runs only when opened. Web pages show in a child web view
      inside the card (Tauri `unstable` multi-webview), checked only by lint for the
      Windows target so far: needs the owner's Windows test. Sources settings page,
      Offline only (asked at first run), custom sites from URL templates.
- [x] Models already on the computer (15 October 2026, owner): found in LM Studio,
      Ollama, Jan, GPT4All, Hugging Face and Downloads, each with its memory fit, usable
      in one click from the Model page and first run.
- [x] S1 Tabs and Look up
- [ ] S2 Web sources
- [x] S3 Page sources (see "Search first")
- [ ] S4 AI Translate (built; measuring per model and language is next)
- [ ] S5 Triggers and rules (app rules done; select-to-show, double-click and site rules next)
- [ ] S6 Polish

## Release 1.0 plan (15 October 2026)

The owner asked for a product that can be sold. Who buys: students, language learners
and people who read hard text at work (contracts, research, government letters), on
Windows. Why us over Definer: works in every app (not only the browser), explains and
translates with a local model so private text stays on the computer, and keeps the words
you looked up.

Built in parallel (each engineer in their own worktree, merged by the PM), then reviewed
by security, database and AI engineers before it reaches `main`:

- [x] Look up box: tray, the hotkey with nothing selected, an optional own shortcut (S1)
- [x] App rules: never read selections in chosen apps, password managers by default (S5)
- [x] Word book: save words from the card, history, export to CSV and Anki, Listen
- [x] Private AI Translate in the AI tab, with the language and meaning guards (S4);
      translation quality per model and language is not measured yet (eval translate task)
- [x] Review (security, database, AI): 9 findings, all fixed (Chinese and Japanese
      paragraphs were treated as one word, same-script text handed back went unnoticed,
      false alarms on names, a translation could overwrite a saved explanation, a history
      file could be deleted when it couldn't be moved aside, and smaller ones)
- [x] After the review (15 October 2026): web translators beside the AI (compare in the
      Translate tab, Bing Translator as an option), a translated passage shown sentence by
      sentence under its original, a new logo (a line of text with one word highlighted),
      the README redone as a product page, and the installer published as the repo's
      Latest release
- [x] Updates (15 October 2026, owner): each build is 0.1.<run number>; with the update
      key in GitHub, builds are signed and installed apps ask, then update themselves.
      Key set by the owner; build 0.1.19 is the first signed one (`latest.json` published,
      signature checked on the release). Installing an update on a real PC is not verified
      yet.
- [x] Website (15 October 2026, owner): one page at plainly.omikhan.com, Next.js in
      `site/`, hosted on Vercel next to omikhan.com. Waiting on the owner to import the
      project and add the subdomain (`site/README.md`).
- [x] Fixes from the owner's first real-PC test (15 October 2026): the card came up cut
      off (it opened at a fixed start height and the page reports only size changes; it
      now opens at the last measured size), it can be dragged by its header and resized
      from its corner (size remembered, double-click to fit), web pages ask for their
      phone layout so they fit, clicking into a web page lets you type there, and the
      model's memory is freed when the card closes ("Free memory after: The card closes",
      the new default). Needs the owner's check on Windows: dragging and resizing a
      non-activating window, typing in the web page, the memory going back.
- [x] Second real-PC test (15 October 2026): updates now show their download in the main
      window; dragging and resizing follow the real cursor from Rust (the page's mouse
      coordinates went stale while the window moved, so dragging did nothing and resizing
      lost track); a resized card fills its window; the card stays open until its X (no
      more closing on a click elsewhere, so the pin is gone). Bing still serves its desktop
      page (it reads the browser's hidden "Windows" hint, not the phone identity); the
      owner is fine with that.
- [x] Third test: dragging and resizing still did nothing (the follow thread asked Windows
      whether the mouse button was held, which says "no" while another app is in front);
      now the page reports each move and the app reads the real cursor. The shortcut
      warning with a dead shortcut: registration at startup can lose a race with the
      closing copy after an update; it is now retried every 5 seconds.
- [ ] Not verified on Windows yet: the Look up box's focus and placement, app rules on
      real apps, the Windows voice, export through the save dialog

Decisions that belong to the owner (not built until decided):

- Price and how to take payments (one-time licence or yearly; a merchant of record such
  as Paddle or Lemon Squeezy handles tax). Costs money and has legal terms.
- Code signing: unsigned installers get a SmartScreen warning, which hurts sales. A
  certificate costs money every year.
- A product website (GitHub Pages is free) and where the download lives.

## Numbers (for the marketing kit; measured, not estimated)

- Tests (15 October 2026): 198 Rust (core and Windows crate, including end-to-end engine
  tests against a stand-in llama-server, download tests against a flaky local server, and
  the word book's SQLite tests), 37 front-end unit tests, 108 Playwright checks (journeys
  and WCAG 2.2 AA scans, each in light and dark).
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
- Until 15 October 2026 the Windows CI job built and uploaded the NSIS installer on every
  push to main, then smoke-tested it: silent install, the bundled llama-server runs, the
  app starts and finds its engine and dictionary. The push CI is paused for the redesign
  (owner's call); "Release (draft)" still builds the installer on demand.

## Blocked on the owner

- Testing on a real Windows machine: download the latest build (README's Download link,
  rebuilt on every push); the checklist is `docs/testing/windows.md`.
- Publishing the first release: run "Release (draft)" and press Publish when happy.

## Decisions made while building

- Renamed to **Plainly** on 15 October 2026 (owner). The app identifier stays
  `dev.omikhan.textexplainer` so settings, models and the word book carry over; the
  repository, crate names and paths keep the old name. The installer is now
  `Plainly-x64-setup.exe`; an install made under the old name sits in its own folder and
  can be uninstalled once Plainly is installed.

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
