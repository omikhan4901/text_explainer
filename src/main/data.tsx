// Settings, app info and download progress for the main window, shared via context.
import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import { api, on } from "../lib/ipc";
import { applyAppearance, followSystemTheme } from "../lib/theme";
import type { AppInfo, DownloadEvent, Settings } from "../lib/types";

interface Data {
  settings: Settings;
  info: AppInfo;
  download: DownloadEvent | null;
  error: string | null;
  clearError: () => void;
  save: (next: Settings) => Promise<boolean>;
  update: (patch: Partial<Settings>) => Promise<boolean>;
  refresh: () => Promise<void>;
}

const Ctx = createContext<Data | null>(null);

export function useData(): Data {
  const d = useContext(Ctx);
  if (!d) throw new Error("useData outside DataProvider");
  return d;
}

function load(): Promise<[Settings, AppInfo]> {
  return Promise.all([api.getSettings(), api.appInfo()]);
}

export function DataProvider({ children, fallback }: { children: ReactNode; fallback?: ReactNode }) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [download, setDownload] = useState<DownloadEvent | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Read only from event handlers (system theme changes).
  const appearanceRef = useRef<Settings["appearance"] | undefined>(undefined);

  const apply = useCallback(([s, i]: [Settings, AppInfo]) => {
    setSettings(s);
    setInfo(i);
    applyAppearance(s.appearance);
  }, []);
  const refresh = useCallback(() => load().then(apply), [apply]);

  useEffect(() => {
    void load().then(apply);
    const stopTheme = followSystemTheme(() => appearanceRef.current);
    const offSettings = on<null>("te://settings", () => void load().then(apply));
    const offDownload = on<DownloadEvent>("te://download", (e) => {
      setDownload(e);
      if (e.state !== "progress") void load().then(apply);
    });
    return () => {
      stopTheme();
      void offSettings.then((f) => f());
      void offDownload.then((f) => f());
    };
  }, [apply]);

  useEffect(() => {
    appearanceRef.current = settings?.appearance;
  }, [settings]);

  if (!settings || !info) return <>{fallback}</>;

  const save = async (next: Settings) => {
    const before = settings;
    setSettings(next);
    applyAppearance(next.appearance);
    try {
      setSettings(await api.saveSettings(next));
      return true;
    } catch (e) {
      setError(String(e));
      setSettings(before);
      applyAppearance(before.appearance);
      return false;
    }
  };
  const value: Data = {
    settings,
    info,
    download,
    error,
    clearError: () => setError(null),
    save,
    update: (patch) => save({ ...settings, ...patch }),
    refresh,
  };
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

/** Progress of the download of `id`, if one is running. */
export function useDownloadOf(id: string): { downloaded: number; total: number; speed: number } | null {
  const { download, info } = useData();
  if (download?.state === "progress" && download.id === id) {
    return { downloaded: download.downloaded, total: download.total, speed: download.bytes_per_sec };
  }
  if (info.downloading === id) return { downloaded: 0, total: 1, speed: 0 };
  return null;
}
