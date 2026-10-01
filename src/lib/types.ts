// TypeScript mirrors of the Rust types the app sends (serde field names).

export type Level = "simpler" | "plain" | "clearer";
export const LEVELS: Level[] = ["simpler", "plain", "clearer"];

export type OutputChoice = { mode: "same_as_text" } | { mode: "fixed"; code: string };

export type ModelChoice =
  | { kind: "none" }
  | { kind: "catalog"; id: string }
  | { kind: "file"; path: string }
  | { kind: "endpoint"; base_url: string; model: string; api_key?: string | null };

export type Theme = "system" | "light" | "dark";
export type AccentName = "plum" | "saffron" | "garnet" | "ink";
export type ReadingFont = "plex" | "atkinson" | "serif";

export interface Appearance {
  theme: Theme;
  accent: AccentName;
  reading_font: ReadingFont;
  font_size: number;
  line_height: number;
}

export interface CustomPrompts {
  simpler: string | null;
  plain: string | null;
  clearer: string | null;
}

export interface Settings {
  version: number;
  hotkey: string;
  double_copy: boolean;
  clipboard_fallback: boolean;
  level: Level;
  output: OutputChoice;
  model: ModelChoice;
  temperature: number;
  top_p: number;
  context_size: number;
  threads: number | null;
  idle_unload_minutes: number;
  prompts: CustomPrompts;
  extra_server_args: string[];
  appearance: Appearance;
  start_with_windows: boolean;
  first_run_done: boolean;
}

export type Tier = "fastest" | "fast" | "balanced" | "best";
export type Fit = "comfortable" | "tight" | "too_big";

export interface ModelInfo {
  id: string;
  name: string;
  maker: string;
  tier: Tier;
  params: string;
  quant: string;
  file_name: string;
  url: string;
  size_bytes: number;
  sha256: string;
  license: string;
  license_url: string;
  note: string;
  ram_needed_bytes: number;
  fit: Fit;
  installed: boolean;
  partial_bytes: number;
}

export interface LanguageInfo {
  code: string;
  name: string;
  native: string;
}

export interface AppInfo {
  version: string;
  models: ModelInfo[];
  recommended: string;
  total_ram_bytes: number | null;
  languages: LanguageInfo[];
  engine_found: boolean;
  model_loaded: boolean;
  downloading: string | null;
  paused: boolean;
  default_prompts: { simpler: string; plain: string; clearer: string };
}

export type SelectionKind = "empty" | "word" | "phrase" | "passage";

export interface Fact {
  kind: "number" | "acronym" | "name";
  text: string;
}

export interface Report {
  missing: Fact[];
  added: Fact[];
}

export interface Timings {
  prompt_n?: number | null;
  prompt_ms?: number | null;
  predicted_n?: number | null;
  predicted_ms?: number | null;
  predicted_per_second?: number | null;
}

export type ExplainEvent =
  | {
      type: "started";
      kind: SelectionKind;
      source: string;
      truncated: boolean;
      grade_before: number | null;
      parts: number;
      language: string | null;
    }
  | { type: "loading" }
  | { type: "delta"; part: number; text: string }
  | { type: "part_done"; part: number; text: string }
  | { type: "restart_part"; part: number }
  | {
      type: "done";
      text: string;
      grade_after: number | null;
      report: Report;
      timings: Timings | null;
      elapsed_ms: number;
    }
  | { type: "error"; message: string; detail: string | null };

export interface Sense {
  pos: "noun" | "verb" | "adjective" | "adverb";
  definition: string;
  example: string | null;
  synonyms: string[];
}

export interface DictionaryEntry {
  lemma: string;
  senses: Sense[];
}

export type PopupEvent =
  | { type: "open"; id: number }
  | { type: "dictionary"; id: number; entry: DictionaryEntry }
  | { type: "explain"; id: number; event: ExplainEvent }
  | { type: "no_selection"; hotkey: string }
  | { type: "no_model" }
  | { type: "capture_failed"; message: string }
  | { type: "settings" };

export type DownloadEvent =
  | { state: "progress"; id: string; downloaded: number; total: number; bytes_per_sec: number }
  | { state: "done"; id: string }
  | { state: "failed"; id: string; message: string }
  | { state: "cancelled"; id: string };
