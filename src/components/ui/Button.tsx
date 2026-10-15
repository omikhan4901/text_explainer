import type { ButtonHTMLAttributes } from "react";

type Variant = "primary" | "secondary" | "ghost" | "danger";

const styles: Record<Variant, string> = {
  primary: "bg-control text-on-control hover:bg-control-hover",
  secondary: "border border-border-strong bg-surface text-text hover:bg-surface-2",
  ghost: "text-muted hover:bg-surface-2 hover:text-text",
  danger: "border border-border-strong bg-surface text-danger hover:bg-danger-soft",
};

export function Button({
  variant = "secondary",
  size = "md",
  className = "",
  ...rest
}: { variant?: Variant; size?: "sm" | "md" } & ButtonHTMLAttributes<HTMLButtonElement>) {
  const sizing = size === "sm" ? "h-7 px-2.5 text-[13px]" : "h-8 px-3.5 text-[13px]";
  return (
    <button
      type="button"
      className={`inline-flex items-center justify-center gap-1.5 rounded-md font-medium whitespace-nowrap transition-[background-color,color,transform] duration-150 active:scale-[0.98] disabled:pointer-events-none disabled:opacity-45 ${sizing} ${styles[variant]} ${className}`}
      {...rest}
    />
  );
}

/** A text button that reads like a link. */
export function LinkButton({ className = "", ...rest }: ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button
      type="button"
      className={`font-medium text-accent underline decoration-current/30 underline-offset-[3px] transition-colors hover:decoration-current ${className}`}
      {...rest}
    />
  );
}
