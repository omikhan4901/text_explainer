import { CheckCircle, WarningCircle, XCircle } from "@phosphor-icons/react";
import { useCallback, useEffect, useState } from "react";
import { Button } from "../components/ui/Button";
import { strings } from "../i18n";
import { api } from "../lib/ipc";
import type { FoundModel } from "../lib/types";
import { useData } from "./data";

const t = strings.model;
const c = strings.common;
const FIT_ICON = { comfortable: CheckCircle, tight: WarningCircle, too_big: XCircle } as const;
const FIT_COLOR = { comfortable: "text-success", tight: "text-warn", too_big: "text-danger" } as const;

/** Looks for models already on this computer once, and again on request. */
export function useFoundModels() {
  const [found, setFound] = useState<FoundModel[] | null>(null);
  const [round, setRound] = useState(0);
  useEffect(() => {
    let live = true;
    api
      .findModels()
      .then((list) => live && setFound(list))
      .catch(() => live && setFound([]));
    return () => {
      live = false;
    };
  }, [round]);
  const search = useCallback(() => {
    setFound(null);
    setRound((r) => r + 1);
  }, []);
  return { found, search };
}

/** Models found on this computer, each with whether it fits and a Use button. */
export function FoundModels({ found, limit, onUse }: { found: FoundModel[] | null; limit?: number; onUse?: () => void }) {
  const { settings, update } = useData();
  if (found === null) {
    return (
      <div className="px-4 py-3.5" role="status">
        <div className="skeleton-line h-2.5 w-48 rounded-full" />
        <p className="mt-2 text-[13px] text-muted">{t.searching}</p>
      </div>
    );
  }
  if (found.length === 0) return <p className="px-4 py-3.5 text-[13px] text-muted">{t.noneFound}</p>;
  return (
    <>
      {found.slice(0, limit).map((m) => {
        const inUse = settings.model.kind === "file" && settings.model.path === m.path;
        const FitIcon = FIT_ICON[m.fit];
        return (
          <div key={m.path} className="flex items-start gap-4 px-4 py-3">
            <div className="min-w-0 flex-1">
              <p className="truncate font-medium" title={m.path}>
                {m.name}
              </p>
              <p className="mt-0.5 text-[12px] text-muted">
                {m.source} · {c.gb(m.size_bytes)}
              </p>
              <p className={`mt-1 flex items-center gap-1.5 text-[12px] font-medium ${FIT_COLOR[m.fit]}`}>
                <FitIcon size={14} weight="fill" aria-hidden /> {t.fit[m.fit]}
              </p>
            </div>
            {inUse ? (
              <span className="inline-flex h-7 shrink-0 items-center gap-1.5 px-1 text-[13px] font-medium text-success">
                <CheckCircle size={15} weight="fill" aria-hidden /> {t.inUse}
              </span>
            ) : (
              <Button
                size="sm"
                className="shrink-0"
                disabled={m.fit === "too_big"}
                aria-label={`${t.use} ${m.name}`}
                onClick={() => void update({ model: { kind: "file", path: m.path } }).then((ok) => ok && onUse?.())}
              >
                {t.use}
              </Button>
            )}
          </div>
        );
      })}
    </>
  );
}
