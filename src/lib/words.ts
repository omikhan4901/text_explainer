// The Word book on the front end: which term the card saves, how rows are summarised and
// dated, and an in-memory book with the same rules as `history.rs` (used by the mock).
import type { Lookup, NewWord, Page, SavedWord, SelectionKind, WordKind } from "./types";

/** Limits enforced by the app (`history.rs`). */
export const MAX_TERM_CHARS = 200;
export const MAX_QUERY_CHARS = 300;
export const MAX_HISTORY = 2000;
export const MAX_PAGE = 200;
const DAY_MS = 24 * 60 * 60 * 1000;

/** “Ubiquitous,” → Ubiquitous */
export function bareTerm(s: string): string {
  return s.trim().replace(/^[^\p{L}\p{M}\p{N}]+|[^\p{L}\p{M}\p{N}]+$/gu, "");
}

const oneLine = (s: string) => s.trim().split(/\s+/u).filter(Boolean).join(" ");

/** The key that makes "Ubiquitous" and " ubiquitous " one saved word. */
export function termKey(s: string): string {
  return oneLine(s).toLowerCase();
}

const chars = (s: string) => [...s].length;

/** What the card saves for its header text, or null when it can't be saved: an empty
 * selection, a header cut short (the selection was long) or more than 200 characters. */
export function savableTerm(query: string, kind: SelectionKind): { term: string; kind: WordKind } | null {
  if (kind === "empty" || query.endsWith("…")) return null;
  const term = kind === "passage" ? oneLine(query) : bareTerm(query);
  if (!term || chars(term) > MAX_TERM_CHARS) return null;
  return { term, kind };
}

/** One line for a list row: the definition, else the explanation. */
export function summary(word: Pick<SavedWord, "definition" | "explanation">): string {
  return oneLine(word.definition ?? word.explanation ?? "");
}

/** "just now", "5 minutes ago", "yesterday", then a date. */
export function relativeTime(then: number, now: number, locale: string, justNow: string): string {
  const seconds = Math.round((now - then) / 1000);
  if (seconds < 45) return justNow;
  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return rtf.format(-minutes, "minute");
  const hours = Math.round(minutes / 60);
  if (hours < 24) return rtf.format(-hours, "hour");
  const days = Math.round(hours / 24);
  if (days < 7) return rtf.format(-days, "day");
  return shortDate(then, locale);
}

export function shortDate(ms: number, locale: string): string {
  return new Intl.DateTimeFormat(locale, { day: "numeric", month: "short", year: "numeric" }).format(ms);
}

const clampPage = (limit: number) => Math.min(Math.max(Math.floor(limit) || 1, 1), MAX_PAGE);
const text = (s: string | null | undefined) => {
  const t = s?.trim();
  return t ? [...t].slice(0, 8000).join("") : null;
};

/** An in-memory word book with the app's rules: one entry per key, newest first, history
 * pruned by age and capped. */
export class WordBook {
  private words: SavedWord[] = [];
  private lookups: Lookup[] = [];
  private nextId = 1;

  constructor(private now: () => number = Date.now) {}

  save(word: NewWord): SavedWord {
    const term = oneLine(word.term);
    if (!term) throw "there is nothing to save";
    if (chars(term) > MAX_TERM_CHARS) throw `that is too long to save (at most ${MAX_TERM_CHARS} characters)`;
    const key = termKey(term);
    const now = this.now();
    const old = this.words.find((w) => termKey(w.term) === key);
    const saved: SavedWord = {
      id: old?.id ?? this.nextId++,
      term,
      kind: word.kind,
      definition: text(word.definition) ?? old?.definition ?? null,
      explanation: text(word.explanation) ?? old?.explanation ?? null,
      context: old?.context ?? null,
      created_at: old?.created_at ?? now,
      updated_at: now,
    };
    this.words = old ? this.words.map((w) => (w === old ? saved : w)) : [...this.words, saved];
    return { ...saved };
  }

  remove(term: string): boolean {
    const before = this.words.length;
    this.words = this.words.filter((w) => termKey(w.term) !== termKey(term));
    return this.words.length < before;
  }

  isSaved(term: string): boolean {
    return this.words.some((w) => termKey(w.term) === termKey(term));
  }

  list(search: string, limit: number, offset: number): Page<SavedWord> {
    const needle = termKey(search);
    const matches = this.words
      .filter((w) => !needle || [termKey(w.term), w.definition ?? "", w.explanation ?? ""].some((f) => f.toLowerCase().includes(needle)))
      .sort((a, b) => b.created_at - a.created_at || b.id - a.id);
    return { items: matches.slice(offset, offset + clampPage(limit)).map((w) => ({ ...w })), total: matches.length };
  }

  record(query: string, kind: WordKind, keep: boolean, days: number): boolean {
    const q = [...oneLine(query)].slice(0, MAX_QUERY_CHARS).join("");
    if (!keep || !q) return false;
    const now = this.now();
    const cutoff = now - Math.min(Math.max(days, 1), 365) * DAY_MS;
    this.lookups = [{ id: this.nextId++, query: q, kind, created_at: now }, ...this.lookups.filter((l) => l.created_at >= cutoff)].slice(0, MAX_HISTORY);
    return true;
  }

  history(limit: number, offset: number): Page<Lookup> {
    return { items: this.lookups.slice(offset, offset + clampPage(limit)).map((l) => ({ ...l })), total: this.lookups.length };
  }

  clearHistory(): number {
    const n = this.lookups.length;
    this.lookups = [];
    return n;
  }
}
