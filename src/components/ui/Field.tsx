import type { ReactNode } from "react";

/** A setting row: label and hint on the left, the control on the right. */
export function Field({ label, hint, htmlFor, children }: { label: string; hint?: string; htmlFor?: string; children: ReactNode }) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-x-6 gap-y-2 px-4 py-3">
      <div className="min-w-0 flex-1">
        <label htmlFor={htmlFor} className="font-medium">
          {label}
        </label>
        {hint && <p className="mt-0.5 text-[13px] text-muted">{hint}</p>}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

export const inputClass =
  "h-8 rounded-md border border-border-strong bg-surface px-2.5 text-[13px] text-text transition-colors placeholder:text-muted hover:border-muted focus:border-accent focus:outline-none";
