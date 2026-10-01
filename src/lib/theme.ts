// Theme and accent on <html>: `.dark` and `data-accent`, as in the Atlas design.
import type { AccentName, Appearance, Theme } from "./types";

export type ThemeChoice = Theme;
export type Accent = AccentName;

export const ACCENTS: Accent[] = ["plum", "saffron", "garnet", "ink"];

export function prefersDark(): boolean {
  return typeof window.matchMedia === "function" && window.matchMedia("(prefers-color-scheme: dark)").matches;
}

export function applyTheme(theme: ThemeChoice, accent: Accent, root: HTMLElement = document.documentElement): void {
  const dark = theme === "dark" || (theme === "system" && prefersDark());
  root.classList.toggle("dark", dark);
  if (accent === "plum") root.removeAttribute("data-accent");
  else root.setAttribute("data-accent", accent);
}

/** Applies the saved appearance and remembers it for the next window's first paint. */
export function applyAppearance(a: Appearance): void {
  applyTheme(a.theme, a.accent);
  const root = document.documentElement;
  root.style.setProperty("--reading-size", `${a.font_size}px`);
  root.style.setProperty("--reading-leading", String(a.line_height));
  root.style.setProperty("--reading-font", `var(--font-reading-${a.reading_font})`);
  try {
    localStorage.setItem("te.theme", a.theme);
    localStorage.setItem("te.accent", a.accent);
  } catch {
    // Storage may be unavailable; the next window just starts with the defaults.
  }
}

/** Re-applies "system" when Windows switches between light and dark. */
export function followSystemTheme(get: () => Appearance | undefined): () => void {
  if (typeof window.matchMedia !== "function") return () => {};
  const mq = window.matchMedia("(prefers-color-scheme: dark)");
  const onChange = () => {
    const a = get();
    if (a?.theme === "system") applyTheme(a.theme, a.accent);
  };
  mq.addEventListener("change", onChange);
  return () => mq.removeEventListener("change", onChange);
}
