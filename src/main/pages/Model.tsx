import { open } from "@tauri-apps/plugin-dialog";
import { FileArrowUp } from "@phosphor-icons/react";
import { useState } from "react";
import { Button } from "../../components/ui/Button";
import { Field, inputClass } from "../../components/ui/Field";
import { Group, Section } from "../../components/ui/Section";
import { Segmented } from "../../components/ui/Segmented";
import { Tag } from "../../components/ui/Tag";
import { strings } from "../../i18n";
import { api, inApp } from "../../lib/ipc";
import type { Level, Settings } from "../../lib/types";
import { PageTitle } from "../App";
import { useData } from "../data";
import { FoundModels, useFoundModels } from "../FoundModels";
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
        <p role="alert" className="mb-6 rounded-lg bg-danger-soft px-3.5 py-2.5 text-[13px] text-danger">
          {t.engineMissing}
        </p>
      )}
      <Section title={t.catalog} description={info.total_ram_bytes ? t.ram(info.total_ram_bytes) : undefined}>
        <Group>
          {models.map((m) => (
            <ModelRow key={m.id} model={m} recommended={m.id === info.recommended} />
          ))}
        </Group>
      </Section>
      <Found />
      <OwnModel />
      <Tuning />
      <Prompts />
    </>
  );
}

function Found() {
  const { found, search } = useFoundModels();
  return (
    <Section
      title={t.found}
      description={t.foundLead}
      aside={
        <Button size="sm" variant="ghost" onClick={search} disabled={found === null}>
          {t.lookAgain}
        </Button>
      }
    >
      <Group>
        <FoundModels found={found} />
      </Group>
    </Section>
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
    <Section title={t.own} description={t.ownBody}>
      <Group>
        <div className="flex flex-wrap items-center justify-between gap-3 px-4 py-3">
          <div className="min-w-0">
            <p className="flex items-center gap-2 font-medium">
              {t.file}
              {current.kind === "file" && <Tag tone="accent">{t.usingFile}</Tag>}
            </p>
            {current.kind === "file" && (
              <p className="mt-0.5 truncate font-mono text-[11px] text-muted" title={current.path}>
                {current.path}
              </p>
            )}
          </div>
          <Button size="sm" onClick={() => void chooseFile()}>
            <FileArrowUp size={14} /> {t.chooseFile}
          </Button>
        </div>
        <form
          className="grid gap-3 px-4 py-3.5 sm:grid-cols-[1fr_1fr]"
          onSubmit={(e) => {
            e.preventDefault();
            void update({ model: { kind: "endpoint", base_url: url.trim(), model: name.trim(), api_key: key.trim() || null } });
          }}
        >
          <p className="flex items-center gap-2 font-medium sm:col-span-2">
            {t.endpoint}
            {current.kind === "endpoint" && <Tag tone="accent">{t.usingEndpoint}</Tag>}
          </p>
          <label className="grid gap-1 text-[13px] text-muted">
            {t.endpointUrl}
            <input className={inputClass} value={url} onChange={(e) => setUrl(e.target.value)} required pattern="https?://.+" />
          </label>
          <label className="grid gap-1 text-[13px] text-muted">
            {t.endpointModel}
            <input className={inputClass} value={name} onChange={(e) => setName(e.target.value)} required placeholder="qwen3.5:4b" />
          </label>
          <label className="grid gap-1 text-[13px] text-muted">
            {t.endpointKey}
            <input className={inputClass} value={key} onChange={(e) => setKey(e.target.value)} type="password" autoComplete="off" />
          </label>
          <div className="flex items-end">
            <Button type="submit">{t.useEndpoint}</Button>
          </div>
        </form>
      </Group>
    </Section>
  );
}

function Tuning() {
  const { settings, update } = useData();
  const [args, setArgs] = useState(settings.extra_server_args.join(" "));
  const numberSelect = <K extends keyof Settings>(key: K, label: string, options: [number | null, string][], value: number | null) => (
    <select
      aria-label={label}
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
    <Section
      title={t.advanced}
      description={t.advancedLead}
      aside={
        <Button size="sm" variant="ghost" onClick={() => void api.unloadModel()}>
          {t.unload}
        </Button>
      }
    >
      <Group>
        <Field label={t.temperature} hint={t.temperatureHint}>
          <Slider label={t.temperature} value={settings.temperature} min={0} max={1.5} step={0.05} onChange={(v) => void update({ temperature: v })} />
        </Field>
        <Field label={t.topP}>
          <Slider label={t.topP} value={settings.top_p} min={0.1} max={1} step={0.05} onChange={(v) => void update({ top_p: v })} />
        </Field>
        <Field label={t.context} hint={t.contextHint}>
          {numberSelect(
            "context_size",
            t.context,
            [
              [2048, "2,048"],
              [4096, "4,096"],
              [8192, "8,192"],
              [16384, "16,384"],
            ],
            settings.context_size,
          )}
        </Field>
        <Field label={t.threads}>
          {numberSelect(
            "threads",
            t.threads,
            [[null, t.threadsAuto], ...[2, 4, 6, 8, 12, 16].map((n) => [n, String(n)] as [number, string])],
            settings.threads,
          )}
        </Field>
        <Field label={t.idle}>
          {numberSelect(
            "idle_unload_minutes",
            t.idle,
            [[0, t.idleNever], ...[5, 10, 30, 60].map((m) => [m, t.idleMinutes(m)] as [number, string])],
            settings.idle_unload_minutes,
          )}
        </Field>
        <Field label={t.serverArgs} hint={t.serverArgsHint}>
          <input
            aria-label={t.serverArgs}
            className={`${inputClass} w-64 font-mono`}
            value={args}
            onChange={(e) => setArgs(e.target.value)}
            onBlur={() => void update({ extra_server_args: args.split(/\s+/).filter(Boolean) })}
            placeholder="--mlock"
          />
        </Field>
      </Group>
    </Section>
  );
}

function Slider({
  label,
  value,
  min,
  max,
  step,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (v: number) => void;
}) {
  const [local, setLocal] = useState(value);
  return (
    <span className="flex items-center gap-3">
      <input
        type="range"
        aria-label={label}
        min={min}
        max={max}
        step={step}
        value={local}
        onChange={(e) => setLocal(Number(e.target.value))}
        onPointerUp={() => onChange(local)}
        onKeyUp={() => onChange(local)}
        className="w-36 accent-[var(--accent)]"
      />
      <span className="w-10 text-right font-mono text-[12px] text-muted tabular-nums">{local.toFixed(2)}</span>
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
    <Section
      title={t.prompts}
      description={t.promptsLead}
      aside={<Tag tone={isCustom ? "warn" : "neutral"}>{isCustom ? t.promptCustom : t.promptDefault}</Tag>}
    >
      <Group className="p-3">
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
          className={`${inputClass} mt-3 h-64 w-full py-2 font-mono text-[12px] leading-relaxed`}
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
      </Group>
    </Section>
  );
}
