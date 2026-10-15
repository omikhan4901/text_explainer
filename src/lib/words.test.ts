import { MAX_HISTORY, MAX_PAGE, WordBook, relativeTime, savableTerm, summary, termKey } from "./words";

describe("savableTerm", () => {
  it("saves a word without its punctuation and a passage as one line", () => {
    expect(savableTerm("“Ubiquitous,”", "word")).toEqual({ term: "Ubiquitous", kind: "word" });
    expect(savableTerm("carte blanche.", "phrase")).toEqual({ term: "carte blanche", kind: "phrase" });
    expect(savableTerm("  The rent is due.  ", "passage")).toEqual({ term: "The rent is due.", kind: "passage" });
    expect(savableTerm("অভিধান।", "word")).toEqual({ term: "অভিধান", kind: "word" });
  });

  it("refuses empty, cut-short and too-long selections", () => {
    expect(savableTerm("", "word")).toBeNull();
    expect(savableTerm("“…”", "word")).toBeNull();
    expect(savableTerm("x", "empty")).toBeNull();
    expect(savableTerm(`${"a".repeat(120)}…`, "passage")).toBeNull();
    expect(savableTerm("ক".repeat(200), "passage")).not.toBeNull();
    expect(savableTerm("ক".repeat(201), "passage")).toBeNull();
  });
});

describe("termKey and summary", () => {
  it("folds case and whitespace", () => {
    expect(termKey("  Carte  BLANCHE\n")).toBe("carte blanche");
    expect(termKey("ÉCOLE")).toBe("école");
  });

  it("summarises with the definition first", () => {
    expect(summary({ definition: "present\neverywhere", explanation: "x" })).toBe("present everywhere");
    expect(summary({ definition: null, explanation: "found everywhere" })).toBe("found everywhere");
    expect(summary({ definition: null, explanation: null })).toBe("");
  });
});

describe("relativeTime", () => {
  const now = Date.UTC(2026, 9, 15, 12);
  const at = (ms: number) => relativeTime(now - ms, now, "en", "just now");
  it("reads like speech, then a date", () => {
    expect(at(10_000)).toBe("just now");
    expect(at(5 * 60_000)).toBe("5 minutes ago");
    expect(at(3 * 3600_000)).toBe("3 hours ago");
    expect(at(24 * 3600_000)).toBe("yesterday");
    expect(at(30 * 24 * 3600_000)).toBe("Sep 15, 2026");
  });
});

describe("WordBook", () => {
  it("upserts by key, keeps the first saved time and given texts", () => {
    let t = 1000;
    const book = new WordBook(() => t);
    const first = book.save({ term: "Ubiquitous", kind: "word", definition: "everywhere", explanation: null });
    t = 2000;
    const second = book.save({ term: " ubiquitous ", kind: "word", definition: null, explanation: "found everywhere" });
    expect(second).toMatchObject({
      id: first.id,
      term: "ubiquitous",
      definition: "everywhere",
      explanation: "found everywhere",
      created_at: 1000,
      updated_at: 2000,
    });
    expect(book.list("", 10, 0).total).toBe(1);
    expect(book.isSaved("UBIQUITOUS")).toBe(true);
    expect(book.remove("Ubiquitous")).toBe(true);
    expect(book.remove("Ubiquitous")).toBe(false);
  });

  it("searches literally, newest first, in capped pages", () => {
    let t = 0;
    const book = new WordBook(() => ++t);
    for (const term of ["50%", "500", "a_b", "axb"]) book.save({ term, kind: "word", definition: null, explanation: null });
    expect(book.list("%", 10, 0).items.map((w) => w.term)).toEqual(["50%"]);
    expect(book.list("_", 10, 0).items.map((w) => w.term)).toEqual(["a_b"]);
    expect(book.list("", 2, 0).items.map((w) => w.term)).toEqual(["axb", "a_b"]);
    for (let i = 0; i < MAX_PAGE + 5; i++) book.save({ term: `w${i}`, kind: "word", definition: null, explanation: null });
    expect(book.list("", 10_000, 0).items).toHaveLength(MAX_PAGE);
    expect(() => book.save({ term: "  ", kind: "word", definition: null, explanation: null })).toThrow();
  });

  it("keeps history only when asked, pruned by age and capped", () => {
    const day = 24 * 3600_000;
    let t = 0;
    const book = new WordBook(() => t);
    expect(book.record("secret", "word", false, 30)).toBe(false);
    book.record("old", "word", true, 30);
    t = 31 * day;
    book.record("new\n line", "phrase", true, 30);
    expect(book.history(10, 0).items.map((l) => l.query)).toEqual(["new line"]);
    for (let i = 0; i < MAX_HISTORY + 3; i++) book.record(`q${i}`, "word", true, 30);
    expect(book.history(1, 0).total).toBe(MAX_HISTORY);
    expect(book.clearHistory()).toBe(MAX_HISTORY);
    expect(book.history(10, 0).total).toBe(0);
  });
});
