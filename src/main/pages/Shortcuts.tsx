import { Plus, Trash } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import { Button } from "../../components/ui/Button";
import { Field, inputClass } from "../../components/ui/Field";
import { IconButton } from "../../components/ui/IconButton";
import { Kbd } from "../../components/ui/Kbd";
import { Group, Section } from "../../components/ui/Section";
import { Switch } from "../../components/ui/Switch";
import { strings } from "../../i18n";
import { api } from "../../lib/ipc";
import { PageTitle } from "../App";
import { useData } from "../data";

const t = strings.shortcuts;
const a = strings.apps;
/** Most apps the list keeps (`MAX_APPS` in rules.rs). */
const MAX_APPS = 50;

type ShortcutField = "hotkey" | "lookup_hotkey";

/** Turns a key event into the app's shortcut format ("Ctrl+Shift+Space"), or null while
 * only modifiers are held. */
export function shortcutFromEvent(e: Pick<KeyboardEvent, "ctrlKey" | "altKey" | "shiftKey" | "metaKey" | "code">): string | null {
  const code = e.code;
  if (/^(Control|Shift|Alt|Meta|OS)(Left|Right)?$/.test(code)) return null;
  let key: string | null = null;
  if (code.startsWith("Key")) key = code.slice(3);
  else if (code.startsWith("Digit")) key = code.slice(5);
  else if (/^F\d{1,2}$/.test(code)) key = code;
  else if (code === "Space") key = "Space";
  else if (["Period", "Comma", "Slash", "Semicolon", "Quote", "BracketLeft", "BracketRight", "Backslash", "Minus", "Equal", "Backquote"].includes(code))
    key = code;
  if (!key) return null;
  const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(Boolean);
  return [...mods, key].join("+");
}

export function ShortcutsPage() {
  const { settings, update } = useData();
  const [recording, setRecording] = useState<ShortcutField | null>(null);
  const [problem, setProblem] = useState<{ field: ShortcutField; message: string } | null>(null);
  const { hotkey: main, lookup_hotkey: lookup } = settings;

  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      if (e.code === "Escape") {
        setRecording(null);
        return;
      }
      const hotkey = shortcutFromEvent(e);
      if (!hotkey) return;
      const field = recording;
      setRecording(null);
      const other = field === "hotkey" ? lookup : main;
      if (other.toLowerCase() === hotkey.toLowerCase()) {
        setProblem({ field, message: t.sameAsOther });
        return;
      }
      api
        .validateHotkey(hotkey)
        .then(() => update(field === "hotkey" ? { hotkey } : { lookup_hotkey: hotkey }))
        .then(() => setProblem(null))
        .catch((err) => setProblem({ field, message: String(err) }));
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording, update, main, lookup]);

  const hintFor = (field: ShortcutField, fallback: string) => (problem?.field === field ? problem.message : fallback);
  const pressing = (
    <span className="text-[13px] font-medium text-accent" role="status">
      {t.press}
    </span>
  );

  return (
    <>
      <PageTitle lead={t.lead}>{t.title}</PageTitle>
      <Section title={t.open}>
        <Group>
          <Field label={t.hotkey} hint={hintFor("hotkey", t.hotkeyHint)}>
            <div className="flex items-center gap-3">
              {recording === "hotkey" ? pressing : <Kbd keys={main} />}
              <Button size="sm" onClick={() => setRecording((r) => (r === "hotkey" ? null : "hotkey"))}>
                {recording === "hotkey" ? strings.common.cancel : t.change}
              </Button>
            </div>
          </Field>
          <Field label={t.lookupHotkey} hint={hintFor("lookup_hotkey", t.lookupHotkeyHint)}>
            <div className="flex items-center gap-3">
              {recording === "lookup_hotkey" ? pressing : lookup ? <Kbd keys={lookup} /> : <span className="text-[13px] text-muted">{strings.common.off}</span>}
              {lookup && recording !== "lookup_hotkey" && (
                <Button size="sm" variant="ghost" onClick={() => void update({ lookup_hotkey: "" })}>
                  {t.turnOff}
                </Button>
              )}
              {recording === "lookup_hotkey" ? (
                <Button size="sm" onClick={() => setRecording(null)}>
                  {strings.common.cancel}
                </Button>
              ) : (
                <Button size="sm" aria-label={lookup ? t.changeLookup : t.setLookup} onClick={() => setRecording("lookup_hotkey")}>
                  {lookup ? t.change : t.set}
                </Button>
              )}
            </div>
          </Field>
          <Switch label={t.doubleCopy} hint={t.doubleCopyHint} checked={settings.double_copy} onChange={(v) => void update({ double_copy: v })} />
        </Group>
      </Section>
      <Section title={t.behaviour}>
        <Group>
          <Switch label={t.clipboard} hint={t.clipboardHint} checked={settings.clipboard_fallback} onChange={(v) => void update({ clipboard_fallback: v })} />
          <Switch label={t.startup} hint={t.startupHint} checked={settings.start_with_windows} onChange={(v) => void update({ start_with_windows: v })} />
        </Group>
      </Section>
      <ExcludedApps />
    </>
  );
}

/** "Don't open in these apps": the card never reads a selection in these. */
function ExcludedApps() {
  const { settings, update } = useData();
  const list = settings.excluded_apps;
  const [name, setName] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  const [lastApp, setLastApp] = useState<string | null>(null);

  useEffect(() => {
    api
      .lastApp()
      .then(setLastApp)
      .catch(() => setLastApp(null));
  }, []);

  const add = async (raw: string) => {
    let app: string;
    try {
      app = await api.validateApp(raw);
    } catch (e) {
      return setProblem(String(e));
    }
    if (list.includes(app)) return setProblem(a.already(app));
    if (list.length >= MAX_APPS) return setProblem(a.full(MAX_APPS));
    if (await update({ excluded_apps: [...list, app] })) {
      setProblem(null);
      if (raw === name) setName("");
    }
  };
  const suggestion = lastApp && !list.includes(lastApp) ? lastApp : null;

  return (
    <Section title={a.title} description={a.lead}>
      <Group>
        {list.length === 0 ? (
          <p className="px-4 py-3 text-[13px] text-muted">{a.empty}</p>
        ) : (
          <ul aria-label={a.list} className="divide-y divide-border">
            {list.map((app) => (
              <li key={app} className="flex items-center justify-between gap-3 py-1.5 pr-2.5 pl-4">
                <span className="min-w-0 truncate font-mono text-[13px]">{app}</span>
                <IconButton label={a.remove(app)} onClick={() => void update({ excluded_apps: list.filter((x) => x !== app) })}>
                  <Trash size={14} />
                </IconButton>
              </li>
            ))}
          </ul>
        )}
        <form
          className="grid gap-3 px-4 py-3.5 sm:grid-cols-[1fr_auto]"
          onSubmit={(e) => {
            e.preventDefault();
            void add(name);
          }}
        >
          <label className="grid gap-1 text-[13px] text-muted">
            {a.name}
            <input
              className={`${inputClass} w-full min-w-0 font-mono`}
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="notepad.exe"
              spellCheck={false}
              aria-invalid={problem !== null}
              aria-describedby="add-app-problem"
            />
          </label>
          <div className="flex items-end gap-2">
            <Button type="submit">
              <Plus size={14} /> {a.add}
            </Button>
            {suggestion && (
              <Button variant="ghost" onClick={() => void add(suggestion)}>
                {a.addLast(suggestion)}
              </Button>
            )}
          </div>
          <p
            id="add-app-problem"
            role={problem ? "alert" : undefined}
            className={`text-[12px] sm:col-span-2 ${problem ? "text-danger first-letter:uppercase" : "text-muted"}`}
          >
            {problem ?? a.hint}
          </p>
        </form>
      </Group>
    </Section>
  );
}
