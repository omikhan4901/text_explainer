import { useEffect, useState } from "react";
import { Button } from "../components/ui/Button";
import { Progress } from "../components/ui/Progress";
import { strings } from "../i18n";
import { on } from "../lib/ipc";
import type { UpdateProgress as Update } from "../lib/types";

const t = strings.update;

/** Covers the main window while an update the person accepted downloads and installs, so
 * the wait before the installer opens is never silent. */
export function UpdateProgress() {
  const [update, setUpdate] = useState<Update | null>(null);
  useEffect(() => {
    const off = on<Update>("te://update", setUpdate);
    return () => void off.then((f) => f());
  }, []);
  if (!update) return null;

  const done = update.state === "downloading" && update.total ? update.downloaded / update.total : update.state === "installing" ? 1 : 0;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-overlay p-6">
      <div role="dialog" aria-modal aria-labelledby="update-title" className="w-full max-w-sm rounded-xl bg-surface p-6 text-text shadow-[var(--shadow-card)]">
        <h2 id="update-title" className="text-[15px] font-semibold">
          {t.title(update.version)}
        </h2>
        {update.state === "failed" ? (
          <>
            <p className="mt-2 text-[13px] text-muted">{t.failed}</p>
            <p className="mt-2 text-[12px] break-words text-muted">{update.message}</p>
            <div className="mt-5 flex justify-end">
              <Button size="sm" onClick={() => setUpdate(null)}>
                {t.close}
              </Button>
            </div>
          </>
        ) : (
          <>
            <div className="mt-4">
              <Progress value={done} label={t.downloading} />
            </div>
            <p role="status" className="mt-2 text-[13px] text-muted">
              {update.state === "installing"
                ? t.installing
                : update.total
                  ? `${t.downloading} · ${t.of(t.mb(update.downloaded), t.mb(update.total))}`
                  : `${t.downloading} · ${t.mb(update.downloaded)}`}
            </p>
          </>
        )}
      </div>
    </div>
  );
}
