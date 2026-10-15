import { MagnifyingGlass } from "@phosphor-icons/react";
import { useEffect, useRef, useState } from "react";
import { Kbd } from "../components/ui/Kbd";
import { strings } from "../i18n";
import { api, on } from "../lib/ipc";
import { applyAppearance, followSystemTheme } from "../lib/theme";
import type { Appearance } from "../lib/types";

const t = strings.lookup;
/** The box's size and the transparent space around it for its shadow (match lookup.rs). */
const WIDTH = 560;
const HEIGHT = 96;
const MARGIN = 12;
/** Longest text sent (`MAX_QUERY_CHARS` in sources.rs). */
const MAX_CHARS = 1500;

/** The Look up box: type a word or phrase, Enter opens the card on it, Esc hides the box. */
export function Lookup() {
  const [text, setText] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  const input = useRef<HTMLInputElement>(null);
  const appearance = useRef<Appearance | undefined>(undefined);

  useEffect(() => {
    const load = () =>
      void api.getSettings().then((s) => {
        appearance.current = s.appearance;
        applyAppearance(s.appearance);
      });
    load();
    const stopTheme = followSystemTheme(() => appearance.current);
    // The app shows the box again: start empty, with the current look.
    const off = on<null>("te://lookup", () => {
      setText("");
      setProblem(null);
      load();
      input.current?.focus();
    });
    const focus = () => input.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.preventDefault();
      setText("");
      setProblem(null);
      void api.lookupHide();
    };
    window.addEventListener("focus", focus);
    window.addEventListener("keydown", onKey);
    return () => {
      stopTheme();
      window.removeEventListener("focus", focus);
      window.removeEventListener("keydown", onKey);
      void off.then((f) => f());
    };
  }, []);

  const submit = async () => {
    if (!text.trim()) return setProblem(t.empty);
    try {
      await api.lookUp(text);
      setText("");
      setProblem(null);
    } catch (e) {
      setProblem(String(e));
    }
  };

  return (
    <main className="p-3" style={{ width: WIDTH + 2 * MARGIN }}>
      <form
        role="search"
        aria-label={t.title}
        onSubmit={(e) => {
          e.preventDefault();
          void submit();
        }}
        className="card-in flex flex-col overflow-hidden rounded-xl bg-surface text-text shadow-[var(--shadow-card)]"
        style={{ height: HEIGHT }}
      >
        <div className="flex flex-1 items-center gap-3 px-4">
          <MagnifyingGlass size={18} className="shrink-0 text-muted" aria-hidden />
          <input
            ref={input}
            autoFocus
            value={text}
            onChange={(e) => {
              setText(e.target.value);
              setProblem(null);
            }}
            maxLength={MAX_CHARS}
            aria-label={t.placeholder}
            placeholder={t.placeholder}
            aria-invalid={problem !== null}
            aria-describedby="lookup-problem"
            enterKeyHint="search"
            autoComplete="off"
            spellCheck={false}
            className="h-full min-w-0 flex-1 bg-transparent text-[17px] text-text placeholder:text-muted focus:outline-none"
          />
        </div>
        <div className="flex h-9 shrink-0 items-center justify-between gap-3 border-t border-border px-4 text-[12px] text-muted">
          <p id="lookup-problem" role={problem ? "alert" : undefined} className={`min-w-0 truncate ${problem ? "text-danger" : ""}`}>
            {problem}
          </p>
          <p className="flex shrink-0 items-center gap-3">
            <span className="flex items-center gap-1.5">
              <Kbd keys="Enter" /> {t.submit}
            </span>
            <span className="flex items-center gap-1.5">
              <Kbd keys="Esc" /> {t.close}
            </span>
          </p>
        </div>
      </form>
    </main>
  );
}
