// What the reading card shows, driven by events from the app. Pure so it's easy to test.
import type { AiMode, AiTask, DictionaryEntry, PopupEvent, Report, SelectionKind, TranslateAvailability, WebTab } from "../lib/types";

/** The card's tabs: web searches (`web:<source id>`), the bundled dictionary, and the
 * model's explanation ("Explain with AI"). */
export type TabId = "explain" | "dictionary" | `web:${string}`;

export interface Explaining {
  view: "explain";
  id: number;
  /** "requested": the person opened the AI tab and the card has asked the app. */
  ai: AiMode | "requested";
  tab: TabId;
  /** The selection, one line, for the header. */
  query: string;
  web: WebTab[];
  /** What the AI tab does: explain, or translate into the language from Settings. */
  task: AiTask;
  translate: TranslateAvailability;
  started: boolean;
  kind: SelectionKind;
  source: string;
  truncated: boolean;
  language: string | null;
  gradeBefore: number | null;
  gradeAfter: number | null;
  /** Text per part (long selections are rewritten in parts). */
  parts: string[];
  loadingModel: boolean;
  done: boolean;
  report: Report | null;
  error: { message: string; detail: string | null } | null;
  elapsedMs: number | null;
  tokensPerSecond: number | null;
  /** The bundled dictionary's entry, for single words and short terms. */
  dictionary: DictionaryEntry | null;
}

export type CardState =
  | { view: "empty" }
  | { view: "no_selection"; hotkey: string }
  | { view: "no_model" }
  | { view: "capture_failed"; message: string }
  | Explaining;

/** What the card itself does (the person picks a tab, or Explain / Translate in the AI tab). */
export type CardAction = { type: "select_tab"; tab: TabId } | { type: "set_task"; task: AiTask };

export const initialState: CardState = { view: "empty" };

export const webTab = (id: string): TabId => `web:${id}`;
export const webSourceOf = (tab: TabId): string | null => (tab.startsWith("web:") ? tab.slice(4) : null);

function fresh(event: Extract<PopupEvent, { type: "open" }>): Explaining {
  return {
    view: "explain",
    id: event.id,
    ai: event.ai,
    // Searches come first; with none, the AI tab (a word's dictionary entry takes over
    // when it arrives).
    tab: event.web[0] ? webTab(event.web[0].id) : "explain",
    query: event.query,
    web: event.web,
    task: "explain",
    translate: event.translate,
    started: false,
    kind: event.kind,
    source: "",
    truncated: false,
    language: null,
    gradeBefore: null,
    gradeAfter: null,
    parts: [],
    loadingModel: false,
    done: false,
    report: null,
    error: null,
    elapsedMs: null,
    tokensPerSecond: null,
    dictionary: null,
  };
}

export function reduce(state: CardState, event: PopupEvent | CardAction): CardState {
  switch (event.type) {
    case "open":
      return fresh(event);
    case "select_tab":
      if (state.view !== "explain" || !tabs(state).includes(event.tab)) return state;
      // Opening the AI tab starts the model (the card then asks the app for it).
      return { ...state, tab: event.tab, ai: event.tab === "explain" && state.ai === "deferred" ? "requested" : state.ai };
    case "set_task": {
      if (state.view !== "explain" || state.task === event.task || !canRun(state, event.task)) return state;
      // The app runs the request again with the other task; the old answer goes now.
      return {
        ...state,
        task: event.task,
        started: false,
        parts: [],
        loadingModel: false,
        done: false,
        gradeAfter: null,
        report: null,
        error: null,
        elapsedMs: null,
        tokensPerSecond: null,
      };
    }
    case "no_selection":
      return { view: "no_selection", hotkey: event.hotkey };
    case "no_model":
      return { view: "no_model" };
    case "capture_failed":
      return { view: "capture_failed", message: event.message };
    case "settings":
      return state;
    case "dictionary": {
      if (state.view !== "explain" || state.id !== event.id) return state;
      // With no searches, a word opens on its entry unless the model is already answering.
      const toEntry = state.tab === "explain" && !state.started;
      return {
        ...state,
        dictionary: event.entry,
        tab: toEntry ? "dictionary" : state.tab,
        source: state.source || event.entry.lemma,
      };
    }
    case "explain": {
      // Events from an earlier request, or for the task the person just left, are ignored.
      if (state.view !== "explain" || state.id !== event.id || event.task !== state.task) return state;
      const e = event.event;
      switch (e.type) {
        case "started":
          // Also the start of a redo at another reading level: the old answer goes.
          return {
            ...state,
            ai: "running",
            started: true,
            kind: e.kind,
            source: e.source,
            truncated: e.truncated,
            language: e.language,
            gradeBefore: e.grade_before,
            gradeAfter: null,
            parts: Array.from({ length: e.parts }, () => ""),
            loadingModel: false,
            done: false,
            report: null,
            error: null,
          };
        case "loading":
          return { ...state, loadingModel: true };
        case "delta":
          return { ...state, loadingModel: false, parts: setPart(state.parts, e.part, (state.parts[e.part] ?? "") + e.text) };
        case "part_done":
          return { ...state, loadingModel: false, parts: setPart(state.parts, e.part, e.text) };
        case "restart_part":
          return { ...state, parts: setPart(state.parts, e.part, "") };
        case "done":
          return {
            ...state,
            loadingModel: false,
            done: true,
            gradeAfter: e.grade_after,
            report: e.report,
            elapsedMs: e.elapsed_ms,
            tokensPerSecond: e.timings?.predicted_per_second ?? null,
          };
        case "error":
          return { ...state, loadingModel: false, done: true, error: { message: e.message, detail: e.detail } };
      }
    }
  }
  return state;
}

function setPart(parts: string[], index: number, text: string): string[] {
  const next = parts.slice();
  while (next.length <= index) next.push("");
  next[index] = text;
  return next;
}

/** The searches and the dictionary, in order (the AI tab sits apart, at the end). */
export function tabs(state: Explaining): TabId[] {
  const list: TabId[] = state.web.map((w) => webTab(w.id));
  if (state.dictionary) list.push("dictionary");
  list.push("explain");
  return list;
}

export function isWord(state: Explaining): boolean {
  return state.kind === "word" || state.kind === "phrase";
}

/** The full answer so far. */
export function answerText(state: Explaining): string {
  return state.parts.filter((p) => p.trim()).join("\n\n");
}

/** A dictionary entry as plain text, for copying. */
export function entryText(entry: DictionaryEntry): string {
  const senses = entry.senses.map((s, i) => `${i + 1}. (${s.pos}) ${s.definition}${s.example ? ` "${s.example}"` : ""}`);
  return [entry.lemma, ...senses].join("\n");
}

/** What the copy button copies on the open tab ("" while there's nothing finished). */
export function copyable(state: Explaining): string {
  if (state.tab === "dictionary") return state.dictionary ? entryText(state.dictionary) : "";
  if (state.tab === "explain") return state.done && !state.error ? answerText(state) : "";
  return "";
}

/** True while the answer is still arriving (show the caret). */
export function isStreaming(state: Explaining): boolean {
  return !state.done && state.parts.some((p) => p.length > 0);
}

/** True while the model works on this request. */
export function isWorking(state: Explaining): boolean {
  return (state.ai === "requested" || state.ai === "running") && !state.done;
}

/** Whether the AI tab can do `task` for this selection (Translate needs a supported language
 * that the text isn't already in). */
export function canRun(state: Explaining, task: AiTask): boolean {
  return task === "explain" || state.translate.state === "available";
}

/** Only show the grade when it means something (explained passages, both grades known). */
export function gradeBadge(state: Explaining): { before: number; after: number } | null {
  if (state.task !== "explain" || state.kind !== "passage" || state.gradeBefore == null || state.gradeAfter == null) return null;
  return { before: state.gradeBefore, after: state.gradeAfter };
}
