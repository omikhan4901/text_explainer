# Text Explainer in the browser: plan

Decided by the owner on 15 October 2026: Text Explainer gets a browser extension that does
everything [Definer](https://chromewebstore.google.com/detail/mkhlnnpfjieohiohgbphcnfdkmniofac)
does, plus the local AI that Definer can't do. It stays local software: the AI and the
dictionary run on the person's computer, and the desktop app itself still never goes online.

## 1. What the extension does

### What Definer does (parity)

| Feature | What it means here |
|---|---|
| Bubble next to the selection | Select text on any page and a bubble opens beside it, without a new tab and without leaving the page |
| Several sources, one bubble | Each source is a tab in the bubble: dictionary, translation, Wikipedia, web search |
| Custom sources | Add any site as a source from a URL template such as `https://example.com/search?q={query}` |
| Toolbar search | Click the toolbar icon, type a word or phrase, get the same tabs |
| Trigger settings | Choose what opens the bubble: selecting text, double-clicking a word, selecting while holding a key, the shortcut, or the right-click menu |
| Site rules | Turn the bubble off on chosen sites, or only turn it on for chosen sites |
| Translation | Pick the target language once; the translation tab uses it |
| Chrome and Firefox | One code base, built for both (and Edge, which takes Chrome's build) |

### More than Definer

| Feature | What it means here |
|---|---|
| **Explain** (first tab) | The plain-language rewrite at Simpler / Plain / Clearer, with the reading grade before and after, streamed from the local model |
| **AI Translate** | Translation by the local model, so the text never goes to a translation service |
| **Meaning guard** | If a rewrite or translation drops or invents a number, date or name, the bubble says so |
| **Language guard** | The model can't drift into a third language (the GBNF grammar the app already uses) |
| **Offline dictionary** | The bundled WordNet dictionary answers in about 60 microseconds with no internet |
| **Meaning in context** | For a single word, the extension sends the sentence around it, so "here it means…" fits the page |
| **Outside the browser too** | The desktop shortcut and double Ctrl+C keep working in PDFs, Word and chat apps, with the same card |
| **Offline only switch** | One switch hides every web source; then nothing leaves the computer at all |

### The bubble

```
┌─────────────────────────────────────────────────────┐
│ Explain · Translate · Dictionary · Wikipedia · +    │  tabs; the order is a setting
├─────────────────────────────────────────────────────┤
│ (content of the open tab)                           │
├─────────────────────────────────────────────────────┤
│ Simpler | Plain | Clearer        Grade 17 → 9   ⧉ 📌 │  only on AI tabs
└─────────────────────────────────────────────────────┘
```

- Same Atlas design as the desktop card (tokens shared from `src/styles/tokens.css`),
  white by default, dark when the system is, Plum / Saffron / Garnet / Ink accents.
- Lives in a closed Shadow DOM so pages can't restyle it and it can't break pages.
- Esc or a click outside closes it; pin keeps it open; it never steals focus from a text
  field the person is typing in.
- Placed by the selection's rectangle, flipped above when there's no room below, kept on
  screen.
- A single word opens on Dictionary; a sentence or more opens on Explain. Both are settings.

## 2. How it works

```
Browser                                    Desktop
┌────────────────────────────┐             ┌────────────────────────────────┐
│ content script: bubble     │             │ Text Explainer (Tauri app)     │
│   (Shadow DOM, Atlas)      │             │  te-core: explain, translate,  │
│ service worker:            │  native     │  dictionary, guards            │
│   web sources (fetch)      │  messaging  │  llama-server (127.0.0.1)      │
│   bridge to the app ───────┼──► te-host ─┼──► named pipe, this user only  │
│ toolbar popup, options     │   (stdio)   │                                │
└────────────────────────────┘             └────────────────────────────────┘
```

- **`extension/`**: Manifest V3 extension in TypeScript and React, built with Vite into
  `dist-extension/chrome` and `dist-extension/firefox` (the manifests differ only in the
  background key and Firefox's `browser_specific_settings`).
- **Bridge, native messaging:** the browser starts `te-host.exe` (a small new crate,
  `crates/host`) and talks to it over stdin/stdout. The host forwards each request to the
  running app over a Windows named pipe that only the same user can open, and streams the
  answer back. The model stays loaded once, shared by the desktop card and the browser.
  Native messaging was chosen over a local web server because no website can reach it:
  the browser only lets the extension IDs listed in the host's manifest start it.
- **Messages:** `status`, `explain {text, level, context}`, `translate {text, to, context}`,
  `define {word, context}`, `cancel {id}`. Answers stream as `chunk`, then `done` with the
  guard results, or `error` with a reason the bubble can show.
- **App not running:** the host starts it in the tray and waits up to 10 seconds. Not
  installed: the AI and Dictionary tabs say "Install Text Explainer to use the local AI"
  with a link; web sources still work.
- **Web sources:** fetched by the extension's service worker, from the person's browser,
  only for sources they turned on. Built in, all free with no key:
  Wiktionary and Wikipedia (REST APIs), Free Dictionary API, a web search tab, and Google
  Translate as a page. Host permissions for a source are requested when it is turned on,
  not at install.
- **Page sources** (custom sources and Google Translate): the site is shown in a frame
  inside the bubble. Many sites refuse to be framed; for those requests only, a
  `declarativeNetRequest` rule removes `X-Frame-Options` and `frame-ancestors`, which is
  what Definer does. If a site still won't show, the tab offers "Open in a new tab".
- **Installer:** the NSIS installer adds `te-host.exe` and registers it for Chrome, Edge
  and Firefox under `HKCU` (no admin rights), and removes it on uninstall.

## 3. Privacy, said plainly

- The desktop app still makes no network requests except downloads the person starts.
- The AI tabs and the dictionary run on the computer. The selected text goes from the
  browser to the app on the same machine and nowhere else.
- Web sources are like visiting those sites: the browser sends the selected text to the
  sites the person turned on, and only when they open that tab. The bubble marks them with
  a small globe so the difference is visible.
- The "Offline only" switch hides every web source.
- The bubble needs to run on every site to appear there, which browsers show as "read and
  change all your data on all websites". The store listing and the first-run page explain
  why, and site rules can narrow it.
- No analytics, no account, no server of ours.

## 4. Milestones

Each milestone ships on its own, passes `scripts/check.sh` and CI, and is pushed to `main`.
Windows MVP release (M6 in `IMPLEMENTATION_PLAN.md`) comes first; this work starts after it
or alongside it where it doesn't touch the release.

| # | Milestone | Done when |
|---|---|---|
| B1 | Extension shell | `extension/` builds for Chrome and Firefox; bubble on selection with Atlas design, placement, Esc / click outside / pin; toolbar popup with a search box; options page; Playwright tests that load the unpacked extension in Chromium |
| B2 | Web sources | Built-in sources, custom sources from URL templates, page sources in a frame with the header rule, tab order and on/off per source, "Offline only" switch, globe marks |
| B3 | Bridge | `crates/host`, the named pipe in the app, streaming Explain and Dictionary in the bubble, "app not running" and "not installed" states, installer registration, tests against a fake app |
| B4 | AI Translate | Translation prompts, the language guard for the target language, meaning guard on translations, a translation corpus in `eval/` (English ↔ Bangla, Spanish, Hindi, Arabic, Chinese) measured per model in `docs/models.md` |
| B5 | Triggers and rules | Double-click, modifier key, shortcut (`commands`), right-click menu, site allow / block lists, which tab opens first |
| B6 | Release | WCAG 2.2 AA checks on the bubble, popup and options in light and dark; store listings and screenshots; README, `GUIDE.md` and the marketing kit updated |

## 5. Testing

- Vitest for the bubble's placement, source URL templates, site rules and message parsing.
- Playwright loads the unpacked Chrome build into Chromium and checks the journeys on local
  test pages: select → bubble → switch tabs → Esc; toolbar search; options; a page that
  tries to restyle the bubble; a page with a strict CSP.
- Web sources are tested against local fixtures, never the real sites, so CI stays offline
  and stable.
- `crates/host` is tested on Linux with a fake pipe, and on the Windows runner for real.
- Translation quality is measured by the eval workflow, on demand, like rewrites.

## 6. Costs and decisions for the owner

- **Chrome Web Store:** a one-time **$5** developer registration. Needs the owner's yes
  before B6. Until then the extension installs unpacked (developer mode).
- **Firefox Add-ons and Edge Add-ons:** free.
- **Small models and translation:** 2B models on a CPU may translate poorly into some
  languages, especially Bangla. B4 measures this before the UI promises it; languages that
  score badly get a "may be rough" label or a bigger-model hint.
