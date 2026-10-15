import { CheckCircle, DownloadSimple, Trash, WarningCircle, X, XCircle } from "@phosphor-icons/react";
import { Button } from "../components/ui/Button";
import { Progress } from "../components/ui/Progress";
import { Tag } from "../components/ui/Tag";
import { strings } from "../i18n";
import { api } from "../lib/ipc";
import type { ModelInfo } from "../lib/types";
import { useData, useDownloadOf } from "./data";

const t = strings.model;
const c = strings.common;

export function fitTone(fit: ModelInfo["fit"]) {
  return fit === "comfortable" ? "success" : fit === "tight" ? "warn" : "danger";
}

const FIT_ICON = { comfortable: CheckCircle, tight: WarningCircle, too_big: XCircle } as const;
const FIT_COLOR = { comfortable: "text-success", tight: "text-warn", too_big: "text-danger" } as const;

/** One model: what it is, whether it fits, and the one action that makes sense now. */
export function ModelRow({ model, recommended = false, compact = false }: { model: ModelInfo; recommended?: boolean; compact?: boolean }) {
  const { settings, info, update } = useData();
  const progress = useDownloadOf(model.id);
  const inUse = settings.model.kind === "catalog" && settings.model.id === model.id;
  const otherDownloading = info.downloading !== null && info.downloading !== model.id;
  const FitIcon = FIT_ICON[model.fit];

  const action = progress ? (
    <Button size="sm" variant="ghost" onClick={() => void api.cancelDownload()}>
      <X size={13} /> {t.cancel}
    </Button>
  ) : inUse ? (
    <span className="inline-flex h-7 items-center gap-1.5 px-1 text-[13px] font-medium text-success">
      <CheckCircle size={15} weight="fill" aria-hidden /> {t.inUse}
    </span>
  ) : model.installed ? (
    <Button size="sm" variant="primary" onClick={() => void update({ model: { kind: "catalog", id: model.id } })}>
      {t.use}
    </Button>
  ) : (
    <Button
      size="sm"
      variant={recommended ? "primary" : "secondary"}
      disabled={otherDownloading || model.fit === "too_big"}
      onClick={() => void api.downloadModel(model.id, true).catch(() => {})}
    >
      <DownloadSimple size={14} />
      {model.partial_bytes > 0 ? t.resume(c.gb(model.partial_bytes)) : t.download(c.gb(model.size_bytes))}
    </Button>
  );

  return (
    <div className={compact ? "px-4 py-4" : "px-4 py-3.5"}>
      <div className="flex items-start gap-4">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="font-semibold">{model.name}</span>
            {recommended && <Tag tone="accent">{t.recommended}</Tag>}
          </div>
          <p className="mt-0.5 text-[12px] text-muted">
            {t.tier[model.tier]} · {model.maker} · {model.params} · {c.gb(model.size_bytes)} ·{" "}
            <button
              type="button"
              className="underline decoration-current/30 underline-offset-2 hover:decoration-current"
              onClick={() => void api.openUrl(model.license_url)}
            >
              {model.license}
            </button>
          </p>
          <p className="mt-1.5 max-w-[56ch] text-[13px] text-muted">{model.note}</p>
          <p className={`mt-1.5 flex items-center gap-1.5 text-[12px] font-medium ${FIT_COLOR[model.fit]}`}>
            <FitIcon size={14} weight="fill" aria-hidden /> {t.fit[model.fit]}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-0.5">
          {action}
          {model.installed && !progress && (
            <Button
              size="sm"
              variant="ghost"
              className="w-7 px-0"
              aria-label={`${t.delete} ${model.name}`}
              title={t.delete}
              onClick={() => void api.deleteModel(model.id).then(() => {})}
            >
              <Trash size={14} />
            </Button>
          )}
        </div>
      </div>
      {progress && (
        <div className="mt-3 space-y-1.5">
          <Progress value={progress.downloaded / progress.total} label={`${t.downloading} ${model.name}`} />
          <p className="font-mono text-[11px] text-muted tabular-nums">
            {t.remaining(c.gb(progress.downloaded), c.gb(progress.total), c.perSecond(progress.speed))}
          </p>
        </div>
      )}
    </div>
  );
}
