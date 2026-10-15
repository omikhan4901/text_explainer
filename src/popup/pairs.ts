// Pairs each sentence of a passage with its translation, so the card can show the
// original above the translation, sentence by sentence (like a bilingual reader).

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
 * While a part is still arriving, its sentences so far are paired in order.
 */
export function pairUp(sourceParts: string[], resultParts: string[], done: boolean): Pair[] {
  const pairs: Pair[] = [];
  sourceParts.forEach((source, i) => {
    const result = resultParts[i] ?? "";
    const complete = done || resultParts.slice(i + 1).some((p) => p.length > 0);
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
