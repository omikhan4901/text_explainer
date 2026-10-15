import { Field, inputClass } from "../../components/ui/Field";
import { Group, Section } from "../../components/ui/Section";
import { Segmented } from "../../components/ui/Segmented";
import { strings } from "../../i18n";
import { ACCENTS } from "../../lib/theme";
import type { AccentName, Appearance, Level, ReadingFont, Theme } from "../../lib/types";
import { Reading } from "../../popup/Popup";
import { PageTitle } from "../App";
import { useData } from "../data";

const t = strings.reading;
/** Each accent's light and dark value (tokens.css), so every swatch shows on both themes. */
const SWATCH: Record<AccentName, [string, string]> = {
  ink: ["#1b1b1a", "#ececea"],
  plum: ["#6c3d78", "#d3aedc"],
  saffron: ["#8f5208", "#e7b46a"],
  garnet: ["#9b2c3c", "#f0a0ab"],
};

export function ReadingPage() {
  const { settings, info, update } = useData();
  const a = settings.appearance;
  const setA = (patch: Partial<Appearance>) => void update({ appearance: { ...a, ...patch } });
  const dark = document.documentElement.classList.contains("dark");
  const output = settings.output.mode === "fixed" ? settings.output.code : "";
  return (
    <>
      <PageTitle lead={t.lead}>{t.title}</PageTitle>

      <Section title={t.preview}>
        <div className="rounded-xl bg-surface px-5 py-4 shadow-[var(--shadow-card)]">
          <Reading text={t.sample} streaming={false} />
        </div>
      </Section>

      <Section title={t.explaining}>
        <Group>
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
        </Group>
      </Section>

      <Section title={t.text}>
        <Group>
          <Field label={t.font} hint={t.fontHint}>
            <Segmented<ReadingFont>
              label={t.font}
              value={a.reading_font}
              onChange={(reading_font) => setA({ reading_font })}
              options={(["plex", "atkinson", "serif"] as const).map((f) => ({ value: f, label: t.fonts[f] }))}
            />
          </Field>
          <Field label={t.size}>
            <Range label={t.size} value={a.font_size} min={12} max={24} step={1} format={(v) => `${v}px`} onChange={(font_size) => setA({ font_size })} />
          </Field>
          <Field label={t.spacing}>
            <Range
              label={t.spacing}
              value={a.line_height}
              min={1.2}
              max={2.2}
              step={0.1}
              format={(v) => v.toFixed(1)}
              onChange={(line_height) => setA({ line_height })}
            />
          </Field>
        </Group>
      </Section>

      <Section title={t.look}>
        <Group>
          <Field label={t.theme}>
            <Segmented<Theme>
              label={t.theme}
              value={a.theme}
              onChange={(theme) => setA({ theme })}
              options={(["system", "light", "dark"] as const).map((v) => ({ value: v, label: t.themes[v] }))}
            />
          </Field>
          <Field label={t.accent} hint={t.accentHint}>
            <div role="radiogroup" aria-label={t.accent} className="flex gap-1.5">
              {ACCENTS.map((name) => (
                <button
                  key={name}
                  type="button"
                  role="radio"
                  aria-checked={a.accent === name}
                  aria-label={t.accents[name]}
                  title={t.accents[name]}
                  onClick={() => setA({ accent: name })}
                  className={`grid size-7 place-items-center rounded-full transition-shadow ${a.accent === name ? "shadow-[0_0_0_1.5px_var(--text)]" : "hover:shadow-[0_0_0_1.5px_var(--border-strong)]"}`}
                >
                  <span className="size-5 rounded-full" style={{ background: SWATCH[name][dark ? 1 : 0] }} />
                </button>
              ))}
            </div>
          </Field>
        </Group>
      </Section>
    </>
  );
}

function Range({
  label,
  value,
  min,
  max,
  step,
  format,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  format: (v: number) => string;
  onChange: (v: number) => void;
}) {
  return (
    <span className="flex items-center gap-3">
      <input
        type="range"
        aria-label={label}
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="w-36 accent-[var(--accent)]"
      />
      <span className="w-10 text-right font-mono text-[12px] text-muted tabular-nums">{format(value)}</span>
    </span>
  );
}
