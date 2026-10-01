// Theme and accent on <html>: `.dark` and `data-accent`, as in the Atlas design.
export type ThemeChoice = "system" | "light" | "dark";
export type Accent = "plum" | "saffron" | "garnet" | "ink";

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
