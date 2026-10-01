import { shortcutFromEvent } from "./Shortcuts";

const ev = (code: string, mods: Partial<Record<"ctrlKey" | "altKey" | "shiftKey" | "metaKey", boolean>> = {}) => ({
  code,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  ...mods,
});

test("builds shortcuts in the app's format", () => {
  expect(shortcutFromEvent(ev("Space", { ctrlKey: true, shiftKey: true }))).toBe("Ctrl+Shift+Space");
  expect(shortcutFromEvent(ev("KeyE", { altKey: true }))).toBe("Alt+E");
  expect(shortcutFromEvent(ev("Digit1", { ctrlKey: true }))).toBe("Ctrl+1");
  expect(shortcutFromEvent(ev("F8"))).toBe("F8");
});

test("waits while only modifiers are held", () => {
  expect(shortcutFromEvent(ev("ControlLeft", { ctrlKey: true }))).toBeNull();
  expect(shortcutFromEvent(ev("ShiftRight", { shiftKey: true }))).toBeNull();
  expect(shortcutFromEvent(ev("Tab"))).toBeNull();
});
