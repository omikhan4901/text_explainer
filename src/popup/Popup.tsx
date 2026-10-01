import { AlertTriangle, Check, Copy, Pin, PinOff, X } from "lucide-react";
import { useCallback, useEffect, useLayoutEffect, useReducer, useRef, useState } from "react";
import { IconButton } from "../components/ui/IconButton";
import { Segmented } from "../components/ui/Segmented";
import { strings } from "../i18n";
import { api, inApp, on } from "../lib/ipc";
import { mock } from "../lib/mock";
import { applyAppearance, followSystemTheme } from "../lib/theme";
import type { Level, PopupEvent, Settings } from "../lib/types";
import { answerText, gradeBadge, initialState, isStreaming, reduce, type Explaining } from "./state";

const t = strings.card;
/** Transparent space around the card for its shadow (matches popup.rs MARGIN). */
const MARGIN = 12;

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
      const demo = new URLSearchParams(location.search).get("demo") ?? "passage";
      setTimeout(() => void mock.playDemo(demo), 50);
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

  return (
    <div className="p-3" style={{ width: 440 + 2 * MARGIN }}>
      <div
        ref={cardRef}
        role="dialog"
        aria-label={strings.app.name}
        className="flex max-h-[560px] flex-col overflow-hidden rounded-[var(--radius-card)] border border-border bg-surface text-text shadow-[var(--shadow-card)]"
      >
        {state.view === "explain" ? (
          <ExplainView
            state={state}
            level={settings?.level ?? "plain"}
            pinned={pinned}
            onPin={togglePin}
          />
        ) : (
          <MessageView state={state} />
        )}
      </div>
    </div>
  );
}

function Header({ children, pinned, onPin, copyText }: { children?: React.ReactNode; pinned?: boolean; onPin?: () => void; copyText?: string }) {
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    if (!copyText) return;
    await api.copyText(copyText);
    setCopied(true);
    setTimeout(() => setCopied(false), 1200);
  };
  return (
    <div className="flex h-11 shrink-0 items-center gap-2 border-b border-border px-3">
      <div className="flex min-w-0 flex-1 items-center gap-2">{children}</div>
      {onPin && (
        <IconButton label={pinned ? t.unpin : t.pin} active={pinned} aria-pressed={pinned} onClick={onPin}>
          {pinned ? <PinOff size={15} /> : <Pin size={15} />}
        </IconButton>
      )}
      {copyText !== undefined && (
        <IconButton label={copied ? t.copied : t.copy} onClick={copy} disabled={!copyText}>
          {copied ? <Check size={15} /> : <Copy size={15} />}
        </IconButton>
      )}
      <IconButton label={t.close} onClick={() => void api.popupClose()}>
        <X size={16} />
      </IconButton>
    </div>
  );
}

function ExplainView({ state, level, pinned, onPin }: { state: Explaining; level: Level; pinned: boolean; onPin: () => void }) {
  const text = answerText(state);
  const streaming = isStreaming(state);
  const badge = gradeBadge(state);
  const isTerm = state.kind === "word" || state.kind === "phrase";
  const report = state.report;
  const flagged = report && (report.missing.length > 0 || report.added.length > 0);

  return (
    <>
      <Header pinned={pinned} onPin={onPin} copyText={state.done && !state.error ? text : ""}>
        {isTerm ? (
          <span className="truncate font-semibold" title={state.source}>
            {state.source}
          </span>
        ) : (
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
        )}
        {badge && (
          <span title={t.gradeTitle} className="ml-auto shrink-0 rounded-full bg-accent-soft px-2 py-0.5 text-xs font-medium text-accent-soft-text">
            {t.grade(badge.before, badge.after)}
          </span>
        )}
      </Header>

      <div className="min-h-0 overflow-y-auto px-4 py-3" aria-live="polite" aria-busy={!state.done}>
        {isTerm && state.dictionary ? (
          <TermView state={state} text={text} streaming={streaming} />
        ) : state.error ? (
          <ErrorBlock message={state.error.message} detail={state.error.detail} />
        ) : text ? (
          <Reading text={text} streaming={streaming} />
        ) : state.loadingModel ? (
          <p className="text-sm text-muted">{t.loadingModel}</p>
        ) : (
          <Skeleton />
        )}
      </div>

      {(flagged || state.truncated) && !state.error && (
        <div className="space-y-2 border-t border-border px-4 py-2.5 text-xs">
          {flagged && report && (
            <div className="flex gap-2 rounded-lg bg-warn-soft px-2.5 py-2 text-warn-text" title={t.checkHint}>
              <AlertTriangle size={14} className="mt-px shrink-0" aria-hidden />
              <div>
                {report.missing.length > 0 && (
                  <p>
                    {t.checkMissing} <strong>{report.missing.map((f) => f.text).join(", ")}</strong>
                  </p>
                )}
                {report.added.length > 0 && (
                  <p>
                    {t.checkAdded} <strong>{report.added.map((f) => f.text).join(", ")}</strong>
                  </p>
                )}
              </div>
            </div>
          )}
          {state.truncated && <p className="text-muted">{t.truncated}</p>}
        </div>
      )}
    </>
  );
}

/** A word: its meaning here (from the model) above the dictionary senses. */
function TermView({ state, text, streaming }: { state: Explaining; text: string; streaming: boolean }) {
  const entry = state.dictionary!;
  // No Open → explain events at all means there's no model: dictionary only.
  const modelAnswering = state.started;
  return (
    <div className="space-y-4">
      {modelAnswering && (
        <section>
          <h3 className="mb-1 font-sans text-xs font-semibold tracking-wide text-muted uppercase">{t.inThisText}</h3>
          {state.error ? (
            <p className="text-sm text-danger-text">{state.error.message}</p>
          ) : text ? (
            <Reading text={text} streaming={streaming} />
          ) : (
            <Skeleton lines={1} />
          )}
        </section>
      )}
      <section>
        <h3 className="mb-1.5 font-sans text-xs font-semibold tracking-wide text-muted uppercase">{t.dictionary}</h3>
        <ol className="space-y-2 text-sm">
          {entry.senses.slice(0, 4).map((sense, i) => (
            <li key={i} className="leading-snug">
              <span className="mr-1.5 text-xs text-muted italic">{sense.pos}</span>
              {sense.definition}
              {sense.example && <span className="block text-muted">“{sense.example}”</span>}
              {sense.synonyms.length > 0 && (
                <span className="block text-xs text-muted">
                  {t.also} {sense.synonyms.join(", ")}
                </span>
              )}
            </li>
          ))}
        </ol>
      </section>
      {!modelAnswering && <p className="text-xs text-muted">{t.noModelForWords}</p>}
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
    <div className="space-y-2.5 py-1" aria-label={t.thinking}>
      {[92, 100, 76].slice(0, lines).map((w, i) => (
        <div key={i} className="h-2.5 animate-pulse rounded-full bg-surface-2" style={{ width: `${w}%` }} />
      ))}
    </div>
  );
}

function ErrorBlock({ message, detail }: { message: string; detail: string | null }) {
  return (
    <div className="space-y-2 text-sm">
      <p className="font-medium text-danger-text">{t.errorTitle}</p>
      <p className="text-muted first-letter:uppercase">{message}.</p>
      {detail && (
        <details className="text-xs text-muted">
          <summary className="cursor-pointer select-none">{t.details}</summary>
          <pre className="mt-1 max-h-32 overflow-auto rounded-lg bg-surface-2 p-2 whitespace-pre-wrap">{detail}</pre>
        </details>
      )}
      <button type="button" onClick={() => void api.openMain()} className="text-xs font-medium text-accent underline-offset-2 hover:underline">
        {t.settings}
      </button>
    </div>
  );
}

function MessageView({ state }: { state: Exclude<ReturnType<typeof reduce>, Explaining> }) {
  if (state.view === "empty") return <Header />;
  const [title, body, action] =
    state.view === "no_selection"
      ? [t.noSelectionTitle, t.noSelection(state.hotkey), null]
      : state.view === "no_model"
        ? [t.noModelTitle, t.noModel, t.setUp]
        : [t.captureFailed, state.message, null];
  return (
    <>
      <Header>
        <span className="font-semibold">{title}</span>
      </Header>
      <div className="space-y-3 px-4 py-3 text-sm text-muted">
        <p>{body}</p>
        {action && (
          <button
            type="button"
            onClick={() => void api.openMain()}
            className="h-9 rounded-lg bg-accent px-4 text-sm font-semibold text-on-accent hover:bg-accent-hover"
          >
            {action}
          </button>
        )}
      </div>
    </>
  );
}
