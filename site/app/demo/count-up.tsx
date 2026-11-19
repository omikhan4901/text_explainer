"use client";

import { useLayoutEffect, useRef } from "react";
import gsap from "gsap";

type Piece = { kind: "text"; text: string } | { kind: "num"; value: number; decimals: number };

/** Split a display string into literal text and the numbers that count. */
function pieces(value: string): Piece[] {
  const out: Piece[] = [];
  const re = /\d+(?:\.\d+)?/g;
  let last = 0;
  let match: RegExpExecArray | null;
  while ((match = re.exec(value))) {
    if (match.index > last) out.push({ kind: "text", text: value.slice(last, match.index) });
    const raw = match[0];
    const dot = raw.indexOf(".");
    out.push({ kind: "num", value: Number(raw), decimals: dot === -1 ? 0 : raw.length - dot - 1 });
    last = match.index + raw.length;
  }
  if (last < value.length) out.push({ kind: "text", text: value.slice(last) });
  return out;
}

function render(parts: Piece[], t: number, exact: string) {
  if (t >= 1) return exact;
  return parts
    .map((part) => (part.kind === "text" ? part.text : (part.value * t).toFixed(part.decimals)))
    .join("");
}

/** Counts each number in `value` from 0 once it scrolls into view. Units stay put. */
export function CountValue({ value, className }: { value: string; className?: string }) {
  const ref = useRef<HTMLSpanElement>(null);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (reduce) {
      el.textContent = value;
      return;
    }
    const parts = pieces(value);
    const apply = (t: number) => {
      el.textContent = render(parts, t, value);
    };
    apply(0);
    const proxy = { t: 0 };
    const tween = gsap.to(proxy, {
      t: 1,
      duration: 1.15,
      ease: "power2.out",
      paused: true,
      onUpdate: () => apply(proxy.t),
      onComplete: () => {
        el.textContent = value;
      },
    });
    const io = new IntersectionObserver(
      (entries) => {
        const last = entries[entries.length - 1];
        if (last && last.intersectionRatio >= 0.4) {
          tween.play();
          io.disconnect();
        }
      },
      { threshold: [0, 0.4, 1] },
    );
    io.observe(el);
    return () => {
      io.disconnect();
      tween.kill();
    };
  }, [value]);

  return (
    <span ref={ref} className={className}>
      {value}
    </span>
  );
}
