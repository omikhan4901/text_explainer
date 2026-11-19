import type { ReactNode } from "react";
import { Icon } from "../icons";

/** Arrow or I-beam. Timelines move it with transforms only; the tip is the top left. */
export function Cursor({ kind }: { kind: "arrow" | "ibeam" }) {
  const arrow = kind === "arrow";
  return (
    <div data-cursor={kind} className="pointer-events-none absolute top-0 left-0 z-30 opacity-0" style={{ filter: "drop-shadow(0 1px 1px rgb(0 0 0 / 0.35))" }}>
      {arrow ? (
        <svg width="22" height="22" viewBox="0 0 24 24" aria-hidden="true">
          <path
            d="M5.2 2.4 5.2 17.6 9.2 13.8 12.7 20.4 15.4 19.1 11.8 12.7 17.6 12.6 Z"
            fill="#111111"
            stroke="#ffffff"
            strokeWidth="1.5"
            strokeLinejoin="round"
          />
        </svg>
      ) : (
        <svg width="16" height="26" viewBox="0 0 16 26" aria-hidden="true">
          <path d="M3 3.2H13M8 3.2V22.8M3 22.8H13" stroke="#ffffff" strokeWidth="4.2" strokeLinecap="round" />
          <path d="M3 3.2H13M8 3.2V22.8M3 22.8H13" stroke="#111111" strokeWidth="1.8" strokeLinecap="round" />
        </svg>
      )}
    </div>
  );
}

/** A row of keyboard keys. Pressing is a 5px drop and a thinner side. */
export function Keycaps({ keys }: { keys: string[] }) {
  return (
    <div className="flex items-end gap-3">
      {keys.map((key) => (
        <div
          key={key}
          data-key={key}
          className="grid h-16 min-w-[72px] place-items-center rounded-[14px] border border-line bg-surface px-4 text-[22px] font-medium text-text opacity-0 shadow-[0_6px_0_var(--key-lip)]"
        >
          {key}
        </div>
      ))}
    </div>
  );
}

/** A generic document window. The page stays white in dark mode; the chrome follows the theme. */
export function DocWindow({ title, children, className }: { title: string; children: ReactNode; className?: string }) {
  return (
    <div className={`overflow-hidden rounded-xl border border-line bg-sunken shadow-[var(--shadow)] ${className ?? ""}`}>
      <div className="flex h-[38px] items-center gap-2 border-b border-line bg-surface px-3 text-text">
        <Icon name="file" size={15} className="shrink-0 text-muted" />
        <span className="min-w-0 flex-1 truncate text-[13px]">{title}</span>
        <span className="flex items-center gap-[7px] text-muted" aria-hidden="true">
          <span className="block h-px w-2.5 bg-current" />
          <span className="block size-[9px] border border-current" />
          <span className="relative block size-2.5">
            <span className="absolute top-1/2 left-0 h-px w-full -translate-y-1/2 rotate-45 bg-current" />
            <span className="absolute top-1/2 left-0 h-px w-full -translate-y-1/2 -rotate-45 bg-current" />
          </span>
        </span>
      </div>
      <div className="h-[calc(100%-38px)] p-2">
        <div className="h-full overflow-hidden bg-white px-5 py-4 text-[#1b1b1a]">{children}</div>
      </div>
    </div>
  );
}
