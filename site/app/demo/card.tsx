import type { ReactNode } from "react";
import { Icon } from "../icons";

const WEB_TABS = ["Google", "Bing", "DuckDuckGo", "Translate"] as const;
const MODES = [
  ["explain", "Explain"],
  ["translate", "Translate"],
] as const;
const LEVELS = [
  ["simpler", "Simpler"],
  ["plain", "Plain"],
  ["clearer", "Clearer"],
] as const;

export function PlainlyCard({
  quote,
  tab,
  mode,
  sourceLabel = "English",
  source,
  answerLabel,
  answer,
  lead,
  level,
  grade,
  showGrade = true,
  className,
}: {
  quote: string;
  tab: "google" | "explain";
  mode: "explain" | "translate";
  sourceLabel?: string;
  source: string;
  answerLabel: string;
  answer: string;
  /** Explain-state answer kept in the DOM so a timeline can swap to `answer` without a re-render. */
  lead?: { label: string; text: string };
  level: "simpler" | "plain" | "clearer";
  grade: { from: number; to?: number };
  showGrade?: boolean;
  className?: string;
}) {
  const google = tab === "google";
  const translating = mode === "translate";
  return (
    <div
      data-part="card"
      className={`w-[480px] overflow-hidden rounded-xl bg-surface text-text opacity-0 ${className ?? ""}`}
      style={{ boxShadow: "var(--shadow), 0 0 0 1px var(--line)" }}
    >
      <div className="flex h-11 items-center gap-3 pr-1.5 pl-4">
        {/* A span, not a <p>: globals.css gives every <p> `text-wrap: pretty`, which turns wrapping back on. */}
        <span className="block min-w-0 flex-1 truncate text-[13px] text-muted" title={quote}>
          {"\u201C"}
          {quote}
          {"\u201D"}
        </span>
        <span className="grid size-7 shrink-0 place-items-center text-muted" aria-hidden="true">
          <Icon name="x" size={15} />
        </span>
      </div>

      <div className="flex items-center gap-0.5 border-b border-line px-2 pb-2">
        {WEB_TABS.map((name) => {
          const on = google && name === "Google";
          return (
            <span
              key={name}
              data-part={name === "Google" ? "google-tab" : undefined}
              className={`grid h-7 shrink-0 place-items-center rounded-md px-2.5 text-[13px] font-medium whitespace-nowrap ${
                on ? "bg-sunken text-text" : "text-muted"
              }`}
            >
              {name}
            </span>
          );
        })}
        <span className="min-w-2 flex-1" />
        <span
          data-part="explain-tab"
          className={`flex h-7 shrink-0 items-center gap-1.5 rounded-md border px-2.5 text-[13px] font-medium whitespace-nowrap ${
            google ? "border-line bg-surface text-text" : "border-transparent bg-ink text-on-ink"
          }`}
        >
          <Icon name="sparkle" size={13} />
          Explain with AI
        </span>
      </div>

      <div data-part="web" className={google ? "block" : "hidden"}>
        <div className="h-[360px] space-y-5 px-4 pt-4">
          {[
            ["w-24", "w-[78%]", "w-full", "w-2/3"],
            ["w-16", "w-[64%]", "w-full", "w-3/5"],
            ["w-28", "w-[84%]", "w-[92%]", "w-1/2"],
          ].map((lines, i) => (
            <div key={i} className="space-y-2">
              {lines.map((w, j) => (
                <div key={j} className={`${j === 1 ? "h-3" : "h-2"} rounded-full bg-sunken ${w}`} />
              ))}
            </div>
          ))}
        </div>
        <div className="flex h-9 items-center justify-between gap-3 border-t border-line px-3">
          <span className="flex min-w-0 items-center gap-1.5 text-[12px] text-muted">
            <Icon name="globe" size={13} />
            google.com
          </span>
          <span className="flex shrink-0 items-center gap-1.5 text-[12px] font-medium text-muted">
            Open in browser
            <Icon name="open" size={13} />
          </span>
        </div>
      </div>

      <div data-part="explain" className={`flex-col ${google ? "hidden" : "flex"}`}>
        <div className="px-4 pt-3">
          <Segmented value={mode} options={MODES} part={(id) => `mode-${id}`} />
        </div>
        <div className="px-4 pt-3 pb-4">
          <div className="rounded-lg bg-sunken px-[12px] py-[10px]">
            <Label>
              {sourceLabel}
              <span className="ml-1.5 font-normal tracking-normal normal-case">detected</span>
            </Label>
            <p className="line-clamp-4 text-[13.5px] leading-relaxed">{source}</p>
          </div>
          <div className="mt-3.5">
            {lead ? (
              <>
                <div data-part="lead">
                  <Label>{lead.label}</Label>
                  <p data-part="lead-answer" className="text-[15px] leading-[1.6] text-text">
                    <Words text={lead.text} />
                  </p>
                </div>
                <div data-part="translated" className="hidden">
                  <Label>
                    {answerLabel}
                    <Chevron />
                  </Label>
                  <p data-part="answer" className="text-[15px] leading-[1.6] text-text">
                    <Words text={answer} />
                  </p>
                </div>
              </>
            ) : (
              <>
                <Label>
                  {answerLabel}
                  {translating && <Chevron />}
                </Label>
                <p data-part="answer" className="text-[15px] leading-[1.6] text-text">
                  <Words text={answer} />
                </p>
              </>
            )}
            {translating && (
              <span
                data-part="compare"
                className={`mt-3 inline-flex items-center gap-1.5 text-[12.5px] text-muted ${lead ? "opacity-0" : ""}`}
              >
                <Icon name="translate" size={13} />
                Compare in the Translate tab
              </span>
            )}
          </div>
        </div>
        <div className="mt-auto flex h-10 items-center justify-between gap-3 border-t border-line px-2">
          <Segmented value={level} options={LEVELS} part={(id) => `level-${id}`} />
          <span className="flex items-center gap-2">
            {showGrade && (
              <span className="text-[12px] text-muted">
                Grade <span className="font-medium text-text tabular-nums">{grade.from}</span>
                {grade.to !== undefined && (
                  <span data-part="grade-to" className="opacity-0">
                    {" \u2192 "}
                    <span data-grade-num="" className="inline-block min-w-[2ch] text-right font-medium text-text tabular-nums">
                      {grade.to}
                    </span>
                  </span>
                )}
              </span>
            )}
            <span className="grid size-7 place-items-center text-muted" aria-hidden="true">
              <Icon name="copy" size={15} />
            </span>
          </span>
        </div>
      </div>
    </div>
  );
}

function Chevron() {
  return (
    <svg width="10" height="10" viewBox="0 0 12 12" aria-hidden="true" className="ml-1 shrink-0">
      <path d="M2.2 4.3 6 8l3.8-3.7" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function Label({ children }: { children: ReactNode }) {
  return <div className="mb-1 flex min-h-6 items-center text-[11px] font-semibold tracking-[0.06em] text-muted uppercase">{children}</div>;
}

function Segmented({
  value,
  options,
  part,
}: {
  value: string;
  options: readonly (readonly [string, string])[];
  part?: (id: string) => string;
}) {
  return (
    <div className="inline-flex rounded-md bg-sunken p-[2px]">
      {options.map(([id, label]) => {
        const on = id === value;
        return (
          <span
            key={id}
            data-part={part?.(id)}
            className={`grid h-6 place-items-center rounded-[5px] px-2 text-[12px] font-medium ${
              on ? "bg-surface text-text shadow-[0_0_0_1px_var(--line)]" : "text-muted"
            }`}
          >
            {label}
          </span>
        );
      })}
    </div>
  );
}

/** One span per word, trailing space included, so a timeline can reveal them. */
function Words({ text }: { text: string }) {
  const words = text.split(/\s+/).filter(Boolean);
  return words.map((word, i) => (
    <span key={`${word}-${i}`} data-word="" className="opacity-0">
      <span className="relative inline-block">
        <span data-underline="" className="absolute inset-x-0 bottom-0 h-[40%] origin-left scale-x-0 rounded-[2px] bg-[var(--mark)]" />
        <span className="relative">{word}</span>
      </span>
      {i < words.length - 1 ? " " : ""}
    </span>
  ));
}
