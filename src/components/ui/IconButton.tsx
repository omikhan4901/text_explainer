import type { ButtonHTMLAttributes, ReactNode } from "react";

/** A small square button with an icon and an accessible name. */
export function IconButton({
  label,
  children,
  active = false,
  className = "",
  ...rest
}: { label: string; children: ReactNode; active?: boolean } & ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      aria-pressed={rest["aria-pressed"]}
      className={`grid size-7 place-items-center rounded-md transition-colors ${
        active ? "bg-accent-soft text-accent-soft-text" : "text-muted hover:bg-surface-2 hover:text-text"
      } ${className}`}
      {...rest}
    >
      {children}
    </button>
  );
}
