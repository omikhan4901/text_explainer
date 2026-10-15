import { LockSimple } from "@phosphor-icons/react";
import { Button, LinkButton } from "../../components/ui/Button";
import { Kbd } from "../../components/ui/Kbd";
import { Group, Section } from "../../components/ui/Section";
import { strings } from "../../i18n";
import { api } from "../../lib/ipc";
import type { PageId } from "../App";
import { useData } from "../data";

const t = strings.home;

export function HomePage({ go }: { go: (p: PageId) => void }) {
  const { settings, info } = useData();
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

  return (
    <>
      <h1 className="sr-only">{t.title}</h1>
      {info.hotkey === "taken" && (
        <div role="alert" className="mb-6 flex flex-wrap items-center justify-between gap-3 rounded-lg bg-warn-soft px-3.5 py-2.5 text-[13px] text-warn">
          <span>{t.hotkeyTaken(settings.hotkey)}</span>
          <LinkButton className="text-warn" onClick={() => go("shortcuts")}>
            {t.hotkeyTakenAction}
          </LinkButton>
        </div>
      )}

      <header className="mb-10">
        {ready ? (
          <>
            <p className="text-[26px] leading-[1.25] font-semibold tracking-[-0.025em] text-balance">{info.paused ? t.pausedHeadline : t.headline}</p>
            {info.paused ? (
              <p className="mt-3 text-[14px] text-muted">{t.pausedBody}</p>
            ) : (
              <div className="mt-4 space-y-2 text-[14px] text-muted">
                <p className="flex flex-wrap items-center gap-2">
                  {t.waySelect} <Kbd keys={settings.hotkey} size="lg" />
                </p>
                {settings.double_copy && (
                  <p className="flex flex-wrap items-center gap-2">
                    {t.wayCopy} <Kbd keys="Ctrl+C" /> <Kbd keys="C" />
                  </p>
                )}
              </div>
            )}
          </>
        ) : (
          <>
            <p className="text-[26px] leading-[1.25] font-semibold tracking-[-0.025em] text-balance">{t.needsModelHeadline}</p>
            <p className="mt-3 max-w-[48ch] text-[14px] text-muted">{t.needsModelBody}</p>
            <Button variant="primary" className="mt-5" onClick={() => go("model")}>
              {t.chooseModel}
            </Button>
          </>
        )}
      </header>

      <Section title={t.tryTitle} description={t.tryBody}>
        <Group>
          <p className="px-5 pt-4 pb-4 leading-relaxed select-text" style={{ fontFamily: "var(--reading-font)" }}>
            {t.sample}
          </p>
          <div className="flex items-center justify-between gap-3 py-2.5 pr-2.5 pl-5">
            <span className="text-[12px] text-muted">{t.sampleSource}</span>
            <Button size="sm" disabled={!ready} onClick={() => void api.explainText(t.sample)}>
              {t.explainSample}
            </Button>
          </div>
        </Group>
      </Section>

      <Section title={t.model}>
        <Group>
          <div className="flex items-center justify-between gap-4 px-4 py-3">
            <div className="min-w-0">
              <p className="truncate font-medium">{modelName}</p>
              <p className="mt-0.5 text-[13px] text-muted">{ready ? t.modelRuns : t.needsModelBody}</p>
            </div>
            <Button size="sm" onClick={() => go("model")}>
              {t.changeModel}
            </Button>
          </div>
        </Group>
      </Section>

      <p className="mt-8 flex items-center gap-2 text-[12px] text-muted">
        <LockSimple size={13} aria-hidden /> {settings.offline_only ? t.privacy : t.privacyWeb}
      </p>
    </>
  );
}
