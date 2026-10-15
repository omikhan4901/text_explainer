// A stand-in for the Rust app when the pages run in a plain browser. It keeps settings in
// memory, simulates downloads, and can play scripted reading-card scenarios
// (`popup.html?demo=passage`) for development, tests and screenshots.
import type { AiTask, AppInfo, ExplainEvent, ModelInfo, NewWord, PopupEvent, Settings, TranslateAvailability, WebSource, WebTab } from "./types";
import { WordBook } from "./words";

type Handler = (payload: unknown) => void;

export const DEFAULT_SETTINGS: Settings = {
  version: 1,
  hotkey: "Ctrl+Shift+Space",
  double_copy: true,
  clipboard_fallback: true,
  level: "plain",
  output: { mode: "same_as_text" },
  model: { kind: "none" },
  temperature: 0.3,
  top_p: 0.9,
  context_size: 4096,
  threads: null,
  idle_unload_minutes: 10,
  unload_on_close: true,
  card_size: null,
  prompts: { simpler: null, plain: null, clearer: null },
  extra_server_args: [],
  appearance: { theme: "system", accent: "ink", reading_font: "plex", font_size: 15, line_height: 1.6 },
  start_with_windows: true,
  first_run_done: false,
  sources: [
    source("google", "Google", "https://www.google.com/search?q={query}", true, true, true),
    source("bing", "Bing", "https://www.bing.com/search?q={query}", true, true, true),
    source("duckduckgo", "DuckDuckGo", "https://duckduckgo.com/?q={query}", true, true, true),
    source("wikipedia", "Wikipedia", "https://en.wikipedia.org/w/index.php?search={query}", true, true, false),
    source("translate", "Translate", "https://translate.google.com/?sl=auto&tl={lang}&text={query}&op=translate", true, true, true),
    source("bing-translator", "Bing Translator", "https://www.bing.com/translator?from=auto&to={lang}&text={query}", false, true, true),
    source("wiktionary", "Wiktionary", "https://en.wiktionary.org/w/index.php?search={query}", false, true, false),
    source("cambridge", "Cambridge", "https://dictionary.cambridge.org/search/english/direct/?q={query}", false, true, false),
    source("brave", "Brave", "https://search.brave.com/search?q={query}", false, true, true),
    source("images", "Images", "https://www.bing.com/images/search?q={query}", false, true, false),
  ],
  offline_only: false,
  translate_to: "en",
  web_language: { mode: "fixed", code: "en" },
  lookup_hotkey: "",
  excluded_apps: ["keepass.exe", "keepassxc.exe", "1password.exe", "bitwarden.exe"],
  keep_history: true,
  check_updates: true,
  history_days: 30,
};

function source(id: string, name: string, url: string, enabled: boolean, words: boolean, passages: boolean): WebSource {
  return { id, name, url, enabled, words, passages };
}

const GB = 1024 ** 3;

function model(id: string, name: string, maker: string, tier: ModelInfo["tier"], params: string, size: number, license: string, note: string): ModelInfo {
  const ram = size + 0.9 * 1024 ** 3;
  const total = 8 * GB;
  return {
    id,
    name,
    maker,
    tier,
    params,
    quant: "Q4",
    file_name: `${id}.gguf`,
    url: `https://huggingface.co/${id}`,
    size_bytes: size,
    sha256: "0".repeat(64),
    license,
    license_url: "https://www.apache.org/licenses/LICENSE-2.0",
    note,
    ram_needed_bytes: ram,
    fit: ram * 2 <= total ? "comfortable" : ram * 10 <= total * 7 ? "tight" : "too_big",
    installed: false,
    partial_bytes: 0,
  };
}

const MODELS: ModelInfo[] = [
  model(
    "gemma-4-e2b",
    "Gemma 4 E2B",
    "Google",
    "balanced",
    "2B effective",
    2_620_370_976,
    "Apache 2.0",
    "Fastest of the balanced models here and the best at keeping facts in our tests.",
  ),
  model(
    "qwen3.5-4b",
    "Qwen3.5 4B",
    "Alibaba Qwen",
    "balanced",
    "4B",
    2_740_937_888,
    "Apache 2.0",
    "Good rewrites; slower than Gemma 4 E2B on a CPU and needs more memory.",
  ),
  model("qwen3.5-2b", "Qwen3.5 2B", "Alibaba Qwen", "fast", "2B", 1_280_835_840, "Apache 2.0", "Light on memory and quick; drops details more often."),
  model(
    "lfm2.5-1.2b",
    "LFM2.5 1.2B",
    "Liquid AI",
    "fastest",
    "1.2B",
    730_895_168,
    "LFM Open License v1.0",
    "Very fast, but often drops or changes facts and is English only in practice. Free for personal use and smaller companies; check the license.",
  ),
  model(
    "gemma-4-e4b",
    "Gemma 4 E4B",
    "Google",
    "best",
    "4B effective",
    4_215_695_776,
    "Apache 2.0",
    "Higher quality, larger download; best with 12 GB of RAM or more.",
  ),
  model(
    "qwen3.5-9b",
    "Qwen3.5 9B",
    "Alibaba Qwen",
    "best",
    "9B",
    5_680_522_464,
    "Apache 2.0",
    "The most capable here; needs 16 GB of RAM and is slower on CPU.",
  ),
];

const SYSTEM = (style: string) =>
  `You make hard text easy to read. You rewrite the text the user gives you.\n\nRules:\n1. Keep the meaning exactly. Do not add facts, opinions, advice or warnings. Do not leave out facts.\n2. Keep every number, date, amount, percentage and name exactly as written.\n3. ${style}\n4. Explain a technical term in a few plain words the first time it appears.\n5. Spell out an abbreviation the first time it appears, with the short form in brackets.\n6. Keep the order of ideas. Keep lists as lists and keep paragraph breaks.\n7. Write only in {language}.\n8. Reply with the rewritten text only: no title, no introduction, no notes.`;

let settings: Settings = structuredClone(DEFAULT_SETTINGS);
const installed = new Set<string>();
// `?offline=1` starts with "Offline only" on (no web tabs).
if (typeof location !== "undefined" && new URLSearchParams(location.search).get("offline") === "1") {
  settings = { ...settings, offline_only: true };
}
// `?lang=bn` sets "Translate into" (the AI tab's Translate and the web Translate tab).
const langParam = typeof location !== "undefined" ? new URLSearchParams(location.search).get("lang") : null;
if (langParam) settings = { ...settings, translate_to: langParam };
// `?state=ready` starts as if set up already (for screenshots of the settings pages).
if (typeof location !== "undefined" && new URLSearchParams(location.search).get("state") === "ready") {
  installed.add("gemma-4-e2b");
  settings = { ...settings, first_run_done: true, model: { kind: "catalog", id: "gemma-4-e2b" } };
}
const handlers = new Map<string, Set<Handler>>();
// `?update=downloading` (or `failed`) shows an update in progress in the main window.
const updateParam = typeof location !== "undefined" ? new URLSearchParams(location.search).get("update") : null;

// The Word book, with a few saved words and lookups so the Words page has something to show.
let ago = 0;
const book = new WordBook(() => Date.now() - ago);
{
  const minute = 60_000;
  ago = 4 * 24 * 60 * minute;
  const seed = book;
  const words: [string, NewWord["kind"], string | null, string | null][] = [
    ["notwithstanding", "word", "despite anything to the contrary", "Here it means: whatever else the agreement says."],
    ["accrue", "word", "grow by addition", "The late fee is added to what is owed."],
    ["অভিধান", "word", null, "শব্দের অর্থ লেখা বই।"],
    ["carte blanche", "phrase", "complete freedom to act as you wish", null],
  ];
  for (const [term, kind, definition, explanation] of words) {
    seed.save({ term, kind, definition, explanation });
    ago -= 24 * 60 * minute;
  }
  ago = 3 * 60 * minute;
  for (const [query, kind] of [
    ["lessee", "word"],
    ["carte blanche", "phrase"],
    ["Notwithstanding any provision of this Agreement to the contrary, the Lessee shall remit payment.", "passage"],
  ] as const) {
    seed.record(query, kind, true, 30);
    ago -= 50 * minute;
  }
  ago = 0;
}
let downloadTimer: ReturnType<typeof setInterval> | undefined;
let downloading: string | null = null;

function emit(event: string, payload: unknown) {
  handlers.get(event)?.forEach((h) => h(payload));
}

function info(): AppInfo {
  return {
    version: "0.1.0",
    models: MODELS.map((m) => ({ ...m, installed: installed.has(m.id) })),
    recommended: "gemma-4-e2b",
    total_ram_bytes: 8 * GB,
    languages: [
      { code: "en", name: "English", native: "English" },
      { code: "bn", name: "Bangla (Bengali)", native: "বাংলা" },
      { code: "hi", name: "Hindi", native: "हिन्दी" },
      { code: "es", name: "Spanish", native: "Español" },
      { code: "fr", name: "French", native: "Français" },
    ],
    translate_languages: TRANSLATE_LANGUAGES,
    engine_found: true,
    updates: true,
    model_loaded: false,
    downloading,
    paused: false,
    hotkey_active: !(typeof location !== "undefined" && new URLSearchParams(location.search).get("hotkey") === "taken"),
    default_prompts: {
      simpler: SYSTEM("Write for a 12-year-old: sentences under 12 words, the most common everyday words, one idea per sentence."),
      plain: SYSTEM("Use plain language: sentences under 20 words, common words, active voice."),
      clearer: SYSTEM("Keep the original vocabulary and level, but make it easier to follow: split long sentences, untangle nested clauses, use active voice."),
    },
  };
}

/** Every command the pages called, in order (tests read it as `window.__teMockCalls`). */
const calls: { command: string; args: Record<string, unknown> }[] = [];
if (typeof window !== "undefined") Object.assign(window, { __teMockCalls: calls });

/** An app name as the list keeps it (`rules.rs`), or an error message. */
export function normalizeApp(name: string): string {
  const file = name.trim().split(/[\\/]/).pop()!.trim();
  if (!file) throw "type the app's program name, such as notepad.exe";
  if ([...file].some((c) => c < " " || '<>:"|?*'.includes(c))) throw "the program name has characters a file name can't have";
  const lower = file.toLowerCase();
  if (!lower.endsWith(".exe") || !lower.slice(0, -4).replace(/^[. ]+|[. ]+$/g, "")) throw "the program name must end in .exe";
  if ([...lower].length > 64) throw "the program name is too long";
  return lower;
}

async function call<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  calls.push({ command, args });
  switch (command) {
    case "look_up":
      if (!String(args.text).trim()) throw "Type a word or phrase to look up.";
      return undefined as T;
    case "last_app":
      return "notepad.exe" as T;
    case "validate_app":
      return normalizeApp(String(args.name)) as T;
    case "get_settings":
      return structuredClone(settings) as T;
    case "save_settings":
      if ((args.settings as Settings).lookup_hotkey.toLowerCase() === (args.settings as Settings).hotkey.toLowerCase())
        throw "That shortcut can't be used: it already opens the card.";
      settings = structuredClone(args.settings as Settings);
      emit("te://settings", null);
      return structuredClone(settings) as T;
    case "app_info":
      return info() as T;
    case "download_model": {
      const id = args.id as string;
      const m = MODELS.find((x) => x.id === id)!;
      downloading = id;
      let done = 0;
      clearInterval(downloadTimer);
      return new Promise<T>((resolve) => {
        downloadTimer = setInterval(() => {
          done = Math.min(m.size_bytes, done + m.size_bytes / 40);
          emit("te://download", { state: "progress", id, downloaded: done, total: m.size_bytes, bytes_per_sec: 9.5e6 });
          if (done >= m.size_bytes) {
            clearInterval(downloadTimer);
            installed.add(id);
            downloading = null;
            if (args.activate) settings = { ...settings, model: { kind: "catalog", id } };
            emit("te://download", { state: "done", id });
            emit("te://settings", null);
            resolve(undefined as T);
          }
        }, 80);
      });
    }
    case "cancel_download":
      clearInterval(downloadTimer);
      if (downloading) emit("te://download", { state: "cancelled", id: downloading });
      downloading = null;
      return undefined as T;
    case "delete_model":
      installed.delete(args.id as string);
      return undefined as T;
    case "validate_hotkey": {
      const h = String(args.hotkey);
      if (!/^(Ctrl|Alt|Shift|Super)(\+(Ctrl|Alt|Shift|Super))*\+\w+$/i.test(h)) {
        throw "Use at least one of Ctrl, Alt or Shift.";
      }
      return undefined as T;
    }
    case "set_level":
      settings = { ...settings, level: args.level as Settings["level"] };
      emit("te://popup", { type: "settings" });
      if (pendingAi) void pendingAi(demoId);
      return undefined as T;
    case "set_ai_task":
      if (args.id === demoId) {
        aiTask = args.task as AiTask;
        aiTarget = (args.to as string | null) ?? settings.translate_to;
        if (pendingAi) void pendingAi(demoId);
      }
      return undefined as T;
    case "popup_drag":
      // Windows moves or resizes the window; the app then saves a resized card's size.
      if (args.kind === "resize") {
        settings = { ...settings, card_size: { width: innerWidth - 24, height: innerHeight - 24 } };
        emit("te://popup", { type: "settings" });
      }
      return undefined as T;
    case "set_card_size":
      settings = { ...settings, card_size: args.size as Settings["card_size"] };
      emit("te://popup", { type: "settings" });
      return undefined as T;
    case "explain_now":
      if (args.id === demoId && pendingAi) void pendingAi(demoId);
      return undefined as T;
    case "web_show": {
      const source = settings.sources.find((s) => s.id === args.source);
      if (!source || settings.offline_only) throw "That source is turned off.";
      const query = currentDemo.startsWith("word") ? "ubiquitous" : SOURCE;
      return sourceUrl(source.url, query, settings.translate_to) as T;
    }
    case "find_models":
      await new Promise((r) => setTimeout(r, 250));
      return [
        {
          name: "Qwen3-4B-Instruct-2507-Q4_K_M",
          path: "C:\\Users\\Omi\\.lmstudio\\models\\lmstudio-community\\Qwen3-4B-Instruct-2507-GGUF\\Qwen3-4B-Instruct-2507-Q4_K_M.gguf",
          size_bytes: 2_497_281_120,
          source: "LM Studio",
          ram_needed_bytes: 3_745_921_680,
          fit: "comfortable",
        },
        {
          name: "llama3.2:3b",
          path: "C:\\Users\\Omi\\.ollama\\models\\blobs\\sha256-dde5aa3fc5ffc17176b5e8bdc82f587b24b2678c6c66101bf7da77af9f7ccdff",
          size_bytes: 2_019_377_376,
          source: "Ollama",
          ram_needed_bytes: 3_029_066_064,
          fit: "comfortable",
        },
        {
          name: "Mistral-Small-24B-Q4_K_M",
          path: "C:\\Users\\Omi\\Downloads\\Mistral-Small-24B-Q4_K_M.gguf",
          size_bytes: 14_334_000_000,
          source: "Downloads",
          ram_needed_bytes: 21_501_000_000,
          fit: "too_big",
        },
      ] as T;
    case "save_word":
      return book.save(args.word as NewWord) as T;
    case "remove_word":
      return book.remove(String(args.term)) as T;
    case "check_updates":
      await new Promise((r) => setTimeout(r, 300));
      return { state: "up_to_date" } as T;
    case "word_book_on_disk":
      return true as T;
    case "is_saved":
      return book.isSaved(String(args.term)) as T;
    case "list_words":
      return book.list(String(args.search ?? ""), Number(args.limit), Number(args.offset)) as T;
    case "list_history":
      return book.history(Number(args.limit), Number(args.offset)) as T;
    case "clear_history":
      return book.clearHistory() as T;
    case "export_words": {
      const page = book.list("", 200, 0);
      return page.total as T;
    }
    case "speak":
      return undefined as T;
    case "validate_source": {
      const url = String(args.url).trim();
      if (!/^https?:\/\//i.test(url)) throw "the address must start with https:// or http://";
      if (!url.includes("{query}")) throw "the address must contain {query} where the selected text goes";
      return undefined as T;
    }
    default:
      return undefined as T;
  }
}

function on(event: string, handler: Handler): () => void {
  if (!handlers.has(event)) handlers.set(event, new Set());
  handlers.get(event)!.add(handler);
  if (event === "te://update" && updateParam) {
    setTimeout(
      () =>
        handler(
          updateParam === "failed"
            ? { state: "failed", version: "0.1.22", message: "The download was interrupted." }
            : { state: "downloading", version: "0.1.22", downloaded: 4 * 1024 ** 2, total: 10 * 1024 ** 2 },
        ),
      50,
    );
  }
  return () => handlers.get(event)?.delete(handler);
}

// Scripted reading-card scenarios.
const SOURCE =
  "Notwithstanding any provision of this Agreement to the contrary, the Lessee shall remit payment of the monthly rent of $1,200 no later than the 5th day of each calendar month, failing which a late fee equivalent to 5% of the outstanding amount shall accrue.";
const REWRITES: Record<Settings["level"], string> = {
  simpler:
    "The tenant (the Lessee) must pay $1,200 rent every month. They must pay it by the 5th day of the month. If they pay late, they owe an extra fee. The fee is 5% of the money they still owe. This rule comes before any other rule in the agreement.",
  plain:
    "Whatever else this agreement says, the tenant (the Lessee) must pay the monthly rent of $1,200 by the 5th day of each month. If they don't, a late fee of 5% of the unpaid amount is added.",
  clearer:
    "Despite anything else in this Agreement, the Lessee must pay the monthly rent of $1,200 by the 5th day of each calendar month. If the Lessee pays late, a late fee of 5% of the outstanding amount accrues.",
};

let currentDemo = "";
let demoTimer: ReturnType<typeof setTimeout> | undefined;
let demoId = 0;
/** What the AI tab does in the current demo, once the card asks for it. */
let pendingAi: ((id: number) => Promise<void>) | null = null;

function sleep(ms: number) {
  return new Promise<void>((r) => {
    demoTimer = setTimeout(r, ms);
  });
}

const UBIQUITOUS = {
  lemma: "ubiquitous",
  senses: [
    { pos: "adjective" as const, definition: "being present everywhere at once", example: "ubiquitous computing", synonyms: ["omnipresent"] },
    {
      pos: "adjective" as const,
      definition: "seeming to be everywhere, often to the point of being tiresome",
      example: "the ubiquitous advertising on buses",
      synonyms: ["pervasive", "universal"],
    },
  ],
};

/** The web tabs the app would send for a selection. */
function webTabs(word: boolean): WebTab[] {
  if (settings.offline_only) return [];
  return settings.sources.filter((s) => s.enabled && (word ? s.words : s.passages)).map(({ id, name }) => ({ id, name }));
}

async function playDemo(name: string) {
  currentDemo = name;
  clearTimeout(demoTimer);
  const id = ++demoId;
  aiTask = "explain";
  aiTarget = "";
  if (name === "no_model") return emit("te://popup", { type: "no_model" });
  if (name === "no_selection") return emit("te://popup", { type: "no_selection", hotkey: settings.hotkey });
  const word = name === "word" || name === "word_offline";
  if (name === "word_offline") settings = { ...settings, offline_only: true };
  const web = webTabs(word);
  const hasModel = name !== "word_offline";
  const ai = !hasModel ? "off" : web.length > 0 || word ? "deferred" : "running";
  book.record(word ? "ubiquitous" : SOURCE, word ? "word" : "passage", settings.keep_history, settings.history_days);
  emit("te://popup", {
    type: "open",
    id,
    ai,
    kind: word ? "word" : "passage",
    query: word ? "ubiquitous" : SOURCE,
    web,
    translate: translateAvailability(),
  } satisfies PopupEvent);
  if (word) emit("te://popup", { type: "dictionary", id, entry: UBIQUITOUS } satisfies PopupEvent);
  pendingAi = word ? explainWord : (requestId) => explainPassage(requestId, name);
  if (ai === "running") void pendingAi(id);
}

async function explainPassage(id: number, name: string) {
  if (aiTask === "translate") return translateDemo(id, "passage");
  const send = (event: ExplainEvent) => emit("te://popup", { type: "explain", id, task: "explain", event } satisfies PopupEvent);
  send({ type: "started", kind: "passage", source: SOURCE, truncated: false, grade_before: 17.4, parts: 1, source_parts: [SOURCE], language: "English" });
  if (name === "loading") {
    send({ type: "loading" });
    return;
  }
  if (name === "error") {
    await sleep(300);
    send({ type: "error", message: "the model engine stopped unexpectedly (exit code 1)", detail: "llama_model_load: error loading model: invalid magic" });
    return;
  }
  await sleep(450);
  const text = REWRITES[settings.level];
  const words = text.split(/(?<= )/);
  for (const w of words) {
    if (id !== demoId) return;
    send({ type: "delta", part: 0, text: w });
    await sleep(name === "still" ? 0 : 35);
  }
  send({ type: "part_done", part: 0, text });
  const report = name === "check" ? { missing: [{ kind: "number" as const, text: "5%" }], added: [] } : { missing: [], added: [] };
  send({
    type: "done",
    text,
    grade_after: settings.level === "simpler" ? 4.9 : settings.level === "plain" ? 8.6 : 11.2,
    report,
    timings: { predicted_per_second: 14.2 },
    elapsed_ms: 3200,
  });
}

/** A word's meaning in its text, once the card's AI tab asks for it. */
async function explainWord(id: number) {
  if (aiTask === "translate") return translateDemo(id, "word");
  const send = (event: ExplainEvent) => emit("te://popup", { type: "explain", id, task: "explain", event } satisfies PopupEvent);
  send({
    type: "started",
    kind: "word",
    source: "ubiquitous",
    truncated: false,
    grade_before: null,
    parts: 1,
    source_parts: ["ubiquitous"],
    language: "English",
  });
  await sleep(500);
  if (id !== demoId) return;
  const text = "Here it means found almost everywhere: smartphones are part of nearly every moment of daily life.";
  send({ type: "part_done", part: 0, text });
  send({ type: "done", text, grade_after: null, report: { missing: [], added: [] }, timings: null, elapsed_ms: 480 });
}

/** The AI's translation in the demos (canned; the app runs the local model). */
const TRANSLATIONS: Record<string, { word: string; passage: string }> = {
  bn: {
    word: "সর্বব্যাপী: এখানে অর্থ প্রায় সব জায়গায় পাওয়া যায়।",
    passage:
      "এই চুক্তিতে যা-ই থাকুক না কেন, ভাড়াটিয়াকে প্রতি মাসের ৫ তারিখের মধ্যে মাসিক ভাড়া $1,200 পরিশোধ করতে হবে; না হলে বকেয়া অর্থের 5% বিলম্ব ফি যোগ হবে।",
  },
  es: {
    word: "ubicuo: aquí significa que está en casi todas partes.",
    passage:
      "Sin perjuicio de cualquier disposición en contrario de este Contrato, el Arrendatario pagará el alquiler mensual de $1,200 a más tardar el día 5 de cada mes; de lo contrario, se aplicará un recargo del 5% sobre el importe pendiente.",
  },
};

const TRANSLATE_LANGUAGES = [
  { code: "en", name: "English", native: "English", ai: true, lang: "en" },
  { code: "bn", name: "Bangla", native: "বাংলা", ai: true, lang: "bn" },
  { code: "hi", name: "Hindi", native: "हिन्दी", ai: true, lang: "hi" },
  { code: "ur", name: "Urdu", native: "اردو", ai: true, lang: "ur" },
  { code: "ar", name: "Arabic", native: "العربية", ai: true, lang: "ar" },
  { code: "es", name: "Spanish", native: "Español", ai: true, lang: "es" },
  { code: "fr", name: "French", native: "Français", ai: true, lang: "fr" },
  { code: "de", name: "German", native: "Deutsch", ai: true, lang: "de" },
  { code: "zh-CN", name: "Chinese (Simplified)", native: "简体中文", ai: true, lang: "zh" },
  { code: "ja", name: "Japanese", native: "日本語", ai: true, lang: "ja" },
  { code: "th", name: "Thai", native: "ไทย", ai: false, lang: null },
];

let aiTask: AiTask = "explain";
let aiTarget = "";

/** The demo texts are English: Translate goes into the setting's language (as the app's
 * `translate::offer`, which also sends text in that language into English). */
function translateAvailability(): TranslateAvailability {
  const lang = TRANSLATE_LANGUAGES.find((l) => l.code === settings.translate_to) ?? TRANSLATE_LANGUAGES[0]!;
  const source = { code: "en", name: "English" };
  if (!lang.ai) return { state: "unsupported", name: lang.name, source };
  if (lang.code === "en") return { state: "same_language", name: lang.name, source };
  return { state: "available", code: lang.code, name: lang.name, source };
}

async function translateDemo(id: number, kind: "word" | "passage") {
  const send = (event: ExplainEvent) => emit("te://popup", { type: "explain", id, task: "translate", event } satisfies PopupEvent);
  const lang = aiTarget || settings.translate_to;
  send({
    type: "started",
    kind,
    source: kind === "word" ? "ubiquitous" : SOURCE,
    truncated: false,
    grade_before: null,
    parts: 1,
    source_parts: [kind === "word" ? "ubiquitous" : SOURCE],
    language: TRANSLATE_LANGUAGES.find((l) => l.code === lang)?.name ?? lang,
  });
  await sleep(400);
  if (id !== demoId || aiTask !== "translate" || (aiTarget || settings.translate_to) !== lang) return;
  const text = (TRANSLATIONS[lang] ?? TRANSLATIONS.es!)[kind];
  send({ type: "part_done", part: 0, text });
  send({ type: "done", text, grade_after: null, report: { missing: [], added: [] }, timings: null, elapsed_ms: 900 });
}

/** A source's page address, as the app builds it (sources.rs). */
export function sourceUrl(template: string, query: string, lang: string): string {
  return template.replace("{query}", encodeURIComponent(query.split(/\s+/).join(" "))).replace("{lang}", encodeURIComponent(lang));
}

export const mock = { call, on, playDemo, emit };
