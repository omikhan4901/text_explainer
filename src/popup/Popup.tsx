import { ArrowSquareOut, BookmarkSimple, Check, Copy, Globe, PushPin, PushPinSlash, SpeakerHigh, Sparkle, Warning, X } from "@phosphor-icons/react";
import { Tabs } from "radix-ui";
import { useCallback, useEffect, useLayoutEffect, useReducer, useRef, useState, type ReactNode } from "react";
import { LinkButton } from "../components/ui/Button";
import { IconButton } from "../components/ui/IconButton";
import { Segmented } from "../components/ui/Segmented";
import { strings } from "../i18n";
import { api, inApp, on } from "../lib/ipc";
import { mock } from "../lib/mock";
import { applyAppearance, followSystemTheme } from "../lib/theme";
import { bareTerm, savableTerm } from "../lib/words";
import type { AiTask, DictionaryEntry, Level, PopupEvent, Sense, Settings } from "../lib/types";
import {
  answerText,
  copyable,
  gradeBadge,
  initialState,
  isStreaming,
  isWord,
  isWorking,
  reduce,
  tabs,
  webSourceOf,
  type Explaining,
  type TabId,
} from "./state";

const t = strings.card;
/** Card width and the transparent space around it for its shadow (match popup.rs). */
const WIDTH = 480;
const MARGIN = 12;
/** Height of the area a web page fills, and of the footer under every tab. */
const PAGE_HEIGHT = 400;
const FOOTER_HEIGHT = 40;
const ICON = 15;

export function Popup() {
  const [state, dispatch] = useReducer(reduce, initialState);
  const setTask = useCallback((id: number, task: AiTask) => {
    dispatch({ type: "set_task", task });
    void api.setAiTask(id, task);
  }, []);
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
      // `?tab=explain` (or `dictionary`, `web:bing`) opens a tab as a click would.
      const tab = params.get("tab") as TabId | null;
      if (tab) setTimeout(() => dispatch({ type: "select_tab", tab }), 120);
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

  // The AI tab starts the model once per request, when it is opened.
  const asked = useRef(0);
  const requestId = state.view === "explain" && state.ai === "requested" ? state.id : 0;
  useEffect(() => {
    if (requestId && asked.current !== requestId) {
      asked.current = requestId;
      void api.explainNow(requestId);
    }
  }, [requestId]);

  // Only a web tab shows the page; any other tab hides it.
  const onWebTab = state.view === "explain" && webSourceOf(state.tab) !== null;
  useEffect(() => {
    if (!onWebTab) void api.webHide();
  }, [onWebTab]);

  const togglePin = () => {
    setPinned((p) => {
      void api.popupPin(!p);
      return !p;
    });
  };

  return (
    <div className="p-3" style={{ width: WIDTH + 2 * MARGIN }}>
      <div
        ref={cardRef}
        role="dialog"
        aria-label={strings.app.name}
        className="flex max-h-[560px] flex-col overflow-hidden rounded-xl bg-surface text-text shadow-[var(--shadow-card)]"
      >
        {state.view === "explain" ? (
          <ExplainCard
            state={state}
            level={settings?.level ?? "plain"}
            pinned={pinned}
            onPin={togglePin}
            onTab={(tab) => dispatch({ type: "select_tab", tab })}
            onTask={(task) => setTask(state.id, task)}
          />
        ) : (
          <MessageCard state={state} pinned={pinned} onPin={togglePin} />
        )}
      </div>
    </div>
  );
}

function PinClose({ pinned, onPin }: { pinned: boolean; onPin: () => void }) {
  return (
    <div className="flex shrink-0 items-center gap-0.5">
      <IconButton label={pinned ? t.unpin : t.pin} active={pinned} aria-pressed={pinned} onClick={onPin}>
        {pinned ? <PushPinSlash size={ICON} /> : <PushPin size={ICON} />}
      </IconButton>
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
  onTask,
}: {
  state: Explaining;
  level: Level;
  pinned: boolean;
  onPin: () => void;
  onTab: (tab: TabId) => void;
  onTask: (task: AiTask) => void;
}) {
  const word = isWord(state);
  // With web tabs every tab keeps the page's height, so switching never resizes the card.
  const steady = state.web.length > 0 ? { height: PAGE_HEIGHT + FOOTER_HEIGHT } : undefined;
  return (
    <Tabs.Root value={state.tab} onValueChange={(v) => onTab(v as TabId)} className="flex min-h-0 flex-col">
      <div className="flex h-11 shrink-0 items-center gap-3 pr-1.5 pl-4">
        <p className="min-w-0 flex-1 truncate" title={state.query}>
          {word ? (
            <span className="text-[15px] font-semibold tracking-[-0.01em]">{bareTerm(state.query)}</span>
          ) : (
            <span className="text-[13px] text-muted">“{state.query}”</span>
          )}
        </p>
        <div className="flex shrink-0 items-center gap-0.5">
          <WordActions state={state} />
          <PinClose pinned={pinned} onPin={onPin} />
        </div>
      </div>
      {/* Only the AI tab (offline, no dictionary entry): no row of tabs to choose from. */}
      {tabs(state).length > 1 ? <TabRail state={state} /> : <div className="border-b border-border" />}

      {state.web.map((w) => (
        <Tabs.Content key={w.id} value={`web:${w.id}`} className="focus-visible:outline-none">
          <WebPane requestId={state.id} source={w.id} name={w.name} />
        </Tabs.Content>
      ))}
      <Tabs.Content value="dictionary" className="flex min-h-0 flex-col focus-visible:outline-none" style={steady}>
        {state.dictionary && <DictionaryPane entry={state.dictionary} copyText={copyable(state)} />}
      </Tabs.Content>
      <Tabs.Content value="explain" className="flex min-h-0 flex-col focus-visible:outline-none" style={steady}>
        <ExplainPane state={state} level={level} onTask={onTask} />
      </Tabs.Content>
    </Tabs.Root>
  );
}

/** The tabs: searches and the dictionary scroll; "Explain with AI" stays pinned in reach. */
function TabRail({ state }: { state: Explaining }) {
  const scroller = useRef<HTMLDivElement>(null);
  const [scrolled, setScrolled] = useState(false);
  const local = tabs(state).filter((id) => id !== "explain");
  const measure = () => setScrolled((scroller.current?.scrollLeft ?? 0) > 1);
  // Keep the open tab in view, clear of the pinned AI tab.
  useLayoutEffect(() => {
    const list = scroller.current;
    const active = list?.querySelector<HTMLElement>("[role=tab][data-state=active]");
    const pinned = list?.querySelector<HTMLElement>("[role=tab]:last-child");
    if (!list || !active || !pinned || active === pinned) return;
    const view = list.getBoundingClientRect();
    const tab = active.getBoundingClientRect();
    const covered = view.right - pinned.getBoundingClientRect().width - 24;
    if (tab.right > covered) list.scrollLeft += tab.right - covered;
    else if (tab.left < view.left + 8) list.scrollLeft -= view.left + 8 - tab.left;
  }, [state.tab]);
  const working = isWorking(state);

  return (
    <Tabs.List
      ref={scroller}
      aria-label={t.tabs}
      onScroll={measure}
      onWheel={(e) => {
        // The card never has focus, so a mouse wheel is how a long row of tabs scrolls.
        if (scroller.current && Math.abs(e.deltaY) > Math.abs(e.deltaX)) scroller.current.scrollLeft += e.deltaY;
      }}
      className={`flex shrink-0 items-center gap-0.5 overflow-x-auto border-b border-border px-2 pb-2 [scrollbar-width:none] ${scrolled ? "[mask-image:linear-gradient(90deg,transparent,#000_24px)]" : ""}`}
    >
      {local.map((id) => (
        <Tabs.Trigger
          key={id}
          value={id}
          className="h-7 shrink-0 rounded-md px-2.5 text-[13px] font-medium whitespace-nowrap text-muted transition-colors hover:bg-surface-2 hover:text-text data-[state=active]:bg-surface-2 data-[state=active]:text-text"
        >
          {tabName(state, id)}
        </Tabs.Trigger>
      ))}
      <span className="min-w-4 flex-1" aria-hidden />
      {/* Pinned to the right edge; tabs scrolling under it fade out behind its shadow. */}
      <Tabs.Trigger
        value="explain"
        className="sticky right-0 flex h-7 shrink-0 items-center gap-1.5 rounded-md border border-border-strong bg-surface px-2.5 text-[13px] font-medium whitespace-nowrap text-text shadow-[-16px_0_10px_0_var(--surface)] transition-colors hover:bg-surface-2 data-[state=active]:border-control data-[state=active]:bg-control data-[state=active]:text-on-control"
      >
        <Sparkle size={13} weight="fill" className={working ? "animate-pulse" : undefined} aria-hidden />
        {t.tab.explain}
      </Tabs.Trigger>
    </Tabs.List>
  );
}

function tabName(state: Explaining, id: TabId): string {
  const source = webSourceOf(id);
  if (source) return state.web.find((w) => w.id === source)?.name ?? source;
  return id === "dictionary" ? t.tab.dictionary : t.tab.explain;
}

/** A web source's page. The app lays its web view over this area; until the page paints,
 * a placeholder shaped like search results shows. */
function WebPane({ requestId, source, name }: { requestId: number; source: string; name: string }) {
  const area = useRef<HTMLDivElement>(null);
  const [page, setPage] = useState<{ key: string; url: string | null; error: string | null }>({ key: "", url: null, error: null });
  const key = `${requestId}:${source}`;

  useLayoutEffect(() => {
    const el = area.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    const background = getComputedStyle(document.documentElement).getPropertyValue("--surface").trim() || null;
    let live = true;
    api
      .webShow(source, { x: r.left, y: r.top, width: r.width, height: r.height }, background)
      .then((url) => live && setPage({ key, url, error: null }))
      .catch((e: unknown) => live && setPage({ key, url: null, error: String(e) }));
    return () => {
      live = false;
    };
  }, [key, source]);

  const current = page.key === key ? page : { url: null, error: null };
  const host = current.url ? hostOf(current.url) : name;
  return (
    <>
      <div ref={area} className="relative overflow-hidden" style={{ height: PAGE_HEIGHT }} aria-busy={!current.url && !current.error}>
        {current.error ? (
          <div className="px-4 pt-4 text-[13px]">
            <p className="font-semibold text-danger">{t.webFailed(name)}</p>
            <p className="mt-1 text-muted first-letter:uppercase">{current.error}</p>
          </div>
        ) : (
          <ResultsSkeleton label={t.loadingPage(host)} />
        )}
      </div>
      <Footer>
        <span className="flex min-w-0 items-center gap-1.5 pl-1.5 text-[12px] text-muted">
          <Globe size={13} className="shrink-0" aria-hidden />
          <span className="truncate">{host}</span>
        </span>
        <button
          type="button"
          onClick={() => void api.webOpenInBrowser(source)}
          className="flex h-7 shrink-0 items-center gap-1.5 rounded-md px-2 text-[12px] font-medium text-muted transition-colors hover:bg-surface-2 hover:text-text"
        >
          {t.openInBrowser} <ArrowSquareOut size={13} aria-hidden />
        </button>
      </Footer>
    </>
  );
}

export function hostOf(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}

function ResultsSkeleton({ label }: { label: string }) {
  return (
    <div className="space-y-5 px-4 pt-4" role="status" aria-label={label}>
      {[78, 64, 84].map((w, i) => (
        <div key={i} className="space-y-2">
          <div className="skeleton-line h-2 w-24 rounded-full" />
          <div className="skeleton-line h-3 rounded-full" style={{ width: `${w}%` }} />
          <div className="skeleton-line h-2 rounded-full" />
          <div className="skeleton-line h-2 w-2/3 rounded-full" />
        </div>
      ))}
    </div>
  );
}

function Footer({ children }: { children: ReactNode }) {
  return <div className="mt-auto flex h-10 shrink-0 items-center justify-between gap-3 border-t border-border px-2">{children}</div>;
}

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    if (!text) return;
    await api.copyText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 1200);
  };
  return (
    <IconButton label={copied ? t.copied : t.copy} onClick={() => void copy()} disabled={!text}>
      {copied ? <Check size={ICON} /> : <Copy size={ICON} />}
    </IconButton>
  );
}

/** Explain or Translate: what the AI tab does for this selection. */
function TaskSwitch({ state, onTask }: { state: Explaining; onTask: (task: AiTask) => void }) {
  const tr = state.translate;
  const why = tr.state === "same_language" ? strings.translate.already(tr.name) : tr.state === "unsupported" ? strings.translate.webOnly(tr.name) : undefined;
  return (
    <div className="flex items-center gap-2.5 px-4 pt-3">
      <Segmented<AiTask>
        size="sm"
        label={strings.translate.label}
        value={state.task}
        onChange={onTask}
        options={[
          { value: "explain", label: strings.translate.explain },
          { value: "translate", label: strings.translate.to(tr.name), title: why, disabled: tr.state !== "available" },
        ]}
      />
      {why && <span className="min-w-0 truncate text-[12px] text-muted">{why}</span>}
    </div>
  );
}

function ExplainPane({ state, level, onTask }: { state: Explaining; level: Level; onTask: (task: AiTask) => void }) {
  const text = answerText(state);
  const streaming = isStreaming(state);
  const badge = gradeBadge(state);
  const report = state.report;
  const flagged = !state.error && report && (report.missing.length > 0 || report.added.length > 0);
  const passage = state.kind === "passage";

  if (state.ai === "off") {
    return (
      <div className="card-in space-y-2 px-4 pt-3.5 pb-4 text-[13px]">
        <p className="font-semibold">{t.noModelTitle}</p>
        <p className="text-muted">{t.aiNeedsModel}</p>
        <button
          type="button"
          onClick={() => void api.openMain()}
          className="h-7 rounded-md bg-control px-3 text-[13px] font-medium text-on-control transition-colors hover:bg-control-hover"
        >
          {t.setUp}
        </button>
      </div>
    );
  }

  return (
    <>
      <TaskSwitch state={state} onTask={onTask} />
      <div key={`${state.id}-${state.task}`} className="card-in min-h-0 overflow-y-auto px-4 pt-3 pb-4" aria-live="polite" aria-busy={!state.done}>
        {state.error ? (
          <ErrorBlock message={state.error.message} detail={state.error.detail} />
        ) : text ? (
          <Reading text={text} streaming={streaming} />
        ) : (
          <div role="status">
            <Skeleton lines={passage ? 3 : 2} />
            <p className="mt-3 text-[12px] text-muted">{state.loadingModel ? t.loadingModel : t.thinking}</p>
          </div>
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

      <Footer>
        {passage && state.task === "explain" ? (
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
        ) : (
          <span className="pl-1.5 text-[12px] text-muted">{t.onThisComputer}</span>
        )}
        <span className="flex items-center gap-2">
          {badge && (
            <span title={t.gradeTitle} className="text-[12px] text-muted">
              {t.gradeLabel} <span className="font-medium text-text tabular-nums">{t.gradeShort(badge.before, badge.after)}</span>
            </span>
          )}
          <CopyButton text={state.done && !state.error ? text : ""} />
        </span>
      </Footer>
    </>
  );
}

export { bareTerm };

/** Listen (words and short terms) and Save to Words, for selections short enough to keep.
 * A saved word takes the AI's answer when it finishes. */
function WordActions({ state }: { state: Explaining }) {
  const target = savableTerm(state.query, state.kind);
  const term = target?.term ?? "";
  const kind = target?.kind ?? null;
  const [saved, setSaved] = useState<{ id: number; on: boolean } | null>(null);
  const isSaved = saved?.id === state.id && saved.on;
  const answer = state.done && !state.error ? answerText(state) : "";
  /** The last answer stored, so each one is saved once. */
  const stored = useRef("");

  // Whether it's in the book already, once per request.
  useEffect(() => {
    if (!term) return;
    let live = true;
    api
      .isSaved(term)
      .then((on) => live && setSaved({ id: state.id, on }))
      .catch(() => live && setSaved(null));
    return () => {
      live = false;
    };
  }, [state.id, term]);

  useEffect(() => {
    const key = `${state.id}:${answer}`;
    if (!isSaved || !answer || !kind || stored.current === key) return;
    stored.current = key;
    void api.saveWord({ term, kind, definition: null, explanation: answer }).catch(() => undefined);
  }, [isSaved, answer, kind, term, state.id]);

  if (!kind) return null;
  const toggle = async () => {
    try {
      if (isSaved) {
        await api.removeWord(term);
        setSaved({ id: state.id, on: false });
      } else {
        const definition = state.dictionary?.senses[0]?.definition ?? null;
        await api.saveWord({ term, kind, definition, explanation: answer || null });
        stored.current = `${state.id}:${answer}`;
        setSaved({ id: state.id, on: true });
      }
    } catch {
      // The book couldn't be written; the button keeps showing what is stored.
    }
  };
  return (
    <>
      {kind !== "passage" && (
        <IconButton label={t.listen} onClick={() => void api.speak(term).catch(() => undefined)}>
          <SpeakerHigh size={ICON} />
        </IconButton>
      )}
      <IconButton label={t.save} active={isSaved} aria-pressed={isSaved} onClick={() => void toggle()}>
        <BookmarkSimple size={ICON} weight={isSaved ? "fill" : "regular"} />
      </IconButton>
    </>
  );
}

/** A word's senses from the bundled dictionary, grouped by part of speech. */
function DictionaryPane({ entry, copyText }: { entry: DictionaryEntry; copyText: string }) {
  // Senses are numbered straight through, grouped under each part of speech.
  const groups = entry.senses.slice(0, 5).reduce<{ pos: string; items: (Sense & { n: number })[] }[]>((acc, s, i) => {
    const last = acc[acc.length - 1];
    if (last && last.pos === s.pos) last.items.push({ ...s, n: i + 1 });
    else acc.push({ pos: s.pos, items: [{ ...s, n: i + 1 }] });
    return acc;
  }, []);
  return (
    <>
      <div className="card-in min-h-0 space-y-3 overflow-y-auto px-4 pt-3.5 pb-4">
        {groups.map((g, gi) => (
          <section key={gi}>
            <h3 className="mb-1 text-[12px] text-muted italic">{g.pos}</h3>
            <ol className="space-y-2">
              {g.items.map((sense) => (
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
              ))}
            </ol>
          </section>
        ))}
      </div>
      <Footer>
        <span className="pl-1.5 text-[12px] text-muted">{t.dictionarySource}</span>
        <CopyButton text={copyText} />
      </Footer>
    </>
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
    <div className="space-y-2.5 py-1.5" aria-hidden>
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

function MessageCard({ state, pinned, onPin }: { state: Exclude<ReturnType<typeof reduce>, Explaining>; pinned: boolean; onPin: () => void }) {
  if (state.view === "empty") {
    return (
      <div className="flex h-11 items-center justify-end pr-1.5">
        <PinClose pinned={pinned} onPin={onPin} />
      </div>
    );
  }
  const [title, body, action] =
    state.view === "no_selection"
      ? [t.noSelectionTitle, t.noSelection(state.hotkey), null]
      : state.view === "no_model"
        ? [t.noModelTitle, t.noModel, t.setUp]
        : [t.captureFailed, state.message, null];
  return (
    <>
      <div className="flex h-11 shrink-0 items-center gap-3 pr-1.5 pl-4">
        <p className="min-w-0 flex-1 truncate text-[14px] font-semibold">{title}</p>
        <PinClose pinned={pinned} onPin={onPin} />
      </div>
      <div className="card-in space-y-3 px-4 pb-4 text-[13px] text-muted">
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
