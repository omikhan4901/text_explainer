import { Check, Download, Trash2, X } from "lucide-react";
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

/** One model: what it is, whether it fits, and the one action that makes sense now. */
export function ModelRow({ model, recommended = false, compact = false }: { model: ModelInfo; recommended?: boolean; compact?: boolean }) {
  const { settings, info, update } = useData();
  const progress = useDownloadOf(model.id);
  const inUse = settings.model.kind === "catalog" && settings.model.id === model.id;
  const otherDownloading = info.downloading !== null && info.downloading !== model.id;

  const action = progress ? (
    <Button size="sm" variant="ghost" onClick={() => void api.cancelDownload()}>
      <X size={15} /> {t.cancel}
    </Button>
  ) : inUse ? (
    <Tag tone="accent">
      <Check size={13} className="mr-1" /> {t.inUse}
    </Tag>
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
      <Download size={15} />
      {model.partial_bytes > 0 ? t.resume(c.gb(model.partial_bytes)) : t.download(c.gb(model.size_bytes))}
    </Button>
  );

  return (
    <div className={compact ? "" : "py-4"}>
      <div className="flex items-start gap-4">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="font-semibold">{model.name}</span>
            <Tag tone={model.tier === "balanced" ? "accent" : "neutral"}>{t.tier[model.tier]}</Tag>
            {recommended && !compact && <Tag tone="accent">{t.recommended}</Tag>}
          </div>
          <p className="mt-1 text-sm text-muted">
            {model.maker} · {model.params} · {c.gb(model.size_bytes)} ·{" "}
            <button type="button" className="underline-offset-2 hover:underline" onClick={() => void api.openUrl(model.license_url)}>
              {model.license}
            </button>
          </p>
          <p className="mt-1 text-sm text-muted">{model.note}</p>
          <p className="mt-2">
            <Tag tone={fitTone(model.fit)}>{t.fit[model.fit]}</Tag>
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-1">
          {action}
          {model.installed && !progress && (
            <Button size="sm" variant="ghost" aria-label={`${t.delete} ${model.name}`} title={t.delete} onClick={() => void api.deleteModel(model.id).then(() => {})}>
              <Trash2 size={15} />
            </Button>
          )}
        </div>
      </div>
      {progress && (
        <div className="mt-3 space-y-1.5">
          <Progress value={progress.downloaded / progress.total} label={`${t.downloading} ${model.name}`} />
          <p className="text-xs text-muted">{t.remaining(c.gb(progress.downloaded), c.gb(progress.total), c.perSecond(progress.speed))}</p>
        </div>
      )}
    </div>
  );
}
