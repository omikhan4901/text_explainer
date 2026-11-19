"use client";

import { useEffect, useState } from "react";
import { replayScene, useScene, type SceneTimeline } from "./engine";
import { Reveal } from "../reveal";

const SERIF = 'Georgia, "Iowan Old Style", Palatino, "Palatino Linotype", serif';
const CLAUSE =
  "7.2 Notwithstanding any provision of this Agreement to the contrary, the Lessee shall remit payment of the monthly rent of $1,200 no later than the 5th day of each calendar month, failing which a late fee equivalent to 5% of the outstanding amount shall accrue.";
const WARNING_ICON =
  "M236.8,188.09,149.35,36.22h0a24.76,24.76,0,0,0-42.7,0L19.2,188.09a23.51,23.51,0,0,0,0,23.72A24.35,24.35,0,0,0,40.55,224h174.9a24.35,24.35,0,0,0,21.33-12.19A23.51,23.51,0,0,0,236.8,188.09ZM222.93,203.8a8.5,8.5,0,0,1-7.48,4.2H40.55a8.5,8.5,0,0,1-7.48-4.2,7.59,7.59,0,0,1,0-7.72L120.52,44.21a8.75,8.75,0,0,1,15,0l87.45,151.87A7.59,7.59,0,0,1,222.93,203.8ZM120,144V104a8,8,0,0,1,16,0v40a8,8,0,0,1-16,0Zm20,36a12,12,0,1,1-12-12A12,12,0,0,1,140,180Z";

function ClauseText({ text }: { text: string }) {
  const parts = text.split(/(\$1,200|5th|5%)/g);
  return parts.map((part, i) => {
    if (part === "$1,200" || part === "5th") {
      const name = part === "$1,200" ? "rent" : "day";
      return (
        <span key={i} data-fact={name} className="mark mark-full" style={{ backgroundSize: "0% 100%" }}>
          {part}
        </span>
      );
    }
    if (part === "5%") {
      return (
        <span key={i} data-fact="fee" className="rounded-[3px]">
          {part}
        </span>
      );
    }
    return <span key={i}>{part}</span>;
  });
}

function box(el: HTMLElement, origin: HTMLElement) {
  const a = el.getBoundingClientRect();
  const b = origin.getBoundingClientRect();
  return { left: a.left - b.left, right: a.right - b.left, cy: a.top - b.top + a.height / 2 };
}

function curve(x1: number, y1: number, x2: number, y2: number) {
  const mid = (x2 - x1) * 0.5;
  return `M ${x1.toFixed(1)} ${y1.toFixed(1)} C ${(x1 + mid).toFixed(1)} ${y1.toFixed(1)}, ${(x2 - mid).toFixed(1)} ${y2.toFixed(1)}, ${x2.toFixed(1)} ${y2.toFixed(1)}`;
}

function facts(root: HTMLElement, name: string) {
  return [...root.querySelectorAll<HTMLElement>(`[data-fact="${name}"]`)];
}

function buildCheck(tl: SceneTimeline, root: HTMLElement) {
  const field = root.querySelector<HTMLElement>("[data-field]");
  const left = root.querySelector<HTMLElement>("[data-panel='source']");
  const right = root.querySelector<HTMLElement>("[data-panel='plain']");
  const warn = root.querySelector<HTMLElement>("[data-warn]");
  const badge = root.querySelector<HTMLElement>("[data-badge]");
  if (!field || !left || !right || !warn) return;

  const rent = facts(root, "rent");
  const day = facts(root, "day");
  const fee = facts(root, "fee");
  const dark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  const wide = window.matchMedia("(min-width: 768px)").matches;

  const mark = (els: HTMLElement[], at: number) => {
    if (!els.length) return;
    tl.fromTo(els, { backgroundSize: "0% 100%" }, { backgroundSize: "100% 100%", duration: 0.4, ease: "power2.out" }, at);
  };
  const showWarn = (at: number) => {
    tl.fromTo(warn, { opacity: 0, y: 16 }, { opacity: 1, y: 0, duration: 0.45, ease: "power3.out" }, at);
  };
  const paintFee = (at: number) => {
    if (!fee.length) return;
    tl.to(
      fee,
      { backgroundColor: dark ? "#2e2516" : "#fbf1dc", color: dark ? "#e6b35c" : "#8a5300", duration: 0.35 },
      at,
    );
  };

  if (!wide) {
    mark(rent, 0.15);
    mark(day, 0.55);
    paintFee(0.95);
    showWarn(1.35);
    return;
  }

  const marker = root.querySelector<HTMLElement>("[data-fact='marker']");
  const rentPath = root.querySelector<SVGPathElement>("[data-wire='rent']");
  const dayPath = root.querySelector<SVGPathElement>("[data-wire='day']");
  const feePath = root.querySelector<SVGPathElement>("[data-wire='fee']");
  if (!marker || !rentPath || !dayPath || !feePath || !badge) {
    mark(rent, 0.15);
    mark(day, 0.55);
    paintFee(0.95);
    showWarn(1.35);
    return;
  }

  const panelL = box(left, field);
  const panelR = box(right, field);
  const rentL = rent[0] ? box(rent[0], field) : null;
  const rentR = rent[1] ? box(rent[1], field) : null;
  const dayL = day[0] ? box(day[0], field) : null;
  const dayR = day[1] ? box(day[1], field) : null;
  const feeL = fee[0] ? box(fee[0], field) : null;
  const miss = box(marker, field);

  const draw = (path: SVGPathElement, d: string, at: number, dur: number) => {
    path.setAttribute("d", d);
    const len = path.getTotalLength() || 1;
    path.style.strokeDasharray = `${len}`;
    tl.fromTo(path, { strokeDashoffset: len }, { strokeDashoffset: 0, duration: dur, ease: "power2.inOut" }, at);
    return len;
  };
  const dot = (name: string, x: number, y: number, at: number) => {
    const el = root.querySelector<SVGCircleElement>(`[data-dot="${name}"]`);
    if (!el) return;
    el.setAttribute("cx", x.toFixed(1));
    el.setAttribute("cy", y.toFixed(1));
    tl.fromTo(el, { opacity: 0 }, { opacity: 1, duration: 0.15 }, at);
  };

  if (rentL && rentR) {
    const x1 = panelL.right + 6;
    const x2 = panelR.left - 6;
    draw(rentPath, curve(x1, rentL.cy, x2, rentR.cy), 0.15, 0.6);
    dot("rent-a", x1, rentL.cy, 0.15);
    dot("rent-b", x2, rentR.cy, 0.6);
    mark(rent, 0.15);
  }
  if (dayL && dayR) {
    const x1 = panelL.right + 6;
    const x2 = panelR.left - 6;
    draw(dayPath, curve(x1, dayL.cy, x2, dayR.cy), 0.85, 0.6);
    dot("day-a", x1, dayL.cy, 0.85);
    dot("day-b", x2, dayR.cy, 1.3);
    mark(day, 0.85);
  }
  if (feeL) {
    const x1 = panelL.right + 6;
    const x2 = panelR.left - 28;
    const y2 = miss.cy;
    draw(feePath, curve(x1, feeL.cy, x2, y2), 1.55, 0.5);
    paintFee(1.55);
    badge.style.left = `${x2 - 11}px`;
    badge.style.top = `${y2 - 11}px`;
    tl.fromTo(badge, { scale: 0, opacity: 0 }, { scale: 1, opacity: 1, duration: 0.45, ease: "back.out(3)", transformOrigin: "center center" }, 1.95);
    tl.fromTo(right, { x: 0 }, { x: 3, duration: 0.05, ease: "power1.inOut", yoyo: true, repeat: 5 }, 1.95);
  }
  showWarn(2.4);
}

export function CheckSection() {
  const [revision, setRevision] = useState(0);
  const ref = useScene(buildCheck, { once: true, threshold: 0.4, revision });

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    let width = el.clientWidth;
    let timer = 0;
    const observer = new ResizeObserver(() => {
      const next = el.clientWidth;
      if (!next || Math.abs(next - width) < 1) return;
      width = next;
      window.clearTimeout(timer);
      timer = window.setTimeout(() => setRevision((n) => n + 1), 150);
    });
    observer.observe(el);
    return () => {
      observer.disconnect();
      window.clearTimeout(timer);
    };
  }, [ref]);

  return (
    <section className="border-t border-line bg-surface">
      <style>{`
        .fact-warn-row { background: #fbf1dc; color: #8a5300; }
        @media (prefers-color-scheme: dark) {
          .fact-warn-row { background: #2e2516; color: #e6b35c; }
        }
      `}</style>
      <div className="mx-auto max-w-6xl px-5 py-24 md:px-8 md:py-32">
        <Reveal>
          <h2 className="max-w-[14ch] text-4xl leading-[1.05] font-semibold tracking-[-0.03em] md:text-5xl">
            It checks <span className="mark sweep">its own work.</span>
          </h2>
        </Reveal>
        <p className="mt-6 max-w-[46ch] text-[18px] leading-relaxed text-muted">
          If a number, date or name goes missing or appears from nowhere, the card says so.
        </p>

        <div ref={ref} className="mt-14">
          <div data-field="" className="relative">
            <svg className="pointer-events-none absolute inset-0 z-0 hidden h-full w-full overflow-visible md:block" aria-hidden="true">
              <path data-wire="rent" fill="none" stroke="#c98f3c" strokeWidth="3" strokeLinecap="round" />
              <path data-wire="day" fill="none" stroke="#c98f3c" strokeWidth="3" strokeLinecap="round" />
              <path data-wire="fee" fill="none" stroke="#8a5300" strokeWidth="3" strokeLinecap="round" />
              <circle data-dot="rent-a" r="4.5" fill="#c98f3c" opacity="0" />
              <circle data-dot="rent-b" r="4.5" fill="#c98f3c" opacity="0" />
              <circle data-dot="day-a" r="4.5" fill="#c98f3c" opacity="0" />
              <circle data-dot="day-b" r="4.5" fill="#c98f3c" opacity="0" />
            </svg>
            <span
              data-badge=""
              className="pointer-events-none absolute z-20 hidden size-[22px] place-items-center rounded-full text-[13px] font-bold text-white md:grid"
              style={{ background: "#8a5300", opacity: 0, left: 0, top: 0 }}
              aria-hidden="true"
            >
              !
            </span>
            <div className="relative z-10 grid grid-cols-1 gap-8 md:grid-cols-2 md:gap-[120px]">
              <div data-panel="source" className="rounded-2xl bg-bg p-7 shadow-[0_0_0_1px_var(--line)]">
                <div className="mb-4 font-mono text-[12px] tracking-[0.14em] text-muted uppercase">What you selected</div>
                <p className="text-[19px] leading-[1.65]" style={{ fontFamily: SERIF }}>
                  <ClauseText text={CLAUSE} />
                </p>
              </div>
              <div data-panel="plain" className="rounded-2xl bg-bg p-7 shadow-[0_0_0_1px_var(--line)]">
                <div className="mb-4 font-mono text-[12px] tracking-[0.14em] text-muted uppercase">Plain English</div>
                <p className="text-[20px] leading-[1.6]">
                  {"Whatever else this agreement says, the tenant (the Lessee) must pay the monthly rent of "}
                  <span data-fact="rent" className="mark mark-full" style={{ backgroundSize: "0% 100%" }}>
                    $1,200
                  </span>
                  {" by the "}
                  <span data-fact="day" className="mark mark-full" style={{ backgroundSize: "0% 100%" }}>
                    5th
                  </span>
                  {" day of each month. If they don't, a late fee"}
                  <span data-fact="marker" className="inline-block h-[1em] w-0" aria-hidden="true" />
                  {" is added."}
                </p>
              </div>
            </div>
          </div>

          <div data-warn="" className="fact-warn-row mt-6 flex items-center gap-3 rounded-xl px-4 py-3.5 text-[15px] opacity-0">
            <svg viewBox="0 0 256 256" width="22" height="22" aria-hidden="true" className="shrink-0">
              <path fill="currentColor" d={WARNING_ICON} />
            </svg>
            <p>
              Not found in the rewrite: <strong className="font-semibold">5%</strong>
            </p>
          </div>
          <button
            type="button"
            className="press mt-6 text-[14px] font-medium text-muted underline decoration-line underline-offset-4 hover:text-text"
            onClick={() => replayScene(ref.current)}
          >
            Replay
          </button>
        </div>
      </div>
    </section>
  );
}
