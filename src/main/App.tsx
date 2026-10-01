import { BookOpen, Cpu, House, Info, Keyboard, X } from "lucide-react";
import { useState, type ComponentType } from "react";
import { strings } from "../i18n";
import { DataProvider, useData } from "./data";
import { FirstRun } from "./FirstRun";
import { AboutPage } from "./pages/About";
import { HomePage } from "./pages/Home";
import { ModelPage } from "./pages/Model";
import { ReadingPage } from "./pages/Reading";
import { ShortcutsPage } from "./pages/Shortcuts";

export type PageId = "home" | "model" | "reading" | "shortcuts" | "about";

const PAGES: { id: PageId; icon: ComponentType<{ size?: number }>; label: string }[] = [
  { id: "home", icon: House, label: strings.nav.home },
  { id: "model", icon: Cpu, label: strings.nav.model },
  { id: "reading", icon: BookOpen, label: strings.nav.reading },
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
      <nav aria-label="Sections" className="sticky top-0 flex h-dvh w-24 shrink-0 flex-col items-center gap-1 border-r border-border bg-surface py-5">
        <div className="mb-4 grid size-11 place-items-center rounded-xl bg-accent text-on-accent" aria-hidden>
          <svg viewBox="0 0 24 24" className="size-6" fill="none">
            <rect x="5" y="6" width="12" height="2" rx="1" fill="currentColor" opacity="0.6" />
            <rect x="4" y="10.5" width="16" height="3.5" rx="1.2" fill="currentColor" />
            <rect x="5" y="16" width="9" height="2" rx="1" fill="currentColor" opacity="0.6" />
          </svg>
        </div>
        {PAGES.map(({ id, icon: Icon, label }) => (
          <button
            key={id}
            type="button"
            onClick={() => setPage(id)}
            aria-current={page === id ? "page" : undefined}
            className={`flex w-[76px] flex-col items-center gap-1 rounded-xl py-2.5 text-xs font-medium transition-colors ${
              page === id ? "bg-accent-soft text-accent-soft-text" : "text-muted hover:bg-surface-2 hover:text-text"
            }`}
          >
            <Icon size={20} />
            {label}
          </button>
        ))}
      </nav>
      <main className="min-w-0 flex-1 px-10 py-8">
        <div className="mx-auto max-w-3xl">
          <ErrorBanner />
          {page === "home" && <HomePage go={setPage} />}
          {page === "model" && <ModelPage />}
          {page === "reading" && <ReadingPage />}
          {page === "shortcuts" && <ShortcutsPage />}
          {page === "about" && <AboutPage />}
        </div>
      </main>
    </div>
  );
}

function ErrorBanner() {
  const { error, clearError } = useData();
  if (!error) return null;
  return (
    <div role="alert" className="mb-6 flex items-start gap-3 rounded-xl bg-danger-soft px-4 py-3 text-sm text-danger-text">
      <p className="flex-1">{error}</p>
      <button type="button" aria-label="Dismiss" onClick={clearError} className="shrink-0">
        <X size={16} />
      </button>
    </div>
  );
}

export function PageTitle({ children, lead }: { children: string; lead?: string }) {
  return (
    <header className="mb-6">
      <h1 className="text-3xl font-semibold">{children}</h1>
      {lead && <p className="mt-2 max-w-2xl text-muted">{lead}</p>}
    </header>
  );
}
