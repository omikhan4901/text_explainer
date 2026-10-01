// Runs before the first paint so the window never flashes the wrong theme. The saved
// choice is mirrored in localStorage by the settings page; the app's settings file is
// the source of truth and is applied again once loaded.
import { applyTheme, type Accent, type ThemeChoice } from "./theme";

let theme: ThemeChoice = "system";
let accent: Accent = "plum";
try {
  theme = (localStorage.getItem("te.theme") as ThemeChoice | null) ?? "system";
  accent = (localStorage.getItem("te.accent") as Accent | null) ?? "plum";
} catch {
  // Storage can be unavailable; the defaults are fine.
}
applyTheme(theme, accent);
