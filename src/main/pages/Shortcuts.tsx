import { useEffect, useState } from "react";
import { Button } from "../../components/ui/Button";
import { Card } from "../../components/ui/Card";
import { Field } from "../../components/ui/Field";
import { Kbd } from "../../components/ui/Kbd";
import { Switch } from "../../components/ui/Switch";
import { strings } from "../../i18n";
import { api } from "../../lib/ipc";
import { PageTitle } from "../App";
import { useData } from "../data";

const t = strings.shortcuts;

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
  else if (["Period", "Comma", "Slash", "Semicolon", "Quote", "BracketLeft", "BracketRight", "Backslash", "Minus", "Equal", "Backquote"].includes(code)) key = code;
  if (!key) return null;
  const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(Boolean);
  return [...mods, key].join("+");
}

export function ShortcutsPage() {
  const { settings, update } = useData();
  const [recording, setRecording] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      if (e.code === "Escape") {
        setRecording(false);
        return;
      }
      const hotkey = shortcutFromEvent(e);
      if (!hotkey) return;
      setRecording(false);
      api
        .validateHotkey(hotkey)
        .then(() => update({ hotkey }))
        .then(() => setProblem(null))
        .catch((err) => setProblem(String(err)));
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording, update]);

  return (
    <>
      <PageTitle>{t.title}</PageTitle>
      <Card>
        <div className="divide-y divide-border">
          <Field label={t.hotkey} hint={problem ?? t.hotkeyHint}>
            <div className="flex items-center gap-3">
              {recording ? <span className="text-sm text-accent">{t.press}</span> : <Kbd keys={settings.hotkey} />}
              <Button size="sm" onClick={() => setRecording((r) => !r)}>
                {recording ? strings.common.cancel : t.change}
              </Button>
            </div>
          </Field>
          <Switch label={t.doubleCopy} hint={t.doubleCopyHint} checked={settings.double_copy} onChange={(v) => void update({ double_copy: v })} />
          <Switch
            label={t.clipboard}
            hint={t.clipboardHint}
            checked={settings.clipboard_fallback}
            onChange={(v) => void update({ clipboard_fallback: v })}
          />
          <Switch label={t.startup} hint={t.startupHint} checked={settings.start_with_windows} onChange={(v) => void update({ start_with_windows: v })} />
        </div>
      </Card>
    </>
  );
}
