import { bareTerm } from "./Popup";
import { answerText, gradeBadge, initialState, isStreaming, reduce, type CardState, type Explaining } from "./state";
import type { ExplainEvent, PopupEvent } from "../lib/types";

const ex = (id: number, event: ExplainEvent): PopupEvent => ({ type: "explain", id, event });
const started: ExplainEvent = {
  type: "started",
  kind: "passage",
  source: "Hard text.",
  truncated: false,
  grade_before: 16.2,
  parts: 2,
  language: "English",
};

function run(events: PopupEvent[], from: CardState = initialState): CardState {
  return events.reduce(reduce, from);
}

test("streams parts and finishes with the cleaned text", () => {
  const s = run([
    { type: "open", id: 1 },
    ex(1, started),
    ex(1, { type: "loading" }),
    ex(1, { type: "delta", part: 0, text: "The tenant " }),
    ex(1, { type: "delta", part: 0, text: "pays." }),
  ]) as Explaining;
  expect(s.loadingModel).toBe(false);
  expect(answerText(s)).toBe("The tenant pays.");
  expect(isStreaming(s)).toBe(true);

  const done = run(
    [
      ex(1, { type: "part_done", part: 0, text: "The tenant pays." }),
      ex(1, { type: "delta", part: 1, text: "On time." }),
      ex(1, { type: "part_done", part: 1, text: "On time." }),
      ex(1, { type: "done", text: "", grade_after: 6.1, report: { missing: [], added: [] }, timings: { predicted_per_second: 12 }, elapsed_ms: 900 }),
    ],
    s,
  ) as Explaining;
  expect(answerText(done)).toBe("The tenant pays.\n\nOn time.");
  expect(isStreaming(done)).toBe(false);
  expect(gradeBadge(done)).toEqual({ before: 16.2, after: 6.1 });
  expect(done.tokensPerSecond).toBe(12);
});

test("events from an older request are ignored", () => {
  const s = run([{ type: "open", id: 1 }, ex(1, started), { type: "open", id: 2 }, ex(1, { type: "delta", part: 0, text: "stale" })]) as Explaining;
  expect(s.id).toBe(2);
  expect(answerText(s)).toBe("");
});

test("a restarted part drops its drifted text", () => {
  const s = run([
    { type: "open", id: 3 },
    ex(3, started),
    ex(3, { type: "delta", part: 0, text: "这是" }),
    ex(3, { type: "restart_part", part: 0 }),
    ex(3, { type: "delta", part: 0, text: "This is" }),
  ]) as Explaining;
  expect(answerText(s)).toBe("This is");
});

test("errors end the request", () => {
  const s = run([{ type: "open", id: 4 }, ex(4, started), ex(4, { type: "error", message: "boom", detail: "log" })]) as Explaining;
  expect(s.done).toBe(true);
  expect(s.error).toEqual({ message: "boom", detail: "log" });
});

test("words have no grade badge", () => {
  const s = run([
    { type: "open", id: 5 },
    ex(5, { ...started, kind: "word", grade_before: null, parts: 1 } as ExplainEvent),
    ex(5, { type: "done", text: "x", grade_after: null, report: { missing: [], added: [] }, timings: null, elapsed_ms: 1 }),
  ]) as Explaining;
  expect(gradeBadge(s)).toBeNull();
});

test("other views", () => {
  expect(reduce(initialState, { type: "no_model" })).toEqual({ view: "no_model" });
  expect(reduce(initialState, { type: "no_selection", hotkey: "Ctrl+Shift+Space" })).toEqual({
    view: "no_selection",
    hotkey: "Ctrl+Shift+Space",
  });
  expect(reduce(initialState, { type: "settings" })).toBe(initialState);
});

test("dictionary entries attach to the current request", () => {
  const entry = { lemma: "mouse", senses: [{ pos: "noun" as const, definition: "a small rodent", example: null, synonyms: [] }] };
  const s = run([{ type: "open", id: 7 }, { type: "dictionary", id: 7, entry }, { type: "dictionary", id: 6, entry: { ...entry, lemma: "old" } }]) as Explaining;
  expect(s.dictionary?.lemma).toBe("mouse");
  expect(s.kind).toBe("word");
  expect(s.source).toBe("mouse");
});

test("terms are shown without surrounding punctuation", () => {
  expect(bareTerm("“Ubiquitous,”")).toBe("Ubiquitous");
  expect(bareTerm("(habeas corpus).")).toBe("habeas corpus");
  expect(bareTerm("ভাড়া।")).toBe("ভাড়া");
});
