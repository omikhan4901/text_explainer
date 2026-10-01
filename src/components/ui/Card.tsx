import type { HTMLAttributes, ReactNode } from "react";

/** A bordered Atlas card. */
export function Card({ className = "", ...rest }: HTMLAttributes<HTMLDivElement>) {
  return <div className={`rounded-[var(--radius-card)] border border-border bg-surface p-6 ${className}`} {...rest} />;
}

export function CardTitle({ children, aside }: { children: ReactNode; aside?: ReactNode }) {
  return (
    <div className="mb-4 flex items-center justify-between gap-4">
      <h2 className="text-lg font-semibold">{children}</h2>
      {aside}
    </div>
  );
}
