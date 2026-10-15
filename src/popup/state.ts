// What the reading card shows, driven by events from the app. Pure so it's easy to test.
import type { AiMode, DictionaryEntry, PopupEvent, Report, SelectionKind } from "../lib/types";

/** The card's tabs: the model's explanation and the bundled dictionary. */
export type TabId = "explain" | "dictionary";

export interface Explaining {
  view: "explain";
  id: number;
  /** "requested": the person opened Explain on a word and the card has asked the app. */
  ai: AiMode | "requested";
  tab: TabId;
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

/** What the card itself does (the person picks a tab). */
export type CardAction = { type: "select_tab"; tab: TabId };

export const initialState: CardState = { view: "empty" };

function fresh(id: number, ai: AiMode): Explaining {
  return {
    view: "explain",
    id,
    ai,
    tab: "explain",
    started: false,
    kind: "passage",
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
      return fresh(event.id, event.ai);
    case "select_tab":
      if (state.view !== "explain" || !tabs(state).includes(event.tab)) return state;
      // Opening Explain on a word starts the model (the card then asks the app for it).
      return { ...state, tab: event.tab, ai: event.tab === "explain" && state.ai === "deferred" ? "requested" : state.ai };
    case "no_selection":
      return { view: "no_selection", hotkey: event.hotkey };
    case "no_model":
      return { view: "no_model" };
    case "capture_failed":
      return { view: "capture_failed", message: event.message };
    case "settings":
      return state;
    case "dictionary":
      if (state.view !== "explain" || state.id !== event.id) return state;
      return {
        ...state,
        dictionary: event.entry,
        // A word opens on its dictionary entry unless the model is already answering.
        tab: state.ai === "running" && state.started ? state.tab : "dictionary",
        kind: state.started ? state.kind : "word",
        source: state.source || event.entry.lemma,
      };
    case "explain": {
      // Events from an earlier request (still finishing) are ignored.
      if (state.view !== "explain" || state.id !== event.id) return state;
      const e = event.event;
      switch (e.type) {
        case "started":
          return {
            ...state,
            ai: "running",
            started: true,
            kind: e.kind,
            source: e.source,
            truncated: e.truncated,
            language: e.language,
            gradeBefore: e.grade_before,
            parts: Array.from({ length: e.parts }, () => ""),
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

/** The tabs this request has, in order: a word's dictionary entry comes first. */
export function tabs(state: Explaining): TabId[] {
  const list: TabId[] = [];
  if (state.dictionary) list.push("dictionary");
  if (state.ai !== "off") list.push("explain");
  return list;
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
  return state.done && !state.error ? answerText(state) : "";
}

/** True while the answer is still arriving (show the caret). */
export function isStreaming(state: Explaining): boolean {
  return !state.done && state.parts.some((p) => p.length > 0);
}

/** Only show the grade when it means something (passages, both grades known). */
export function gradeBadge(state: Explaining): { before: number; after: number } | null {
  if (state.kind !== "passage" || state.gradeBefore == null || state.gradeAfter == null) return null;
  return { before: state.gradeBefore, after: state.gradeAfter };
}
