import type { HTMLAttributes, ReactNode } from "react";

/** A titled block of a settings page: heading, an optional line under it, then content. */
export function Section({
  title,
  description,
  aside,
  children,
  className = "",
}: {
  title: string;
  description?: ReactNode;
  aside?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={`mt-9 first:mt-0 ${className}`}>
      <div className="mb-3 flex items-end justify-between gap-4">
        <div className="min-w-0">
          <h2 className="text-[15px] font-semibold">{title}</h2>
          {description && <p className="mt-0.5 max-w-[60ch] text-[13px] text-muted">{description}</p>}
        </div>
        {aside && <div className="shrink-0">{aside}</div>}
      </div>
      {children}
    </section>
  );
}

/** A bordered list of rows, separated by hairlines. */
export function Group({ className = "", ...rest }: HTMLAttributes<HTMLDivElement>) {
  return <div className={`divide-y divide-border rounded-xl border border-border bg-surface ${className}`} {...rest} />;
}
