// Pairs a passage with the AI's answer, so the card shows the original above the result:
// sentence by sentence for translations (like a bilingual reader), part by part for
// explanations (a rewrite may split or merge sentences).

export interface Pair {
  source: string;
  /** "" while the translation of this sentence hasn't arrived yet. */
  result: string;
}

const segmenter = new Intl.Segmenter(undefined, { granularity: "sentence" });

/** The sentences of `text` (Unicode rules: "।", "。", "؟" end sentences too). */
export function sentences(text: string): string[] {
  return Array.from(segmenter.segment(text), (s) => s.segment.trim()).filter(Boolean);
}

/**
 * Pairs the source and the translation part by part (long text is translated in parts, so
 * parts always line up). Within a part, sentences are paired when both sides have as many;
 * otherwise the whole part is one pair, so nothing is ever paired with the wrong sentence.
 * While a part is still arriving, its sentences so far are paired in order. With
 * `bySentence` false (explanations), every part is one pair.
 */
export function pairUp(sourceParts: string[], resultParts: string[], done: boolean, bySentence = true): Pair[] {
  const pairs: Pair[] = [];
  sourceParts.forEach((source, i) => {
    const result = resultParts[i] ?? "";
    const complete = done || resultParts.slice(i + 1).some((p) => p.length > 0);
    if (!bySentence) {
      pairs.push({ source: source.trim(), result: result.trim() });
      return;
    }
    const from = sentences(source);
    const to = sentences(result);
    if (complete ? from.length === to.length : to.length <= from.length) {
      from.forEach((s, k) => pairs.push({ source: s, result: to[k] ?? "" }));
    } else {
      pairs.push({ source: source.trim(), result: result.trim() });
    }
  });
  return pairs;
}
