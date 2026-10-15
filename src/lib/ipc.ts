// Calls into the Rust app. Outside the app (plain browser: development, tests,
// screenshots) a mock answers instead, so every screen can be built and checked anywhere.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AiTask, AppInfo, CardSize, ExportFormat, FoundModel, Level, Lookup, NewWord, Page, SavedWord, Settings, UpdateStatus } from "./types";

/** Where the card's content area is, in CSS pixels from the window's top left. */
export interface WebBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}
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
  popupResize: (width: number, height: number) => call<void>("popup_resize", { width, height }),
  popupDrag: (kind: "move" | "resize") => call<void>("popup_drag", { kind }),
  setCardSize: (size: CardSize | null) => call<void>("set_card_size", { size }),
  popupClose: () => call<void>("popup_close"),
  setLevel: (level: Level) => call<void>("set_level", { level }),
  explainNow: (id: number) => call<void>("explain_now", { id }),
  setAiTask: (id: number, task: AiTask, to?: string) => call<void>("set_ai_task", { id, task, to: to ?? null }),
  popupFocus: (focus: boolean) => call<void>("popup_focus", { focus }),
  webShow: (source: string, bounds: WebBounds, background: string | null) => call<string>("web_show", { source, bounds, background }),
  webHide: () => call<void>("web_hide"),
  webOpenInBrowser: (source: string) => call<void>("web_open_in_browser", { source }),
  validateSource: (url: string) => call<void>("validate_source", { url }),
  findModels: () => call<FoundModel[]>("find_models"),
  copyText: (text: string) => call<void>("copy_text", { text }),
  explainText: (text: string) => call<void>("explain_text", { text }),
  openMain: () => call<void>("open_main"),
  openLogs: () => call<void>("open_logs"),
  openUrl: (url: string) => call<void>("open_url", { url }),
  validateHotkey: (hotkey: string) => call<void>("validate_hotkey", { hotkey }),
  setPaused: (paused: boolean) => call<void>("set_paused", { paused }),
  lookUp: (text: string) => call<void>("look_up", { text }),
  lookupHide: () => call<void>("lookup_hide"),
  lastApp: () => call<string | null>("last_app"),
  validateApp: (name: string) => call<string>("validate_app", { name }),
  saveWord: (word: NewWord) => call<SavedWord>("save_word", { word }),
  removeWord: (term: string) => call<boolean>("remove_word", { term }),
  isSaved: (term: string) => call<boolean>("is_saved", { term }),
  listWords: (search: string, limit: number, offset: number) => call<Page<SavedWord>>("list_words", { search, limit, offset }),
  listHistory: (limit: number, offset: number) => call<Page<Lookup>>("list_history", { limit, offset }),
  clearHistory: () => call<number>("clear_history"),
  exportWords: (format: ExportFormat, path: string) => call<number>("export_words", { format, path }),
  speak: (text: string) => call<void>("speak", { text }),
  wordBookOnDisk: () => call<boolean>("word_book_on_disk"),
  checkUpdates: () => call<UpdateStatus>("check_updates"),
};
