// What the reading card shows, driven by events from the app. Pure so it's easy to test.
import type { DictionaryEntry, PopupEvent, Report, SelectionKind } from "../lib/types";

export interface Explaining {
  view: "explain";
  id: number;
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

export const initialState: CardState = { view: "empty" };

function fresh(id: number): Explaining {
  return {
    view: "explain",
    id,
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

export function reduce(state: CardState, event: PopupEvent): CardState {
  switch (event.type) {
    case "open":
      return fresh(event.id);
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
      // Without a model the entry is the whole answer, so the card isn't waiting.
      return { ...state, dictionary: event.entry, kind: state.started ? state.kind : "word", source: state.source || event.entry.lemma };
    case "explain": {
      // Events from an earlier request (still finishing) are ignored.
      if (state.view !== "explain" || state.id !== event.id) return state;
      const e = event.event;
      switch (e.type) {
        case "started":
          return {
            ...state,
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

/** The full answer so far. */
export function answerText(state: Explaining): string {
  return state.parts.filter((p) => p.trim()).join("\n\n");
}

/** True while the answer is still arriving (show the caret). */
export function isStreaming(state: Explaining): boolean {
  return !state.done && state.parts.some((p) => p.length > 0);
}

/** Only show the grade badge when it means something (passages, both grades known). */
export function gradeBadge(state: Explaining): { before: number; after: number } | null {
  if (state.kind !== "passage" || state.gradeBefore == null || state.gradeAfter == null) return null;
  return { before: state.gradeBefore, after: state.gradeAfter };
}
