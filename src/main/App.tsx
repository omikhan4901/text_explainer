import { BookBookmark, Cpu, Globe, House, Info, Keyboard, TextAa, X, type Icon } from "@phosphor-icons/react";
import { useState } from "react";
import { Button } from "../components/ui/Button";
import { Kbd } from "../components/ui/Kbd";
import { strings } from "../i18n";
import { api } from "../lib/ipc";
import { DataProvider, useData } from "./data";
import { FirstRun } from "./FirstRun";
import { AboutPage } from "./pages/About";
import { HomePage } from "./pages/Home";
import { ModelPage } from "./pages/Model";
import { ReadingPage } from "./pages/Reading";
import { ShortcutsPage } from "./pages/Shortcuts";
import { SourcesPage } from "./pages/Sources";
import { WordsPage } from "./pages/Words";

export type PageId = "home" | "words" | "sources" | "model" | "reading" | "shortcuts" | "about";

const PAGES: { id: PageId; icon: Icon; label: string }[] = [
  { id: "home", icon: House, label: strings.nav.home },
  { id: "words", icon: BookBookmark, label: strings.nav.words },
  { id: "sources", icon: Globe, label: strings.nav.sources },
  { id: "reading", icon: TextAa, label: strings.nav.reading },
  { id: "model", icon: Cpu, label: strings.nav.model },
  { id: "shortcuts", icon: Keyboard, label: strings.nav.shortcuts },
  { id: "about", icon: Info, label: strings.nav.about },
];

export function App() {
  return (
    <DataProvider fallback={<div className="min-h-dvh bg-bg" />}>
      <Shell />
    </DataProvider>
  );
}

function Shell() {
  const { settings } = useData();
  const [page, setPage] = useState<PageId>(() => (new URLSearchParams(location.search).get("page") as PageId) || "home");
  if (!settings.first_run_done) return <FirstRun onSeeModels={() => setPage("model")} />;
  return (
    <div className="flex min-h-dvh bg-bg">
      <aside className="sticky top-0 flex h-dvh w-52 shrink-0 flex-col px-3 pt-4 pb-3">
        <div className="mb-5 flex items-center gap-2.5 px-2">
          <Mark />
          <span className="text-[13px] font-semibold">{strings.app.name}</span>
        </div>
        <nav aria-label={strings.nav.label} className="flex flex-col gap-px">
          {PAGES.map(({ id, icon: NavIcon, label }) => {
            const current = page === id;
            return (
              <button
                key={id}
                type="button"
                onClick={() => setPage(id)}
                aria-current={current ? "page" : undefined}
                className={`flex h-8 items-center gap-2.5 rounded-md px-2 text-[13px] transition-colors ${
                  current
                    ? "bg-surface font-medium text-text shadow-[0_0_0_1px_var(--border),0_1px_2px_rgb(0_0_0/0.04)]"
                    : "text-muted hover:bg-surface-2 hover:text-text"
                }`}
              >
                <NavIcon size={16} weight={current ? "fill" : "regular"} aria-hidden />
                {label}
              </button>
            );
          })}
        </nav>
        <Status />
      </aside>
      <main className="min-w-0 flex-1 px-10 pt-9 pb-12">
        <div className="max-w-[640px]">
          <ErrorBanner />
          {page === "home" && <HomePage go={setPage} />}
          {page === "words" && <WordsPage />}
          {page === "sources" && <SourcesPage />}
          {page === "model" && <ModelPage />}
          {page === "reading" && <ReadingPage />}
          {page === "shortcuts" && <ShortcutsPage />}
          {page === "about" && <AboutPage />}
        </div>
      </main>
    </div>
  );
}

/** The app's mark: a highlighted line between two plain ones. */
export function Mark({ size = 22 }: { size?: number }) {
  return (
    <span className="grid place-items-center rounded-[6px] bg-control text-on-control" style={{ width: size, height: size }} aria-hidden>
      <svg viewBox="0 0 16 16" width={size * 0.68} height={size * 0.68} fill="currentColor">
        <rect x="2" y="3" width="9" height="1.6" rx="0.8" opacity="0.55" />
        <rect x="2" y="7" width="12" height="2.4" rx="1" />
        <rect x="2" y="11.6" width="7" height="1.6" rx="0.8" opacity="0.55" />
      </svg>
    </span>
  );
}

/** Whether the shortcut works right now, and the switch to pause it. */
function Status() {
  const { settings, info, refresh } = useData();
  const t = strings.home;
  const ready = settings.model.kind !== "none";
  const state = !ready ? t.needsModel : info.paused ? t.paused : t.ready;
  const toggle = async () => {
    await api.setPaused(!info.paused);
    await refresh();
  };
  return (
    <div className="mt-auto rounded-lg border border-border bg-surface px-3 py-2.5">
      <div className="flex items-center justify-between gap-2">
        <span className="flex items-center gap-2 text-[13px] font-medium">
          <span className={`size-1.5 rounded-full ${ready && !info.paused ? "bg-success" : "bg-border-strong"}`} aria-hidden />
          {state}
        </span>
        {ready && (
          <Button size="sm" variant="ghost" className="-mr-1.5 h-6 px-2 text-[12px]" onClick={() => void toggle()}>
            {info.paused ? t.resume : t.pause}
          </Button>
        )}
      </div>
      {ready && !info.paused && (
        <div className="mt-1.5">
          <Kbd keys={settings.hotkey} />
        </div>
      )}
    </div>
  );
}

function ErrorBanner() {
  const { error, clearError } = useData();
  if (!error) return null;
  return (
    <div role="alert" className="mb-6 flex items-start gap-3 rounded-lg bg-danger-soft px-3.5 py-2.5 text-[13px] text-danger">
      <p className="flex-1">{error}</p>
      <button type="button" aria-label={strings.common.dismiss} onClick={clearError} className="shrink-0">
        <X size={14} />
      </button>
    </div>
  );
}

export function PageTitle({ children, lead }: { children: string; lead?: string }) {
  return (
    <header className="mb-8">
      <h1 className="text-[22px] font-semibold tracking-[-0.02em]">{children}</h1>
      {lead && <p className="mt-1.5 max-w-[56ch] text-[13px] text-muted">{lead}</p>}
    </header>
  );
}
