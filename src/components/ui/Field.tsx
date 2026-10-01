import type { ReactNode } from "react";

/** A setting row: label and hint on the left, the control on the right. */
export function Field({ label, hint, htmlFor, children }: { label: string; hint?: string; htmlFor?: string; children: ReactNode }) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-x-6 gap-y-2 py-3">
      <div className="min-w-0 flex-1">
        <label htmlFor={htmlFor} className="font-medium">
          {label}
        </label>
        {hint && <p className="mt-0.5 text-sm text-muted">{hint}</p>}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

export const inputClass =
  "h-10 rounded-lg border border-border-strong bg-surface px-3 text-sm text-text placeholder:text-muted focus:border-accent focus:outline-none";
