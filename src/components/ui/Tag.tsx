import type { ReactNode } from "react";

type Tone = "neutral" | "accent" | "success" | "warn" | "danger";

const tones: Record<Tone, string> = {
  neutral: "bg-surface-2 text-muted",
  accent: "bg-accent-soft text-text",
  success: "bg-success-soft text-success",
  warn: "bg-warn-soft text-warn",
  danger: "bg-danger-soft text-danger",
};

/** A small square label for a fact about something (a model's tier, whether it fits). */
export function Tag({ tone = "neutral", children }: { tone?: Tone; children: ReactNode }) {
  return <span className={`inline-flex h-5 items-center gap-1 rounded px-1.5 text-[11px] font-medium ${tones[tone]}`}>{children}</span>;
}
