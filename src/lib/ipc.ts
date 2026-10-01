// Calls into the Rust app. Outside the app (plain browser: development, tests,
// screenshots) a mock answers instead, so every screen can be built and checked anywhere.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AppInfo, Level, Settings } from "./types";
import { mock } from "./mock";

export const inApp = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return inApp ? invoke<T>(command, args) : mock.call<T>(command, args);
}

/** Subscribes to an app event; resolves to an unsubscribe function. */
export function on<T>(event: string, handler: (payload: T) => void): Promise<() => void> {
  if (!inApp) return Promise.resolve(mock.on(event, handler as (p: unknown) => void));
  return listen<T>(event, (e) => handler(e.payload));
}

export const api = {
  getSettings: () => call<Settings>("get_settings"),
  saveSettings: (settings: Settings) => call<Settings>("save_settings", { settings }),
  appInfo: () => call<AppInfo>("app_info"),
  downloadModel: (id: string, activate: boolean) => call<void>("download_model", { id, activate }),
  cancelDownload: () => call<void>("cancel_download"),
  deleteModel: (id: string) => call<void>("delete_model", { id }),
  unloadModel: () => call<void>("unload_model"),
  popupResize: (height: number) => call<void>("popup_resize", { height }),
  popupClose: () => call<void>("popup_close"),
  popupPin: (pinned: boolean) => call<void>("popup_pin", { pinned }),
  setLevel: (level: Level) => call<void>("set_level", { level }),
  copyText: (text: string) => call<void>("copy_text", { text }),
  explainText: (text: string) => call<void>("explain_text", { text }),
  openMain: () => call<void>("open_main"),
  openLogs: () => call<void>("open_logs"),
  openUrl: (url: string) => call<void>("open_url", { url }),
  validateHotkey: (hotkey: string) => call<void>("validate_hotkey", { hotkey }),
  setPaused: (paused: boolean) => call<void>("set_paused", { paused }),
};
