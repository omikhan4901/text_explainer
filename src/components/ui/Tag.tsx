import type { ReactNode } from "react";

type Tone = "neutral" | "accent" | "success" | "warn" | "danger";

const tones: Record<Tone, string> = {
  neutral: "bg-surface-2 text-muted",
  accent: "bg-accent-soft text-accent-soft-text",
  success: "bg-success-soft text-success-text",
  warn: "bg-warn-soft text-warn-text",
  danger: "bg-danger-soft text-danger-text",
};

export function Tag({ tone = "neutral", children }: { tone?: Tone; children: ReactNode }) {
  return <span className={`inline-flex items-center rounded-md px-2 py-0.5 text-xs font-semibold ${tones[tone]}`}>{children}</span>;
}
