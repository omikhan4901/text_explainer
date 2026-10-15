// A stand-in for the Rust app when the pages run in a plain browser. It keeps settings in
// memory, simulates downloads, and can play scripted reading-card scenarios
// (`popup.html?demo=passage`) for development, tests and screenshots.
import type { AppInfo, ExplainEvent, ModelInfo, PopupEvent, Settings, WebSource, WebTab } from "./types";

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
    source("wiktionary", "Wiktionary", "https://en.wiktionary.org/w/index.php?search={query}", false, true, false),
    source("cambridge", "Cambridge", "https://dictionary.cambridge.org/search/english/direct/?q={query}", false, true, false),
    source("brave", "Brave", "https://search.brave.com/search?q={query}", false, true, true),
    source("images", "Images", "https://www.bing.com/images/search?q={query}", false, true, false),
  ],
  offline_only: false,
  translate_to: "en",
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
// `?state=ready` starts as if set up already (for screenshots of the settings pages).
if (typeof location !== "undefined" && new URLSearchParams(location.search).get("state") === "ready") {
  installed.add("gemma-4-e2b");
  settings = { ...settings, first_run_done: true, model: { kind: "catalog", id: "gemma-4-e2b" } };
}
const handlers = new Map<string, Set<Handler>>();
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
    engine_found: true,
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

async function call<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  switch (command) {
    case "get_settings":
      return structuredClone(settings) as T;
    case "save_settings":
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
      if (pendingAi) void pendingAi(demoId);
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
  if (name === "no_model") return emit("te://popup", { type: "no_model" });
  if (name === "no_selection") return emit("te://popup", { type: "no_selection", hotkey: settings.hotkey });
  const word = name === "word" || name === "word_offline";
  if (name === "word_offline") settings = { ...settings, offline_only: true };
  const web = webTabs(word);
  const hasModel = name !== "word_offline";
  const ai = !hasModel ? "off" : web.length > 0 || word ? "deferred" : "running";
  emit("te://popup", {
    type: "open",
    id,
    ai,
    kind: word ? "word" : "passage",
    query: word ? "ubiquitous" : SOURCE,
    web,
  } satisfies PopupEvent);
  if (word) emit("te://popup", { type: "dictionary", id, entry: UBIQUITOUS } satisfies PopupEvent);
  pendingAi = word ? explainWord : (requestId) => explainPassage(requestId, name);
  if (ai === "running") void pendingAi(id);
}

async function explainPassage(id: number, name: string) {
  const send = (event: ExplainEvent) => emit("te://popup", { type: "explain", id, event } satisfies PopupEvent);
  send({ type: "started", kind: "passage", source: SOURCE, truncated: false, grade_before: 17.4, parts: 1, language: "English" });
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
  const send = (event: ExplainEvent) => emit("te://popup", { type: "explain", id, event } satisfies PopupEvent);
  send({ type: "started", kind: "word", source: "ubiquitous", truncated: false, grade_before: null, parts: 1, language: "English" });
  await sleep(500);
  if (id !== demoId) return;
  const text = "Here it means found almost everywhere: smartphones are part of nearly every moment of daily life.";
  send({ type: "part_done", part: 0, text });
  send({ type: "done", text, grade_after: null, report: { missing: [], added: [] }, timings: null, elapsed_ms: 480 });
}

/** A source's page address, as the app builds it (sources.rs). */
export function sourceUrl(template: string, query: string, lang: string): string {
  return template.replace("{query}", encodeURIComponent(query.split(/\s+/).join(" "))).replace("{lang}", encodeURIComponent(lang));
}

export const mock = { call, on, playDemo, emit };
