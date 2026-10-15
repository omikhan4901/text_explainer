# Sources, translation and Definer parity: plan

Decided by the owner on 15 October 2026: Text Explainer does everything
[Definer](https://chromewebstore.google.com/detail/mkhlnnpfjieohiohgbphcnfdkmniofac) does,
plus the local AI that Definer can't do. **It stays desktop software, never a browser
extension.** Everything here is built into the app and works in browsers the same way it
works in any other app.

## 1. What it does

### What Definer does (parity)

| Definer | In Text Explainer |
|---|---|
| Bubble next to the selection | The reading card, already placed by the selection's rectangle in any app, browsers included |
| Several sources in one bubble | The card gets tabs: Explain, Translate, Dictionary, then web sources |
| Custom sources | Add any site from a URL template such as `https://example.com/search?q={query}` |
| Toolbar search | A **Look up** box: from the tray menu or its own shortcut, type a word or phrase, get the same tabs |
| Trigger settings | The shortcut and double Ctrl+C (built), plus select-to-show, double-click a word, and select while holding a key, each a setting |
| Site rules | **App and site rules**: turn the card off for chosen apps, and in Chrome, Edge and Firefox for chosen sites (the site is read from the browser's address bar through UI Automation) |
| Translation | Pick the target language once; the Translate tab uses it |

Definer's right-click menu entry has no system-wide equivalent on Windows; the shortcut and
double Ctrl+C cover it.

### More than Definer

| Feature | What it means |
|---|---|
| **Explain** (first tab) | The plain-language rewrite at Simpler / Plain / Clearer with the reading grade before and after, from the local model |
| **AI Translate** | Translation by the local model, so the text never goes to a translation service |
| **Meaning guard** | If a rewrite or translation drops or invents a number, date or name, the card says so |
| **Language guard** | The model can't drift into a third language (the GBNF grammar the app already uses) |
| **Offline dictionary** | WordNet, bundled, about 60 microseconds a lookup, no internet |
| **Every app, not only the browser** | PDFs, Word, chat apps, terminals: anywhere text can be selected |
| **Offline only** | One switch hides every web source; then nothing leaves the computer at all |

### The card

```
┌──────────────────────────────────────────────────────────┐
│ Explain · Translate · Dictionary · Wikipedia · +         │  tabs; order and which show are settings
├──────────────────────────────────────────────────────────┤
│ (content of the open tab)                                │
├──────────────────────────────────────────────────────────┤
│ Simpler | Plain | Clearer      Grade 17 → 9    copy  pin │  only on the AI tabs
└──────────────────────────────────────────────────────────┘
```

- Same design as the rest of the app, same rules: never takes focus from the app the person is reading in,
  Esc or a click outside closes it, pin keeps it open.
- A single word opens on Dictionary; a sentence or more on Explain. Both are settings.
- Web source tabs carry a small globe so local and online are always told apart.
- Only the open tab does any work: a web source is fetched when its tab is opened, not
  before.

## 2. How it works

- **Tabs** are a list in settings: built-in local tabs (Explain, Translate, Dictionary),
  built-in web sources, and custom sources. Each can be shown or hidden and reordered.
- **Web sources, data kind** (Wikipedia summary, Wiktionary definitions, Free Dictionary
  API; all free, no key): fetched by `te-core` over HTTPS with a short timeout and drawn
  in the card in the app's own style, like the local dictionary. No page scripts run.
- **Web sources, page kind** (custom sources, Google Translate, a web search): the site is
  loaded in a child webview inside the card, in its own profile with no access to the
  app's IPC, cookies cleared when the app quits. Reading it doesn't take focus; clicking
  into it does, because then the person chose to use it. "Open in browser" on every page
  tab.
- **Translate** reuses the engine: a translation prompt, the language guard built for the
  target language's script, and the meaning guard on the result.
- **Look up box**: a small window that does take focus (it has a text field), with the
  same tabs underneath.
- **Triggers**: select-to-show and double-click use the low-level mouse hook the card
  already installs for "click outside", plus UI Automation to read the selection. Off by
  default, as decided for the MVP.
- **App and site rules**: the foreground process name, and for supported browsers the
  address bar's host, checked before the card opens.

## 3. Privacy, said plainly

- The AI tabs and the dictionary run on the computer. Selected text sent to them never
  leaves it.
- Web sources are like visiting those sites: when the person opens that tab, the app sends
  the selected text to that site. Nothing is sent for tabs that aren't opened.
- First run asks whether to use web sources; **Offline only** is the preselected answer.
  The switch stays in settings.
- No analytics, no account, no server of ours. Page sources get a separate webview profile.

## 4. Milestones

Each milestone ships on its own, passes `scripts/check.sh` and the Windows CI job, and is
pushed to `main`. The Windows MVP release (M6 in `IMPLEMENTATION_PLAN.md`) comes first.

| # | Milestone | Done when |
|---|---|---|
| S1 | Tabs and Look up | The card has tabs (Explain, Dictionary, then the rest as they land); the Look up box from the tray and a shortcut; tab settings |
| S2 | Web sources | Wikipedia, Wiktionary and Free Dictionary drawn in the card; Offline only switch and the first-run question; tests against saved responses |
| S3 | Page sources | Child webview with its own profile, custom sources from URL templates, Google Translate and web search, "Open in browser" |
| S4 | AI Translate | Translation prompts, language and meaning guards on translations, a translation corpus in `eval/` (English ↔ Bangla, Spanish, Hindi, Arabic, Chinese) measured per model in `docs/models.md` |
| S5 | Triggers and rules | Select-to-show, double-click a word, select while holding a key; app rules, and site rules for Chrome, Edge and Firefox |
| S6 | Polish | WCAG 2.2 AA checks on the new screens in light and dark, screenshots, README, `GUIDE.md` and the marketing kit updated |

## 5. Testing

- Rust tests for source URL templates, the data-source parsers (against saved responses),
  translation prompts and guards, and app / site rule matching.
- Vitest and Playwright for the tabs, Look up box and settings, with IPC mocked.
- CI never calls the real sites; the eval workflow measures translation on demand.

## 6. Risks

- **Small models and translation:** 2B models on a CPU may translate poorly into some
  languages, especially Bangla. S4 measures this before the UI promises it; languages that
  score badly get a "may be rough" label or a bigger-model hint.
- **Memory:** a child webview is another WebView2 renderer (about 30 MB). It is created
  only when a page tab is opened and closed with the card.
- **Sites that refuse to be framed** aren't a problem: the child webview loads the page as
  a top-level page, not in a frame.
