import { open } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { Button } from "../components/ui/Button";
import { Card } from "../components/ui/Card";
import { Kbd } from "../components/ui/Kbd";
import { Progress } from "../components/ui/Progress";
import { strings } from "../i18n";
import { api, inApp } from "../lib/ipc";
import { useData, useDownloadOf } from "./data";
import { ModelRow } from "./ModelRow";

const t = strings.firstRun;

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
    <div className="grid min-h-dvh place-items-center bg-bg px-6 py-10">
      <div className="w-full max-w-xl">
        <ol className="mb-8 flex justify-center gap-2" aria-label="Steps">
          {[0, 1, 2].map((i) => (
            <li key={i} aria-current={i === step ? "step" : undefined} className={`h-1.5 w-10 rounded-full ${i <= step ? "bg-accent" : "bg-border"}`} />
          ))}
        </ol>

        {step === 0 && (
          <div className="text-center">
            <h1 className="text-4xl font-semibold">{t.welcome}</h1>
            <p className="mx-auto mt-4 max-w-md text-lg text-muted">{t.welcomeBody}</p>
            <Button variant="primary" className="mt-8" onClick={() => setStep(1)}>
              {t.start}
            </Button>
          </div>
        )}

        {step === 1 && recommended && (
          <div>
            <h1 className="text-3xl font-semibold">{t.modelTitle}</h1>
            <p className="mt-2 text-muted">{t.modelBody}</p>
            {info.total_ram_bytes && <p className="mt-1 text-sm text-muted">{strings.model.ram(info.total_ram_bytes)}</p>}
            <Card className="mt-6">
              <ModelRow model={recommended} recommended compact />
            </Card>
            <div className="mt-4 flex flex-wrap gap-x-4 gap-y-2 text-sm">
              <button
                type="button"
                className="font-medium text-accent underline-offset-2 hover:underline"
                onClick={() => {
                  finish();
                  onSeeModels();
                }}
              >
                {t.otherModels}
              </button>
              <button type="button" className="font-medium text-accent underline-offset-2 hover:underline" onClick={() => void chooseFile()}>
                {t.haveModel}
              </button>
            </div>
            <div className="mt-8 flex justify-between">
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
            <h1 className="text-3xl font-semibold">{t.tryTitle}</h1>
            <p className="mt-2 flex flex-wrap items-center gap-2 text-muted">
              {t.tryBody(settings.hotkey)} <Kbd keys={settings.hotkey} />
            </p>
            <blockquote className="mt-6 rounded-xl border border-border bg-surface p-5 leading-relaxed select-text">{strings.home.sample}</blockquote>
            {progress && !hasModel && (
              <div className="mt-4 space-y-1.5">
                <Progress value={progress.downloaded / progress.total} label={strings.model.downloading} />
                <p className="text-sm text-muted">{t.downloadingNote}</p>
              </div>
            )}
            <div className="mt-8 flex justify-between">
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
