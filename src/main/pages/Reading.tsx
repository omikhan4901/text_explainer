import { Card } from "../../components/ui/Card";
import { Field, inputClass } from "../../components/ui/Field";
import { Segmented } from "../../components/ui/Segmented";
import { strings } from "../../i18n";
import type { AccentName, Appearance, Level, ReadingFont, Theme } from "../../lib/types";
import { Reading } from "../../popup/Popup";
import { PageTitle } from "../App";
import { useData } from "../data";

const t = strings.reading;
/** Each accent's light and dark value (tokens.css), so every swatch shows on both themes. */
const SWATCH: Record<AccentName, [string, string]> = {
  plum: ["#6d28d9", "#a78bfa"],
  saffron: ["#b45309", "#f59e0b"],
  garnet: ["#9f1239", "#fb7185"],
  ink: ["#18181b", "#e4e4e7"],
};

export function ReadingPage() {
  const { settings, info, update } = useData();
  const a = settings.appearance;
  const setA = (patch: Partial<Appearance>) => void update({ appearance: { ...a, ...patch } });
  const dark = document.documentElement.classList.contains("dark");
  const output = settings.output.mode === "fixed" ? settings.output.code : "";
  return (
    <>
      <PageTitle>{t.title}</PageTitle>
      <Card>
        <div className="divide-y divide-border">
          <Field label={t.level} hint={t.levelHint}>
            <Segmented<Level>
              label={t.level}
              value={settings.level}
              onChange={(level) => void update({ level })}
              options={[
                { value: "simpler", label: strings.level.simpler, title: strings.level.simplerHint },
                { value: "plain", label: strings.level.plain, title: strings.level.plainHint },
                { value: "clearer", label: strings.level.clearer, title: strings.level.clearerHint },
              ]}
            />
          </Field>
          <Field label={t.language} htmlFor="language">
            <select
              id="language"
              className={inputClass}
              value={output}
              onChange={(e) => void update({ output: e.target.value ? { mode: "fixed", code: e.target.value } : { mode: "same_as_text" } })}
            >
              <option value="">{t.sameLanguage}</option>
              {info.languages.map((l) => (
                <option key={l.code} value={l.code}>
                  {l.native === l.name ? l.name : `${l.native} (${l.name})`}
                </option>
              ))}
            </select>
          </Field>
          <Field label={t.font} hint={t.fontHint}>
            <Segmented<ReadingFont>
              label={t.font}
              value={a.reading_font}
              onChange={(reading_font) => setA({ reading_font })}
              options={(["plex", "atkinson", "serif"] as const).map((f) => ({ value: f, label: t.fonts[f] }))}
            />
          </Field>
          <Field label={t.size}>
            <Range value={a.font_size} min={12} max={24} step={1} format={(v) => `${v}px`} onChange={(font_size) => setA({ font_size })} />
          </Field>
          <Field label={t.spacing}>
            <Range value={a.line_height} min={1.2} max={2.2} step={0.1} format={(v) => v.toFixed(1)} onChange={(line_height) => setA({ line_height })} />
          </Field>
          <Field label={t.theme}>
            <Segmented<Theme>
              label={t.theme}
              value={a.theme}
              onChange={(theme) => setA({ theme })}
              options={(["system", "light", "dark"] as const).map((v) => ({ value: v, label: t.themes[v] }))}
            />
          </Field>
          <Field label={t.accent}>
            <div role="radiogroup" aria-label={t.accent} className="flex gap-2">
              {(Object.keys(SWATCH) as AccentName[]).map((name) => (
                <button
                  key={name}
                  type="button"
                  role="radio"
                  aria-checked={a.accent === name}
                  aria-label={t.accents[name]}
                  title={t.accents[name]}
                  onClick={() => setA({ accent: name })}
                  className={`size-8 rounded-full border-2 transition-transform ${a.accent === name ? "scale-110 border-text" : "border-border-strong"}`}
                  style={{ background: SWATCH[name][dark ? 1 : 0] }}
                />
              ))}
            </div>
          </Field>
        </div>
      </Card>
      <Card className="mt-4">
        <p className="mb-3 text-sm font-medium text-muted">{t.preview}</p>
        <Reading
          text="Whatever else this agreement says, the tenant (the Lessee) must pay the monthly rent of $1,200 by the 5th day of each month. If they don't, a late fee of 5% of the unpaid amount is added."
          streaming={false}
        />
      </Card>
    </>
  );
}

function Range({ value, min, max, step, format, onChange }: { value: number; min: number; max: number; step: number; format: (v: number) => string; onChange: (v: number) => void }) {
  return (
    <span className="flex items-center gap-3">
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="w-40 accent-[var(--accent)]"
      />
      <span className="w-12 text-right text-sm tabular-nums text-muted">{format(value)}</span>
    </span>
  );
}
