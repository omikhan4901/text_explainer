import { ArrowDown, ArrowUp, Plus, Trash } from "@phosphor-icons/react";
import { Switch as RadixSwitch } from "radix-ui";
import { useState } from "react";
import { Button } from "../../components/ui/Button";
import { Field, inputClass } from "../../components/ui/Field";
import { Group, Section } from "../../components/ui/Section";
import { Switch } from "../../components/ui/Switch";
import { strings } from "../../i18n";
import { api } from "../../lib/ipc";
import type { WebSource } from "../../lib/types";
import { hostOf } from "../../popup/Popup";
import { PageTitle } from "../App";
import { useData } from "../data";

const t = strings.sources;

/** Languages Google Translate takes, for the Translate tab's target. */
const TRANSLATE_TO: [string, string][] = [
  ["en", "English"],
  ["bn", "বাংলা (Bangla)"],
  ["hi", "हिन्दी (Hindi)"],
  ["ur", "اردو (Urdu)"],
  ["ar", "العربية (Arabic)"],
  ["es", "Español (Spanish)"],
  ["fr", "Français (French)"],
  ["de", "Deutsch (German)"],
  ["pt", "Português (Portuguese)"],
  ["ru", "Русский (Russian)"],
  ["tr", "Türkçe (Turkish)"],
  ["id", "Bahasa Indonesia"],
  ["zh-CN", "中文 (Chinese)"],
  ["ja", "日本語 (Japanese)"],
  ["ko", "한국어 (Korean)"],
];

export function SourcesPage() {
  const { settings, update } = useData();
  const list = settings.sources;
  const setList = (sources: WebSource[]) => void update({ sources });
  const patch = (id: string, p: Partial<WebSource>) => setList(list.map((s) => (s.id === id ? { ...s, ...p } : s)));
  const move = (i: number, by: -1 | 1) => {
    const next = list.slice();
    const [item] = next.splice(i, 1);
    next.splice(i + by, 0, item!);
    setList(next);
  };
  const off = settings.offline_only;

  return (
    <>
      <PageTitle lead={t.lead}>{t.title}</PageTitle>

      <Section title={t.web}>
        <Group>
          <Switch label={t.offline} hint={t.offlineHint} checked={off} onChange={(offline_only) => void update({ offline_only })} />
        </Group>
      </Section>

      <Section title={t.tabs} description={t.tabsLead}>
        {off && <p className="mb-3 rounded-lg bg-surface-2 px-3.5 py-2.5 text-[13px] text-muted">{t.hiddenWhileOffline}</p>}
        <Group>
          {list.map((s, i) => (
            <div key={s.id} className="flex items-center gap-3 px-4 py-2.5">
              <RadixSwitch.Root
                checked={s.enabled}
                onCheckedChange={(enabled) => patch(s.id, { enabled })}
                aria-label={t.show(s.name)}
                className="relative h-5 w-9 shrink-0 rounded-full bg-border-strong transition-colors duration-150 data-[state=checked]:bg-accent"
              >
                <RadixSwitch.Thumb className="block size-4 translate-x-0.5 rounded-full bg-surface shadow-[0_1px_2px_rgb(0_0_0/0.25)] transition-transform duration-150 data-[state=checked]:translate-x-[18px]" />
              </RadixSwitch.Root>
              <div className="min-w-0 flex-1">
                <p className={`truncate font-medium ${s.enabled ? "" : "text-muted"}`}>{s.name}</p>
                <p className="truncate text-[12px] text-muted">{hostOf(s.url.replace("{query}", "x").replace("{lang}", "en"))}</p>
              </div>
              <div className="flex shrink-0 gap-1" role="group" aria-label={t.showFor(s.name)}>
                <Chip pressed={s.words} onClick={() => patch(s.id, { words: !s.words })}>
                  {t.words}
                </Chip>
                <Chip pressed={s.passages} onClick={() => patch(s.id, { passages: !s.passages })}>
                  {t.passages}
                </Chip>
              </div>
              <div className="flex shrink-0">
                <IconAction label={t.moveUp(s.name)} disabled={i === 0} onClick={() => move(i, -1)}>
                  <ArrowUp size={14} />
                </IconAction>
                <IconAction label={t.moveDown(s.name)} disabled={i === list.length - 1} onClick={() => move(i, 1)}>
                  <ArrowDown size={14} />
                </IconAction>
                <IconAction label={t.remove(s.name)} disabled={!s.id.startsWith("custom-")} onClick={() => setList(list.filter((x) => x.id !== s.id))}>
                  <Trash size={14} />
                </IconAction>
              </div>
            </div>
          ))}
        </Group>
        <p className="mt-2 text-[12px] text-muted">{t.aiLast}</p>
      </Section>

      <AddSource onAdd={(s) => setList([...list, s])} existing={list} />

      <Section title={t.translate}>
        <Group>
          <Field label={t.translateTo} hint={t.translateHint} htmlFor="translate-to">
            <select id="translate-to" className={inputClass} value={settings.translate_to} onChange={(e) => void update({ translate_to: e.target.value })}>
              {TRANSLATE_TO.map(([code, name]) => (
                <option key={code} value={code}>
                  {name}
                </option>
              ))}
            </select>
          </Field>
        </Group>
      </Section>
    </>
  );
}

function Chip({ pressed, onClick, children }: { pressed: boolean; onClick: () => void; children: string }) {
  return (
    <button
      type="button"
      aria-pressed={pressed}
      onClick={onClick}
      className="h-6 rounded-[5px] border border-border-strong px-2 text-[12px] font-medium text-muted transition-colors hover:text-text aria-pressed:border-transparent aria-pressed:bg-surface-3 aria-pressed:text-text"
    >
      {children}
    </button>
  );
}

function IconAction({ label, disabled, onClick, children }: { label: string; disabled?: boolean; onClick: () => void; children: React.ReactNode }) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      disabled={disabled}
      onClick={onClick}
      className="grid size-7 place-items-center rounded-md text-muted transition-colors hover:bg-surface-2 hover:text-text disabled:invisible"
    >
      {children}
    </button>
  );
}

/** A site the person adds: a name and an address with {query} where the selection goes. */
function AddSource({ onAdd, existing }: { onAdd: (s: WebSource) => void; existing: WebSource[] }) {
  const [name, setName] = useState("");
  const [url, setUrl] = useState("");
  const [problem, setProblem] = useState<string | null>(null);

  const add = async () => {
    const clean = name.trim();
    if (!clean) return setProblem(t.needName);
    try {
      await api.validateSource(url.trim());
    } catch (e) {
      return setProblem(String(e));
    }
    let id = `custom-${
      clean
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, "-")
        .replace(/^-|-$/g, "") || "site"
    }`;
    for (let n = 2; existing.some((s) => s.id === id); n++) id = `${id.replace(/-\d+$/, "")}-${n}`;
    onAdd({ id, name: clean, url: url.trim(), enabled: true, words: true, passages: true });
    setName("");
    setUrl("");
    setProblem(null);
  };

  return (
    <Section title={t.add} description={t.addLead}>
      <Group>
        <form
          className="grid gap-3 px-4 py-3.5 sm:grid-cols-[10rem_1fr_auto]"
          onSubmit={(e) => {
            e.preventDefault();
            void add();
          }}
        >
          <label className="grid gap-1 text-[13px] text-muted">
            {t.name}
            <input className={`${inputClass} w-full min-w-0`} value={name} onChange={(e) => setName(e.target.value)} placeholder="Merriam-Webster" />
          </label>
          <label className="grid gap-1 text-[13px] text-muted">
            {t.address}
            <input
              className={`${inputClass} w-full min-w-0 font-mono`}
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder="https://www.merriam-webster.com/dictionary/{query}"
              aria-invalid={problem !== null}
              aria-describedby="add-source-problem"
            />
          </label>
          <div className="flex items-end">
            <Button type="submit">
              <Plus size={14} /> {t.addButton}
            </Button>
          </div>
          <p
            id="add-source-problem"
            role={problem ? "alert" : undefined}
            className={`text-[12px] sm:col-span-3 ${problem ? "text-danger first-letter:uppercase" : "text-muted"}`}
          >
            {problem ?? t.addHint}
          </p>
        </form>
      </Group>
    </Section>
  );
}
