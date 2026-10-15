import { DownloadSimple, MagnifyingGlass, SpeakerHigh, Trash } from "@phosphor-icons/react";
import { save } from "@tauri-apps/plugin-dialog";
import { Tabs } from "radix-ui";
import { useEffect, useState, type ReactNode } from "react";
import { Button } from "../../components/ui/Button";
import { Field, inputClass } from "../../components/ui/Field";
import { IconButton } from "../../components/ui/IconButton";
import { Group, Section } from "../../components/ui/Section";
import { Switch } from "../../components/ui/Switch";
import { strings } from "../../i18n";
import { api, inApp } from "../../lib/ipc";
import type { ExportFormat, Lookup, SavedWord } from "../../lib/types";
import { relativeTime, shortDate, summary } from "../../lib/words";
import { PageTitle } from "../App";
import { useData } from "../data";

const t = strings.words;
const PAGE = 50;
const DAYS = [7, 30, 90, 180, 365];

/** The Word book (saved words) and the lookup history. */
export function WordsPage() {
  const [tab, setTab] = useState(() => (new URLSearchParams(location.search).get("tab") === "history" ? "history" : "saved"));
  return (
    <>
      <PageTitle lead={t.lead}>{t.title}</PageTitle>
      <Tabs.Root value={tab} onValueChange={setTab}>
        <Tabs.List aria-label={t.tabs} className="mb-7 flex gap-0.5 border-b border-border pb-2">
          {(["saved", "history"] as const).map((id) => (
            <Tabs.Trigger
              key={id}
              value={id}
              className="h-7 rounded-md px-2.5 text-[13px] font-medium text-muted transition-colors hover:bg-surface-2 hover:text-text data-[state=active]:bg-surface-2 data-[state=active]:text-text"
            >
              {t[id]}
            </Tabs.Trigger>
          ))}
        </Tabs.List>
        <Tabs.Content value="saved" className="focus-visible:outline-none">
          <SavedTab />
        </Tabs.Content>
        <Tabs.Content value="history" className="focus-visible:outline-none">
          <HistoryTab />
        </Tabs.Content>
      </Tabs.Root>
    </>
  );
}

/** Counts up when the window comes back to the front: the card may have changed the book. */
function useFocusVersion(): number {
  const [version, setVersion] = useState(0);
  useEffect(() => {
    const bump = () => setVersion((v) => v + 1);
    window.addEventListener("focus", bump);
    return () => window.removeEventListener("focus", bump);
  }, []);
  return version;
}

/** A list loaded a page at a time, reloaded when its query or the window focus changes. */
function usePaged<T extends { id: number }>(load: (limit: number, offset: number) => Promise<{ items: T[]; total: number }>, key: string) {
  const version = useFocusVersion();
  const [list, setList] = useState<{ key: string; items: T[]; total: number } | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    load(PAGE, 0)
      .then((p) => live && setList({ key, items: p.items, total: p.total }))
      .catch((e: unknown) => live && setError(String(e)));
    return () => {
      live = false;
    };
    // `load` changes with `key`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, version]);
  const current = list?.key === key ? list : null;
  const more = async () => {
    if (!current) return;
    try {
      const p = await load(PAGE, current.items.length);
      const seen = new Set(current.items.map((i) => i.id));
      setList({ key, items: [...current.items, ...p.items.filter((i) => !seen.has(i.id))], total: p.total });
    } catch (e) {
      setError(String(e));
    }
  };
  const drop = (id: number) => setList((l) => l && { ...l, items: l.items.filter((i) => i.id !== id), total: l.total - 1 });
  const clear = () => setList({ key, items: [], total: 0 });
  return { items: current?.items ?? null, total: current?.total ?? 0, error, setError, more, drop, clear };
}

function SavedTab() {
  const [search, setSearch] = useState("");
  const words = usePaged<SavedWord>((limit, offset) => api.listWords(search, limit, offset), search);
  const [notice, setNotice] = useState<string | null>(null);
  const empty = words.items !== null && words.total === 0 && !search.trim();

  const exportAs = async (format: ExportFormat) => {
    const csv = format === "csv";
    const name = csv ? t.csvName : t.ankiName;
    const path = inApp
      ? await save({ defaultPath: name, filters: [{ name: csv ? t.csvFilter : t.ankiFilter, extensions: csv ? ["csv"] : ["txt"] }] })
      : `C:\\Users\\Omi\\Documents\\${name}`;
    if (!path) return;
    try {
      const n = await api.exportWords(format, path);
      words.setError(null);
      setNotice(t.exported(n, path.split(/[\\/]/).pop() ?? path));
    } catch (e) {
      setNotice(null);
      words.setError(String(e));
    }
  };

  const remove = async (w: SavedWord) => {
    try {
      await api.removeWord(w.term);
      words.drop(w.id);
    } catch (e) {
      words.setError(String(e));
    }
  };

  return (
    <Section
      title={t.count(words.total)}
      aside={
        <div className="flex gap-2">
          <Button size="sm" disabled={empty} onClick={() => void exportAs("csv")}>
            <DownloadSimple size={14} aria-hidden /> {t.exportCsv}
          </Button>
          <Button size="sm" disabled={empty} onClick={() => void exportAs("anki")}>
            <DownloadSimple size={14} aria-hidden /> {t.exportAnki}
          </Button>
        </div>
      }
    >
      <Problem error={words.error} notice={notice} />
      <label className="relative mb-3 block">
        <span className="sr-only">{t.search}</span>
        <MagnifyingGlass size={14} className="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-muted" aria-hidden />
        <input
          type="search"
          className={`${inputClass} w-full pl-8`}
          placeholder={t.searchPlaceholder}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
      </label>
      {words.items === null ? null : words.items.length === 0 ? (
        <Empty>{search.trim() ? t.noMatch(search.trim()) : t.empty}</Empty>
      ) : (
        <Group>
          {words.items.map((w) => (
            <div key={w.id} className="flex items-center gap-3 px-4 py-2.5">
              <div className="min-w-0 flex-1">
                <p className="truncate font-medium">{w.term}</p>
                {summary(w) && <p className="truncate text-[13px] text-muted">{summary(w)}</p>}
              </div>
              <time dateTime={new Date(w.created_at).toISOString()} className="shrink-0 text-[12px] text-muted tabular-nums">
                {shortDate(w.created_at, t.locale)}
              </time>
              <div className="flex shrink-0">
                {w.kind !== "passage" && (
                  <IconButton label={t.listen(w.term)} onClick={() => void api.speak(w.term).catch((e: unknown) => words.setError(String(e)))}>
                    <SpeakerHigh size={15} />
                  </IconButton>
                )}
                <IconButton label={t.remove(w.term)} onClick={() => void remove(w)}>
                  <Trash size={15} />
                </IconButton>
              </div>
            </div>
          ))}
        </Group>
      )}
      <More shown={words.items?.length ?? 0} total={words.total} onMore={() => void words.more()} />
      {!empty && <p className="mt-3 max-w-[60ch] text-[12px] text-muted">{t.exportLead}</p>}
    </Section>
  );
}

function HistoryTab() {
  const { settings, update } = useData();
  const lookups = usePaged<Lookup>((limit, offset) => api.listHistory(limit, offset), "history");
  const [confirming, setConfirming] = useState(false);
  const [offerClear, setOfferClear] = useState(false);
  const [now] = useState(() => Date.now());
  const days = DAYS.includes(settings.history_days) ? DAYS : [...DAYS, settings.history_days].sort((a, b) => a - b);

  const clear = async () => {
    try {
      await api.clearHistory();
      lookups.clear();
    } catch (e) {
      lookups.setError(String(e));
    }
    setConfirming(false);
    setOfferClear(false);
  };
  const setKeep = async (keep_history: boolean) => {
    if (await update({ keep_history })) setOfferClear(!keep_history && lookups.total > 0);
  };

  return (
    <>
      <Section title={t.history}>
        <Group>
          <Switch label={t.keep} hint={t.keepHint} checked={settings.keep_history} onChange={(on) => void setKeep(on)} />
          {settings.keep_history && (
            <Field label={t.keepFor} hint={t.keepForHint} htmlFor="history-days">
              <select
                id="history-days"
                className={inputClass}
                value={settings.history_days}
                onChange={(e) => void update({ history_days: Number(e.target.value) })}
              >
                {days.map((d) => (
                  <option key={d} value={d}>
                    {t.days(d)}
                  </option>
                ))}
              </select>
            </Field>
          )}
        </Group>
        {offerClear && <Confirm text={t.turnedOff} yes={t.clear} no={t.keepIt} onYes={() => void clear()} onNo={() => setOfferClear(false)} />}
      </Section>

      <Section
        title={t.recent}
        aside={
          lookups.total > 0 &&
          !confirming && (
            <Button size="sm" variant="ghost" onClick={() => setConfirming(true)}>
              <Trash size={14} aria-hidden /> {t.clear}
            </Button>
          )
        }
      >
        <Problem error={lookups.error} notice={null} />
        {confirming && <Confirm text={t.clearConfirm} yes={t.clearYes} no={t.cancel} onYes={() => void clear()} onNo={() => setConfirming(false)} />}
        {lookups.items === null ? null : lookups.items.length === 0 ? (
          <Empty>{settings.keep_history ? t.historyEmpty : t.historyOff}</Empty>
        ) : (
          <Group>
            {lookups.items.map((l) => (
              <div key={l.id} className="flex items-center gap-3 px-4 py-2.5">
                <p className={`min-w-0 flex-1 truncate ${l.kind === "passage" ? "text-[13px]" : "font-medium"}`} title={l.query}>
                  {l.query}
                </p>
                <time dateTime={new Date(l.created_at).toISOString()} className="shrink-0 text-[12px] text-muted">
                  {relativeTime(l.created_at, now, t.locale, t.justNow)}
                </time>
              </div>
            ))}
          </Group>
        )}
        <More shown={lookups.items?.length ?? 0} total={lookups.total} onMore={() => void lookups.more()} />
      </Section>
    </>
  );
}

function Empty({ children }: { children: ReactNode }) {
  return <p className="rounded-lg bg-surface-2 px-3.5 py-2.5 text-[13px] text-muted">{children}</p>;
}

function More({ shown, total, onMore }: { shown: number; total: number; onMore: () => void }) {
  if (shown >= total) return null;
  return (
    <Button size="sm" variant="ghost" className="mt-2" onClick={onMore}>
      {t.more}
    </Button>
  );
}

function Problem({ error, notice }: { error: string | null; notice: string | null }) {
  if (error) {
    return (
      <p role="alert" className="mb-3 rounded-lg bg-danger-soft px-3.5 py-2.5 text-[13px] text-danger first-letter:uppercase">
        {error}
      </p>
    );
  }
  return (
    <p role="status" className={notice ? "mb-3 text-[13px] text-muted" : "sr-only"}>
      {notice}
    </p>
  );
}

/** An inline "are you sure" with the safe choice focused. */
function Confirm({ text, yes, no, onYes, onNo }: { text: string; yes: string; no: string; onYes: () => void; onNo: () => void }) {
  return (
    <div className="mt-3 mb-3 flex flex-wrap items-center justify-between gap-3 rounded-lg bg-surface-2 px-3.5 py-2.5 text-[13px]">
      <p>{text}</p>
      <div className="flex gap-2">
        <Button size="sm" variant="danger" onClick={onYes}>
          {yes}
        </Button>
        <Button size="sm" variant="ghost" onClick={onNo} autoFocus>
          {no}
        </Button>
      </div>
    </div>
  );
}
