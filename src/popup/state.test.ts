import { bareTerm } from "./Popup";
import { answerText, copyable, gradeBadge, initialState, isStreaming, isWorking, reduce, tabs, type CardState, type Explaining } from "./state";
import type { AiTask, TranslateAvailability, AiMode, ExplainEvent, PopupEvent, SelectionKind, WebTab } from "../lib/types";

const SEARCHES: WebTab[] = [
  { id: "google", name: "Google" },
  { id: "bing", name: "Bing" },
];
const EN = { code: "en", name: "English" };
const SPANISH: TranslateAvailability = { state: "available", code: "es", name: "Spanish", source: EN };

function open(id: number, ai: AiMode, kind: SelectionKind = "passage", web: WebTab[] = [], translate = SPANISH): PopupEvent {
  return { type: "open", id, ai, kind, query: "Hard text.", web, translate };
}

const ex = (id: number, event: ExplainEvent, task: AiTask = "explain"): PopupEvent => ({ type: "explain", id, task, event });
const started: ExplainEvent = {
  type: "started",
  kind: "passage",
  source: "Hard text.",
  truncated: false,
  grade_before: 16.2,
  parts: 2,
  source_parts: ["Hard", "text."],
  language: "English",
};

function run(events: PopupEvent[], from: CardState = initialState): CardState {
  return events.reduce(reduce, from);
}

test("streams parts and finishes with the cleaned text", () => {
  const s = run([
    open(1, "running"),
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
  const s = run([open(1, "running"), ex(1, started), open(2, "running"), ex(1, { type: "delta", part: 0, text: "stale" })]) as Explaining;
  expect(s.id).toBe(2);
  expect(answerText(s)).toBe("");
});

test("a restarted part drops its drifted text", () => {
  const s = run([
    open(3, "running"),
    ex(3, started),
    ex(3, { type: "delta", part: 0, text: "这是" }),
    ex(3, { type: "restart_part", part: 0 }),
    ex(3, { type: "delta", part: 0, text: "This is" }),
  ]) as Explaining;
  expect(answerText(s)).toBe("This is");
});

test("errors end the request", () => {
  const s = run([open(4, "running"), ex(4, started), ex(4, { type: "error", message: "boom", detail: "log" })]) as Explaining;
  expect(s.done).toBe(true);
  expect(s.error).toEqual({ message: "boom", detail: "log" });
});

test("words have no grade badge", () => {
  const s = run([
    open(5, "running"),
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
  const s = run([
    open(7, "deferred", "word"),
    { type: "dictionary", id: 7, entry },
    { type: "dictionary", id: 6, entry: { ...entry, lemma: "old" } },
  ]) as Explaining;
  expect(s.dictionary?.lemma).toBe("mouse");
  expect(s.kind).toBe("word");
  expect(s.source).toBe("mouse");
});

test("terms are shown without surrounding punctuation", () => {
  expect(bareTerm("“Ubiquitous,”")).toBe("Ubiquitous");
  expect(bareTerm("(habeas corpus).")).toBe("habeas corpus");
  expect(bareTerm("ভাড়া।")).toBe("ভাড়া");
});

const mouse = { lemma: "mouse", senses: [{ pos: "noun" as const, definition: "a small rodent", example: "a mouse ran by", synonyms: ["rodent"] }] };

test("the card opens on the first web search; the AI waits for its tab", () => {
  const s = run([open(8, "deferred", "word", SEARCHES), { type: "dictionary", id: 8, entry: mouse }]) as Explaining;
  expect(tabs(s)).toEqual(["web:google", "web:bing", "dictionary", "explain"]);
  expect(s.tab).toBe("web:google");
  expect(s.ai).toBe("deferred");
  expect(isWorking(s)).toBe(false);
  expect(copyable(s)).toBe("");

  const dict = reduce(s, { type: "select_tab", tab: "dictionary" }) as Explaining;
  expect(copyable(dict)).toBe('mouse\n1. (noun) a small rodent "a mouse ran by"');
  expect(dict.ai).toBe("deferred");

  const asked = reduce(dict, { type: "select_tab", tab: "explain" }) as Explaining;
  expect(asked.tab).toBe("explain");
  expect(asked.ai).toBe("requested");
  expect(isWorking(asked)).toBe(true);
  expect(copyable(asked)).toBe("");

  const answering = run([ex(8, { ...started, kind: "word", grade_before: null, parts: 1 } as ExplainEvent)], asked) as Explaining;
  expect(answering.ai).toBe("running");
  expect(answering.tab).toBe("explain");
  expect(answering.dictionary?.lemma).toBe("mouse");

  // Going back and forth doesn't ask again.
  const back = reduce(reduce(answering, { type: "select_tab", tab: "web:bing" }), { type: "select_tab", tab: "explain" }) as Explaining;
  expect(back.ai).toBe("running");
});

test("offline, a word opens on its dictionary entry", () => {
  const s = run([open(9, "deferred", "word"), { type: "dictionary", id: 9, entry: mouse }]) as Explaining;
  expect(tabs(s)).toEqual(["dictionary", "explain"]);
  expect(s.tab).toBe("dictionary");
});

test("without a model the AI tab stays, to say how to set one up", () => {
  const s = run([open(10, "off", "word"), { type: "dictionary", id: 10, entry: mouse }]) as Explaining;
  expect(tabs(s)).toEqual(["dictionary", "explain"]);
  const ai = reduce(s, { type: "select_tab", tab: "explain" }) as Explaining;
  expect(ai.tab).toBe("explain");
  expect(ai.ai).toBe("off");
  expect(isWorking(ai)).toBe(false);
});

test("offline with nothing else to show, a passage is explained at once", () => {
  const s = run([open(11, "running"), ex(11, started), ex(11, { type: "part_done", part: 0, text: "Plain." })]) as Explaining;
  expect(tabs(s)).toEqual(["explain"]);
  expect(s.tab).toBe("explain");
  expect(copyable(s)).toBe("");
  const done = run([ex(11, { type: "done", text: "", grade_after: 5, report: { missing: [], added: [] }, timings: null, elapsed_ms: 1 })], s) as Explaining;
  expect(copyable(done)).toBe("Plain.");
  expect(reduce(done, { type: "select_tab", tab: "dictionary" })).toBe(done);
  expect(reduce(done, { type: "select_tab", tab: "web:google" })).toBe(done);
});

test("a new reading level starts over without leaving the tab", () => {
  const first = run([
    open(12, "running"),
    ex(12, started),
    ex(12, { type: "part_done", part: 0, text: "Old." }),
    ex(12, { type: "done", text: "", grade_after: 9, report: { missing: [{ kind: "number", text: "5%" }], added: [] }, timings: null, elapsed_ms: 1 }),
  ]) as Explaining;
  const again = run([ex(12, started)], first) as Explaining;
  expect(answerText(again)).toBe("");
  expect(again.done).toBe(false);
  expect(again.report).toBeNull();
  expect(again.gradeAfter).toBeNull();
  expect(again.tab).toBe("explain");
});

test("a dictionary entry arriving after the model started keeps the open tab", () => {
  const s = run([
    open(13, "running"),
    ex(13, { ...started, kind: "word", parts: 1 } as ExplainEvent),
    { type: "dictionary", id: 13, entry: mouse },
  ]) as Explaining;
  expect(s.tab).toBe("explain");
  expect(s.kind).toBe("word");
});

test("picking a tab outside a request does nothing", () => {
  expect(reduce(initialState, { type: "select_tab", tab: "explain" })).toBe(initialState);
});

describe("Explain and Translate in the AI tab", () => {
  const done: ExplainEvent = { type: "done", text: "x", grade_after: 8, report: { missing: [], added: [] }, timings: null, elapsed_ms: 1 };

  it("switching to Translate drops the explanation and ignores its late events", () => {
    let s = reduce(initialState, open(1, "running"));
    s = reduce(s, ex(1, started));
    s = reduce(s, ex(1, { type: "delta", part: 0, text: "Plain words" }));
    s = reduce(s, { type: "set_task", task: "translate" });
    expect(s.view === "explain" && s.task).toBe("translate");
    expect(s.view === "explain" && s.parts).toEqual([]);
    s = reduce(s, ex(1, { type: "delta", part: 0, text: " more" }));
    expect(s.view === "explain" && s.parts).toEqual([]);
    s = reduce(s, ex(1, { ...started, grade_before: null }, "translate"));
    s = reduce(s, ex(1, { type: "part_done", part: 0, text: "Texto difícil." }, "translate"));
    s = reduce(s, ex(1, done, "translate"));
    expect(s.view === "explain" && copyable(s)).toBe("Texto difícil.");
  });

  it("translations show no reading grade", () => {
    let s = reduce(initialState, open(1, "running"));
    s = reduce(s, { type: "set_task", task: "translate" });
    s = reduce(s, ex(1, started, "translate"));
    s = reduce(s, ex(1, done, "translate"));
    expect(s.view === "explain" && gradeBadge(s)).toBeNull();
  });

  it("Translate is refused when the text is already in that language or it isn't supported", () => {
    for (const translate of [
      { state: "same_language", name: "English", source: EN },
      { state: "unsupported", name: "Thai", source: EN },
    ] as TranslateAvailability[]) {
      const s = reduce(reduce(initialState, open(1, "running", "passage", [], translate)), { type: "set_task", task: "translate" });
      expect(s.view === "explain" && s.task).toBe("explain");
      // Picking a language makes it possible.
      const picked = reduce(s, { type: "set_task", task: "translate", target: { code: "bn", name: "Bangla" } });
      expect(picked.view === "explain" && picked.task).toBe("translate");
      expect(picked.view === "explain" && picked.target?.code).toBe("bn");
    }
  });

  it("picking another language translates again, and the old one's late events are ignored", () => {
    let s = reduce(initialState, open(1, "running"));
    s = reduce(s, { type: "set_task", task: "translate" });
    s = reduce(s, ex(1, started, "translate"));
    s = reduce(s, ex(1, { type: "part_done", part: 0, text: "Texto." }, "translate"));
    s = reduce(s, { type: "set_task", task: "translate", target: { code: "bn", name: "Bangla" } });
    expect(s.view === "explain" && s.parts).toEqual([]);
    // Choosing the same language again changes nothing.
    expect(reduce(s, { type: "set_task", task: "translate", target: { code: "bn", name: "Bangla" } })).toBe(s);
  });

  it("a new selection starts on Explain", () => {
    let s = reduce(initialState, open(1, "running"));
    s = reduce(s, { type: "set_task", task: "translate" });
    s = reduce(s, open(2, "running"));
    expect(s.view === "explain" && s.task).toBe("explain");
  });
});
