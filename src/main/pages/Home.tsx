import { Lock } from "lucide-react";
import { Button } from "../../components/ui/Button";
import { Card, CardTitle } from "../../components/ui/Card";
import { Kbd } from "../../components/ui/Kbd";
import { strings } from "../../i18n";
import { api } from "../../lib/ipc";
import type { PageId } from "../App";
import { PageTitle } from "../App";
import { useData } from "../data";

const t = strings.home;

export function HomePage({ go }: { go: (p: PageId) => void }) {
  const { settings, info, refresh } = useData();
  const model = settings.model;
  const modelName =
    model.kind === "catalog"
      ? (info.models.find((m) => m.id === model.id)?.name ?? model.id)
      : model.kind === "file"
        ? (model.path.split(/[\\/]/).pop() ?? model.path)
        : model.kind === "endpoint"
          ? `${model.model} (${strings.model.endpoint})`
          : t.noModel;
  const ready = model.kind !== "none";

  const togglePause = async () => {
    await api.setPaused(!info.paused);
    await refresh();
  };

  return (
    <>
      <PageTitle>{t.title}</PageTitle>
      {!info.hotkey_active && (
        <div role="alert" className="mb-4 flex flex-wrap items-center justify-between gap-3 rounded-xl bg-warn-soft px-4 py-3 text-sm text-warn-text">
          <span>{t.hotkeyTaken(settings.hotkey)}</span>
          <button type="button" onClick={() => go("shortcuts")} className="font-semibold underline-offset-2 hover:underline">
            {t.hotkeyTakenAction}
          </button>
        </div>
      )}
      <div className="grid gap-4 md:grid-cols-5">
        <div className={`rounded-[var(--radius-card)] p-6 md:col-span-3 ${ready && !info.paused ? "bg-accent text-on-accent" : "border border-border bg-surface"}`}>
          <p className="text-sm font-semibold opacity-90">{ready ? (info.paused ? t.paused : t.ready) : t.needsModel}</p>
          <p className="mt-6 font-display text-2xl leading-snug font-semibold">
            {ready ? (info.paused ? t.pausedBody : t.readyBody(settings.hotkey)) : t.needsModelBody}
          </p>
          <div className="mt-6">
            {ready ? (
              <button
                type="button"
                onClick={() => void togglePause()}
                className={`h-9 rounded-lg px-4 text-sm font-semibold ${info.paused ? "bg-accent text-on-accent" : "bg-on-accent/15 hover:bg-on-accent/25"}`}
              >
                {info.paused ? t.resume : t.pause}
              </button>
            ) : (
              <Button variant="primary" onClick={() => go("model")}>
                {t.chooseModel}
              </Button>
            )}
          </div>
        </div>
        <Card className="md:col-span-2">
          <p className="text-sm text-muted">{t.model}</p>
          <p className="mt-6 font-display text-xl font-semibold break-words">{modelName}</p>
          <button type="button" onClick={() => go("model")} className="mt-2 text-sm font-medium text-accent underline-offset-2 hover:underline">
            {t.changeModel}
          </button>
        </Card>
      </div>

      <Card className="mt-4">
        <CardTitle>{t.ways}</CardTitle>
        <ul className="space-y-3 text-sm">
          <li className="flex flex-wrap items-center gap-2">
            {t.waySelect} <Kbd keys={settings.hotkey} />
          </li>
          {settings.double_copy && (
            <li className="flex flex-wrap items-center gap-2">
              {t.wayCopy} <Kbd keys="Ctrl+C" /> <Kbd keys="C" />
            </li>
          )}
        </ul>
      </Card>

      <Card className="mt-4">
        <CardTitle>{t.tryTitle}</CardTitle>
        <p className="mb-3 text-sm text-muted">{t.tryBody}</p>
        <blockquote className="rounded-xl border border-border bg-surface-2 p-4 leading-relaxed select-text">{t.sample}</blockquote>
        <div className="mt-4">
          <Button disabled={!ready} onClick={() => void api.explainText(t.sample)}>
            {t.explainSample}
          </Button>
        </div>
      </Card>

      <p className="mt-6 flex items-center gap-2 text-sm text-muted">
        <Lock size={14} aria-hidden /> {t.privacy}
      </p>
    </>
  );
}
