import { pairUp, sentences } from "./pairs";

describe("sentences", () => {
  it("splits English, Bangla and Chinese", () => {
    expect(sentences("Pay by the 5th. A fee of 5% is added.")).toEqual(["Pay by the 5th.", "A fee of 5% is added."]);
    expect(sentences("৫ তারিখের মধ্যে দিন। না হলে ফি লাগবে।")).toEqual(["৫ তারিখের মধ্যে দিন।", "না হলে ফি লাগবে।"]);
    expect(sentences("请在5日前付款。否则加收5%。")).toEqual(["请在5日前付款。", "否则加收5%。"]);
    expect(sentences("  ")).toEqual([]);
  });
});

describe("pairUp", () => {
  const src = "Pay by the 5th. A fee of 5% is added.";

  it("pairs sentence by sentence when the counts match", () => {
    expect(pairUp([src], ["Paga antes del 5. Se añade un 5%."], true)).toEqual([
      { source: "Pay by the 5th.", result: "Paga antes del 5." },
      { source: "A fee of 5% is added.", result: "Se añade un 5%." },
    ]);
  });

  it("keeps the whole part together when the counts differ", () => {
    expect(pairUp([src], ["Paga antes del 5; si no, se añade un 5%."], true)).toEqual([{ source: src, result: "Paga antes del 5; si no, se añade un 5%." }]);
  });

  it("pairs a part that is still arriving in order, the rest waiting", () => {
    expect(pairUp([src], ["Paga antes del 5. Se"], false)).toEqual([
      { source: "Pay by the 5th.", result: "Paga antes del 5." },
      { source: "A fee of 5% is added.", result: "Se" },
    ]);
    expect(pairUp([src], [""], false)).toEqual([
      { source: "Pay by the 5th.", result: "" },
      { source: "A fee of 5% is added.", result: "" },
    ]);
  });

  it("pairs each part with its own translation", () => {
    const pairs = pairUp(["One. Two.", "Three."], ["Uno. Dos.", "Tres."], true);
    expect(pairs.map((p) => p.result)).toEqual(["Uno.", "Dos.", "Tres."]);
    // An earlier part is complete once a later one has text.
    expect(pairUp(["One. Two.", "Three."], ["Uno y dos.", "Tr"], false)[0]).toEqual({ source: "One. Two.", result: "Uno y dos." });
  });

  it("keeps whole parts together for explanations", () => {
    expect(pairUp([src, "Next part."], ["Pay by the 5th; if not, 5% more.", "Next."], true, false)).toEqual([
      { source: src, result: "Pay by the 5th; if not, 5% more." },
      { source: "Next part.", result: "Next." },
    ]);
  });
});
