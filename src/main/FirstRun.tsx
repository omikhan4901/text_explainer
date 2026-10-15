import { open } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { Button, LinkButton } from "../components/ui/Button";
import { Kbd } from "../components/ui/Kbd";
import { Progress } from "../components/ui/Progress";
import { Group } from "../components/ui/Section";
import { Switch } from "../components/ui/Switch";
import { strings } from "../i18n";
import { api, inApp } from "../lib/ipc";
import { Reading } from "../popup/Popup";
import { Mark } from "./App";
import { useData, useDownloadOf } from "./data";
import { ModelRow } from "./ModelRow";

const t = strings.firstRun;
const STEPS = 3;

/** Three calm steps: what it does, get a model, try it. */
export function FirstRun({ onSeeModels }: { onSeeModels: () => void }) {
  const { settings, info, update } = useData();
  const [step, setStep] = useState(0);
  const recommended = info.models.find((m) => m.id === info.recommended) ?? info.models[0];
  const progress = useDownloadOf(recommended?.id ?? "");
  const hasModel = settings.model.kind !== "none";

  const finish = () => void update({ first_run_done: true });
  const chooseFile = async () => {
    if (!inApp) return;
    const path = await open({ multiple: false, directory: false, filters: [{ name: "GGUF model", extensions: ["gguf"] }] });
    if (typeof path === "string") {
      await update({ model: { kind: "file", path } });
      setStep(2);
    }
  };

  return (
    <div className="flex min-h-dvh flex-col bg-bg px-10 py-8">
      <div className="flex items-center justify-between">
        <span className="flex items-center gap-2.5">
          <Mark />
          <span className="text-[13px] font-semibold">{strings.app.name}</span>
        </span>
        <ol className="flex items-center gap-1.5" aria-label={t.steps}>
          {Array.from({ length: STEPS }, (_, i) => (
            <li
              key={i}
              aria-current={i === step ? "step" : undefined}
              aria-label={t.stepOf(i + 1, STEPS)}
              className={`h-1 w-6 rounded-full transition-colors ${i <= step ? "bg-control" : "bg-border-strong"}`}
            />
          ))}
        </ol>
      </div>

      <div key={step} className="card-in mx-auto my-auto w-full max-w-[560px] py-10">
        {step === 0 && (
          <div>
            <h1 className="text-[30px] leading-[1.15] font-semibold tracking-[-0.03em]">{t.welcome}</h1>
            <p className="mt-3 max-w-[46ch] text-[15px] text-muted">{t.welcomeBody}</p>
            <figure className="mt-8 space-y-3">
              <blockquote className="border-l-2 border-border-strong pl-4 text-[13px] leading-relaxed text-muted">{strings.home.sample}</blockquote>
              <div className="ml-4 rounded-xl bg-surface px-4 py-3.5 shadow-[var(--shadow-card)]">
                <Reading text={strings.reading.sample} streaming={false} />
              </div>
              <figcaption className="ml-4 text-[12px] text-muted">{t.example}</figcaption>
            </figure>
            <Group className="mt-8">
              <Switch label={t.searchFirst} hint={t.searchFirstHint} checked={!settings.offline_only} onChange={(on) => void update({ offline_only: !on })} />
            </Group>
            <Button variant="primary" className="mt-6" onClick={() => setStep(1)}>
              {t.start}
            </Button>
          </div>
        )}

        {step === 1 && recommended && (
          <div>
            <h1 className="text-[24px] font-semibold tracking-[-0.025em]">{t.modelTitle}</h1>
            <p className="mt-2 text-muted">{t.modelBody}</p>
            {info.total_ram_bytes && <p className="mt-1 text-[13px] text-muted">{strings.model.ram(info.total_ram_bytes)}</p>}
            <Group className="mt-6">
              <ModelRow model={recommended} recommended compact />
            </Group>
            <div className="mt-4 flex flex-wrap gap-x-5 gap-y-2 text-[13px]">
              <LinkButton
                onClick={() => {
                  finish();
                  onSeeModels();
                }}
              >
                {t.otherModels}
              </LinkButton>
              <LinkButton onClick={() => void chooseFile()}>{t.haveModel}</LinkButton>
            </div>
            <div className="mt-10 flex justify-between">
              <Button variant="ghost" onClick={() => setStep(0)}>
                {t.back}
              </Button>
              <Button variant="primary" disabled={!hasModel && !progress} onClick={() => setStep(2)}>
                {t.next}
              </Button>
            </div>
          </div>
        )}

        {step === 2 && (
          <div>
            <h1 className="text-[24px] font-semibold tracking-[-0.025em]">{t.tryTitle}</h1>
            <p className="mt-2 flex flex-wrap items-center gap-2 text-muted">
              {t.tryBody(settings.hotkey)} <Kbd keys={settings.hotkey} />
            </p>
            <Group className="mt-6 px-5 py-4">
              <p className="leading-relaxed select-text" style={{ fontFamily: "var(--reading-font)" }}>
                {strings.home.sample}
              </p>
            </Group>
            {progress && !hasModel && (
              <div className="mt-4 space-y-1.5">
                <Progress value={progress.downloaded / progress.total} label={strings.model.downloading} />
                <p className="text-[13px] text-muted">{t.downloadingNote}</p>
              </div>
            )}
            <div className="mt-10 flex justify-between">
              <Button variant="ghost" onClick={() => setStep(1)}>
                {t.back}
              </Button>
              <div className="flex gap-2">
                <Button disabled={!hasModel} onClick={() => void api.explainText(strings.home.sample)}>
                  {strings.home.explainSample}
                </Button>
                <Button variant="primary" onClick={finish}>
                  {t.done}
                </Button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
