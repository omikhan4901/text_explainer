import { open } from "@tauri-apps/plugin-dialog";
import { FileUp, Server } from "lucide-react";
import { useState } from "react";
import { Button } from "../../components/ui/Button";
import { Card, CardTitle } from "../../components/ui/Card";
import { Field, inputClass } from "../../components/ui/Field";
import { Segmented } from "../../components/ui/Segmented";
import { Tag } from "../../components/ui/Tag";
import { strings } from "../../i18n";
import { api, inApp } from "../../lib/ipc";
import type { Level, Settings } from "../../lib/types";
import { PageTitle } from "../App";
import { useData } from "../data";
import { ModelRow } from "../ModelRow";

const t = strings.model;

export function ModelPage() {
  const { info } = useData();
  const order = ["balanced", "fast", "fastest", "best"];
  const models = [...info.models].sort((a, b) => order.indexOf(a.tier) - order.indexOf(b.tier));
  return (
    <>
      <PageTitle lead={t.lead}>{t.title}</PageTitle>
      {!info.engine_found && (
        <p role="alert" className="mb-4 rounded-xl bg-danger-soft px-4 py-3 text-sm text-danger-text">
          {t.engineMissing}
        </p>
      )}
      <Card>
        <CardTitle aside={info.total_ram_bytes ? <span className="text-sm text-muted">{t.ram(info.total_ram_bytes)}</span> : undefined}>
          {t.catalog}
        </CardTitle>
        <div className="divide-y divide-border">
          {models.map((m) => (
            <ModelRow key={m.id} model={m} recommended={m.id === info.recommended} />
          ))}
        </div>
      </Card>
      <OwnModel />
      <Tuning />
      <Prompts />
    </>
  );
}

function OwnModel() {
  const { settings, update } = useData();
  const current = settings.model;
  const [url, setUrl] = useState(current.kind === "endpoint" ? current.base_url : "http://127.0.0.1:11434/v1");
  const [name, setName] = useState(current.kind === "endpoint" ? current.model : "");
  const [key, setKey] = useState(current.kind === "endpoint" ? (current.api_key ?? "") : "");

  const chooseFile = async () => {
    if (!inApp) return;
    const path = await open({ multiple: false, directory: false, filters: [{ name: "GGUF model", extensions: ["gguf"] }] });
    if (typeof path === "string") await update({ model: { kind: "file", path } });
  };

  return (
    <Card className="mt-4">
      <CardTitle>{t.own}</CardTitle>
      <p className="-mt-2 mb-4 text-sm text-muted">{t.ownBody}</p>
      <div className="flex flex-wrap items-center gap-3">
        <Button onClick={() => void chooseFile()}>
          <FileUp size={16} /> {t.chooseFile}
        </Button>
        {current.kind === "file" && (
          <span className="flex min-w-0 items-center gap-2 text-sm">
            <Tag tone="accent">{t.usingFile}</Tag>
            <span className="truncate text-muted" title={current.path}>
              {current.path}
            </span>
          </span>
        )}
      </div>
      <form
        className="mt-6 grid gap-3 sm:grid-cols-[1fr_1fr]"
        onSubmit={(e) => {
          e.preventDefault();
          void update({ model: { kind: "endpoint", base_url: url.trim(), model: name.trim(), api_key: key.trim() || null } });
        }}
      >
        <p className="flex items-center gap-2 font-medium sm:col-span-2">
          <Server size={16} aria-hidden /> {t.endpoint}
          {current.kind === "endpoint" && <Tag tone="accent">{t.usingEndpoint}</Tag>}
        </p>
        <label className="grid gap-1 text-sm">
          {t.endpointUrl}
          <input className={inputClass} value={url} onChange={(e) => setUrl(e.target.value)} required pattern="https?://.+" />
        </label>
        <label className="grid gap-1 text-sm">
          {t.endpointModel}
          <input className={inputClass} value={name} onChange={(e) => setName(e.target.value)} required placeholder="qwen3.5:4b" />
        </label>
        <label className="grid gap-1 text-sm">
          {t.endpointKey}
          <input className={inputClass} value={key} onChange={(e) => setKey(e.target.value)} type="password" autoComplete="off" />
        </label>
        <div className="flex items-end">
          <Button type="submit">{t.useEndpoint}</Button>
        </div>
      </form>
    </Card>
  );
}

function Tuning() {
  const { settings, update } = useData();
  const [args, setArgs] = useState(settings.extra_server_args.join(" "));
  const numberSelect = <K extends keyof Settings>(key: K, options: [number | null, string][], value: number | null) => (
    <select
      className={inputClass}
      value={value === null ? "" : String(value)}
      onChange={(e) => void update({ [key]: e.target.value === "" ? null : Number(e.target.value) } as Partial<Settings>)}
    >
      {options.map(([v, label]) => (
        <option key={label} value={v === null ? "" : String(v)}>
          {label}
        </option>
      ))}
    </select>
  );
  return (
    <Card className="mt-4">
      <CardTitle aside={<Button size="sm" variant="ghost" onClick={() => void api.unloadModel()}>{t.unload}</Button>}>{t.advanced}</CardTitle>
      <p className="-mt-2 mb-2 text-sm text-muted">{t.advancedLead}</p>
      <div className="divide-y divide-border">
        <Field label={t.temperature} hint={t.temperatureHint}>
          <Slider value={settings.temperature} min={0} max={1.5} step={0.05} onChange={(v) => void update({ temperature: v })} />
        </Field>
        <Field label={t.topP}>
          <Slider value={settings.top_p} min={0.1} max={1} step={0.05} onChange={(v) => void update({ top_p: v })} />
        </Field>
        <Field label={t.context} hint={t.contextHint}>
          {numberSelect("context_size", [[2048, "2,048"], [4096, "4,096"], [8192, "8,192"], [16384, "16,384"]], settings.context_size)}
        </Field>
        <Field label={t.threads}>
          {numberSelect(
            "threads",
            [[null, t.threadsAuto], ...[2, 4, 6, 8, 12, 16].map((n) => [n, String(n)] as [number, string])],
            settings.threads,
          )}
        </Field>
        <Field label={t.idle}>
          {numberSelect(
            "idle_unload_minutes",
            [[0, t.idleNever], ...[5, 10, 30, 60].map((m) => [m, t.idleMinutes(m)] as [number, string])],
            settings.idle_unload_minutes,
          )}
        </Field>
        <Field label={t.serverArgs} hint={t.serverArgsHint}>
          <input
            className={`${inputClass} w-64 font-mono`}
            value={args}
            onChange={(e) => setArgs(e.target.value)}
            onBlur={() => void update({ extra_server_args: args.split(/\s+/).filter(Boolean) })}
            placeholder="--mlock"
          />
        </Field>
      </div>
    </Card>
  );
}

function Slider({ value, min, max, step, onChange }: { value: number; min: number; max: number; step: number; onChange: (v: number) => void }) {
  const [local, setLocal] = useState(value);
  return (
    <span className="flex items-center gap-3">
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={local}
        onChange={(e) => setLocal(Number(e.target.value))}
        onPointerUp={() => onChange(local)}
        onKeyUp={() => onChange(local)}
        className="w-40 accent-[var(--accent)]"
      />
      <span className="w-10 text-right text-sm tabular-nums text-muted">{local.toFixed(2)}</span>
    </span>
  );
}

function Prompts() {
  const { settings, info, update } = useData();
  const [level, setLevel] = useState<Level>(settings.level);
  const custom = settings.prompts[level];
  const [draft, setDraft] = useState<string>(custom ?? info.default_prompts[level]);
  const [shownFor, setShownFor] = useState<Level>(level);
  if (shownFor !== level) {
    setShownFor(level);
    setDraft(settings.prompts[level] ?? info.default_prompts[level]);
  }
  const isCustom = custom !== null && custom.trim() !== "";
  const save = (value: string | null) => void update({ prompts: { ...settings.prompts, [level]: value } });
  return (
    <Card className="mt-4">
      <CardTitle aside={<Tag tone={isCustom ? "warn" : "neutral"}>{isCustom ? t.promptCustom : t.promptDefault}</Tag>}>{t.prompts}</CardTitle>
      <p className="-mt-2 mb-4 text-sm text-muted">{t.promptsLead}</p>
      <Segmented<Level>
        label={strings.level.label}
        value={level}
        onChange={setLevel}
        options={[
          { value: "simpler", label: strings.level.simpler },
          { value: "plain", label: strings.level.plain },
          { value: "clearer", label: strings.level.clearer },
        ]}
      />
      <textarea
        aria-label={t.prompts}
        className={`${inputClass} mt-3 h-64 w-full py-2 font-mono text-xs leading-relaxed`}
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
      />
      <div className="mt-3 flex gap-2">
        <Button variant="primary" size="sm" onClick={() => save(draft === info.default_prompts[level] ? null : draft)}>
          {strings.common.save}
        </Button>
        <Button
          size="sm"
          variant="ghost"
          disabled={!isCustom}
          onClick={() => {
            setDraft(info.default_prompts[level]);
            save(null);
          }}
        >
          {strings.common.reset}
        </Button>
      </div>
    </Card>
  );
}
