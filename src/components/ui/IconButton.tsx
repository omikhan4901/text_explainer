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
      className={`grid size-7 place-items-center rounded-md transition-colors disabled:opacity-40 ${
        active ? "bg-surface-2 text-text" : "text-muted hover:bg-surface-2 hover:text-text"
      } ${className}`}
      {...rest}
    >
      {children}
    </button>
  );
}
