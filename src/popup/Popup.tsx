import { Check, Copy, PushPin, PushPinSlash, Warning, X } from "@phosphor-icons/react";
import { Tabs } from "radix-ui";
import { useCallback, useEffect, useLayoutEffect, useReducer, useRef, useState, type ReactNode } from "react";
import { LinkButton } from "../components/ui/Button";
import { IconButton } from "../components/ui/IconButton";
import { Segmented } from "../components/ui/Segmented";
import { strings } from "../i18n";
import { api, inApp, on } from "../lib/ipc";
import { mock } from "../lib/mock";
import { applyAppearance, followSystemTheme } from "../lib/theme";
import type { DictionaryEntry, Level, PopupEvent, Sense, Settings } from "../lib/types";
import { answerText, copyable, gradeBadge, initialState, isStreaming, reduce, tabs, type Explaining, type TabId } from "./state";

const t = strings.card;
/** Transparent space around the card for its shadow (matches popup.rs MARGIN). */
const MARGIN = 12;
const ICON = 15;

export function Popup() {
  const [state, dispatch] = useReducer(reduce, initialState);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [pinned, setPinned] = useState(false);
  const cardRef = useRef<HTMLDivElement>(null);
  const settingsRef = useRef<Settings | null>(null);

  const applySettings = useCallback((s: Settings) => {
    settingsRef.current = s;
    setSettings(s);
    applyAppearance(s.appearance);
  }, []);

  useEffect(() => {
    const loadSettings = () => void api.getSettings().then(applySettings);
    loadSettings();
    const stopTheme = followSystemTheme(() => settingsRef.current?.appearance);
    const unlisten = on<PopupEvent>("te://popup", (event) => {
      if (event.type === "open") setPinned(false);
      if (event.type === "settings") loadSettings();
      dispatch(event);
    });
    if (!inApp) {
      const params = new URLSearchParams(location.search);
      setTimeout(() => void mock.playDemo(params.get("demo") ?? "passage"), 50);
      // `?demo=word&tab=explain` opens a tab as a click on it would (screenshots, tests).
      const tab = params.get("tab");
      if (tab === "explain" || tab === "dictionary") setTimeout(() => dispatch({ type: "select_tab", tab }), 120);
    }
    return () => {
      stopTheme();
      void unlisten.then((f) => f());
    };
  }, [applySettings]);

  // The window is sized to the card, so clicks beside it reach the app underneath.
  useLayoutEffect(() => {
    const card = cardRef.current;
    if (!card) return;
    let frame = 0;
    const report = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => void api.popupResize(card.offsetHeight + 2 * MARGIN));
    };
    const observer = new ResizeObserver(report);
    observer.observe(card);
    report();
    return () => {
      observer.disconnect();
      cancelAnimationFrame(frame);
    };
  }, []);

  const togglePin = () => {
    setPinned((p) => {
      void api.popupPin(!p);
      return !p;
    });
  };

  // A word's explanation starts once its tab is opened, and only once per request.
  const asked = useRef(0);
  const requestId = state.view === "explain" && state.ai === "requested" ? state.id : 0;
  useEffect(() => {
    if (requestId && asked.current !== requestId) {
      asked.current = requestId;
      void api.explainNow(requestId);
    }
  }, [requestId]);

  const selectTab = (tab: TabId) => dispatch({ type: "select_tab", tab });

  return (
    <div className="p-3" style={{ width: 440 + 2 * MARGIN }}>
      <div
        ref={cardRef}
        role="dialog"
        aria-label={strings.app.name}
        className="flex max-h-[560px] flex-col overflow-hidden rounded-xl bg-surface text-text shadow-[var(--shadow-card)]"
      >
        {state.view === "explain" ? (
          <ExplainCard state={state} level={settings?.level ?? "plain"} pinned={pinned} onPin={togglePin} onTab={selectTab} />
        ) : (
          <MessageCard state={state} />
        )}
      </div>
    </div>
  );
}

/** Pin, copy and close: the same three on every tab. */
function Actions({ pinned, onPin, copyText }: { pinned?: boolean; onPin?: () => void; copyText?: string }) {
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    if (!copyText) return;
    await api.copyText(copyText);
    setCopied(true);
    setTimeout(() => setCopied(false), 1200);
  };
  return (
    <div className="flex shrink-0 items-center gap-0.5">
      {onPin && (
        <IconButton label={pinned ? t.unpin : t.pin} active={pinned} aria-pressed={pinned} onClick={onPin}>
          {pinned ? <PushPinSlash size={ICON} /> : <PushPin size={ICON} />}
        </IconButton>
      )}
      {copyText !== undefined && (
        <IconButton label={copied ? t.copied : t.copy} onClick={() => void copy()} disabled={!copyText}>
          {copied ? <Check size={ICON} /> : <Copy size={ICON} />}
        </IconButton>
      )}
      <IconButton label={t.close} onClick={() => void api.popupClose()}>
        <X size={ICON} />
      </IconButton>
    </div>
  );
}

function ExplainCard({
  state,
  level,
  pinned,
  onPin,
  onTab,
}: {
  state: Explaining;
  level: Level;
  pinned: boolean;
  onPin: () => void;
  onTab: (tab: TabId) => void;
}) {
  const list = tabs(state);
  return (
    <Tabs.Root value={state.tab} onValueChange={(v) => onTab(v as TabId)} className="flex min-h-0 flex-col">
      <div className="flex h-10 shrink-0 items-stretch gap-2 border-b border-border pr-1.5 pl-2">
        <Tabs.List aria-label={t.tabs} className="flex min-w-0 flex-1 items-stretch">
          {list.map((id) => (
            <Tabs.Trigger
              key={id}
              value={id}
              className="relative px-2.5 text-[13px] font-medium text-muted transition-colors hover:text-text data-[state=active]:text-text data-[state=active]:after:absolute data-[state=active]:after:inset-x-2.5 data-[state=active]:after:-bottom-px data-[state=active]:after:h-0.5 data-[state=active]:after:rounded-full data-[state=active]:after:bg-accent"
            >
              {t.tab[id]}
            </Tabs.Trigger>
          ))}
        </Tabs.List>
        <Actions pinned={pinned} onPin={onPin} copyText={copyable(state)} />
      </div>
      <Tabs.Content value="dictionary" className="min-h-0 overflow-y-auto focus-visible:outline-none" tabIndex={-1}>
        {state.dictionary && <DictionaryView entry={state.dictionary} offline={state.ai === "off"} />}
      </Tabs.Content>
      <Tabs.Content value="explain" className="flex min-h-0 flex-col focus-visible:outline-none" tabIndex={-1}>
        <ExplainView state={state} level={level} />
      </Tabs.Content>
    </Tabs.Root>
  );
}

function ExplainView({ state, level }: { state: Explaining; level: Level }) {
  const text = answerText(state);
  const streaming = isStreaming(state);
  const badge = gradeBadge(state);
  const report = state.report;
  const flagged = !state.error && report && (report.missing.length > 0 || report.added.length > 0);
  const passage = state.kind === "passage";

  return (
    <>
      <div key={state.id} className="card-in min-h-0 overflow-y-auto px-4 pt-3.5 pb-4" aria-live="polite" aria-busy={!state.done}>
        {state.error ? (
          <ErrorBlock message={state.error.message} detail={state.error.detail} />
        ) : text ? (
          <Reading text={text} streaming={streaming} />
        ) : state.loadingModel ? (
          <p className="text-[13px] text-muted">{t.loadingModel}</p>
        ) : (
          <Skeleton lines={passage ? 3 : 1} />
        )}
        {state.truncated && !state.error && <p className="mt-3 text-[12px] text-muted">{t.truncated}</p>}
      </div>

      {flagged && (
        <div className="flex gap-2 border-t border-border bg-warn-soft px-4 py-2 text-[12px] text-warn" title={t.checkHint}>
          <Warning size={14} weight="bold" className="mt-px shrink-0" aria-hidden />
          <div>
            {report.missing.length > 0 && (
              <p>
                {t.checkMissing} <strong className="font-semibold">{report.missing.map((f) => f.text).join(", ")}</strong>
              </p>
            )}
            {report.added.length > 0 && (
              <p>
                {t.checkAdded} <strong className="font-semibold">{report.added.map((f) => f.text).join(", ")}</strong>
              </p>
            )}
          </div>
        </div>
      )}

      {passage && (
        <div className="flex h-10 shrink-0 items-center justify-between gap-3 border-t border-border px-2.5">
          <Segmented<Level>
            size="sm"
            label={strings.level.label}
            value={level}
            onChange={(l) => void api.setLevel(l)}
            options={[
              { value: "simpler", label: strings.level.simpler, title: strings.level.simplerHint },
              { value: "plain", label: strings.level.plain, title: strings.level.plainHint },
              { value: "clearer", label: strings.level.clearer, title: strings.level.clearerHint },
            ]}
          />
          {badge && (
            <span title={t.gradeTitle} className="pr-1.5 text-[12px] text-muted">
              {t.gradeLabel} <span className="font-medium text-text tabular-nums">{t.gradeShort(badge.before, badge.after)}</span>
            </span>
          )}
        </div>
      )}
    </>
  );
}

/** “Ubiquitous,” → Ubiquitous */
export function bareTerm(s: string): string {
  return s.trim().replace(/^[^\p{L}\p{M}\p{N}]+|[^\p{L}\p{M}\p{N}]+$/gu, "");
}

/** A word's senses from the bundled dictionary, grouped by part of speech. */
function DictionaryView({ entry, offline }: { entry: DictionaryEntry; offline: boolean }) {
  // Senses are numbered straight through, grouped under each part of speech.
  const groups = entry.senses.slice(0, 5).reduce<{ pos: string; items: (Sense & { n: number })[] }[]>((acc, s, i) => {
    const last = acc[acc.length - 1];
    if (last && last.pos === s.pos) last.items.push({ ...s, n: i + 1 });
    else acc.push({ pos: s.pos, items: [{ ...s, n: i + 1 }] });
    return acc;
  }, []);
  return (
    <div className="card-in px-4 pt-3 pb-4">
      <p className="text-[19px] font-semibold tracking-[-0.015em]">{bareTerm(entry.lemma)}</p>
      <div className="mt-2 space-y-3">
        {groups.map((g, gi) => (
          <section key={gi}>
            <h3 className="mb-1 text-[12px] text-muted italic">{g.pos}</h3>
            <ol className="space-y-2">
              {g.items.map((sense) => {
                return (
                  <li key={sense.n} className="grid grid-cols-[1.25rem_1fr] text-[14px] leading-snug">
                    <span className="pt-px font-mono text-[11px] text-muted tabular-nums">{sense.n}</span>
                    <div>
                      <p>{sense.definition}</p>
                      {sense.example && <p className="mt-0.5 text-muted">“{sense.example}”</p>}
                      {sense.synonyms.length > 0 && (
                        <p className="mt-0.5 text-[12px] text-muted">
                          {t.also}: <span className="text-text">{sense.synonyms.join(", ")}</span>
                        </p>
                      )}
                    </div>
                  </li>
                );
              })}
            </ol>
          </section>
        ))}
      </div>
      {offline && <p className="mt-4 border-t border-border pt-2.5 text-[12px] text-muted">{t.noModelForWords}</p>}
    </div>
  );
}

/** The answer, in the reading font, paragraph by paragraph. */
export function Reading({ text, streaming }: { text: string; streaming: boolean }) {
  const paragraphs = text.split(/\n{2,}/);
  return (
    <div
      className="space-y-3 whitespace-pre-line [overflow-wrap:anywhere]"
      style={{ fontFamily: "var(--reading-font)", fontSize: "var(--reading-size, 15px)", lineHeight: "var(--reading-leading, 1.6)" }}
    >
      {paragraphs.map((p, i) => (
        <p key={i} className={streaming && i === paragraphs.length - 1 ? "stream-caret" : undefined}>
          {p}
        </p>
      ))}
    </div>
  );
}

function Skeleton({ lines = 3 }: { lines?: number }) {
  return (
    <div className="space-y-2.5 py-1.5" role="status" aria-label={t.thinking}>
      {[94, 100, 68].slice(0, lines).map((w, i) => (
        <div key={i} className="skeleton-line h-2 rounded-full" style={{ width: `${w}%` }} />
      ))}
    </div>
  );
}

function ErrorBlock({ message, detail }: { message: string; detail: string | null }) {
  return (
    <div className="space-y-1.5 text-[13px]">
      <p className="font-semibold text-danger">{t.errorTitle}</p>
      <p className="text-muted first-letter:uppercase">{message}.</p>
      {detail && (
        <details className="text-[12px] text-muted">
          <summary className="select-none">{t.details}</summary>
          <pre className="mt-1 max-h-32 overflow-auto rounded-md bg-surface-2 p-2 font-mono text-[11px] whitespace-pre-wrap">{detail}</pre>
        </details>
      )}
      <LinkButton className="text-[12px]" onClick={() => void api.openMain()}>
        {t.settings}
      </LinkButton>
    </div>
  );
}

function MessageCard({ state }: { state: Exclude<ReturnType<typeof reduce>, Explaining> }) {
  if (state.view === "empty") return <Bar />;
  const [title, body, action] =
    state.view === "no_selection"
      ? [t.noSelectionTitle, t.noSelection(state.hotkey), null]
      : state.view === "no_model"
        ? [t.noModelTitle, t.noModel, t.setUp]
        : [t.captureFailed, state.message, null];
  return (
    <>
      <Bar>
        <span className="px-2 text-[13px] font-semibold">{title}</span>
      </Bar>
      <div className="card-in space-y-3 px-4 pt-3 pb-4 text-[13px] text-muted">
        <p>{body}</p>
        {action && (
          <button
            type="button"
            onClick={() => void api.openMain()}
            className="h-7 rounded-md bg-control px-3 text-[13px] font-medium text-on-control transition-colors hover:bg-control-hover"
          >
            {action}
          </button>
        )}
      </div>
    </>
  );
}

function Bar({ children }: { children?: ReactNode }) {
  return (
    <div className="flex h-10 shrink-0 items-center gap-2 border-b border-border pr-1.5 pl-2">
      <div className="flex min-w-0 flex-1 items-center">{children}</div>
      <Actions />
    </div>
  );
}
