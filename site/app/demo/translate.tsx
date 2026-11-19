"use client";

import { useEffect, useRef, useState } from "react";
import { PlainlyCard } from "./card";
import { Stage, useScene, type SceneTimeline } from "./engine";
import { Cursor } from "./parts";
import { Reveal } from "../reveal";

const NAMES = ["বাংলা", "Español", "हिन्दी", "العربية", "日本語", "Français", "اردو", "Deutsch", "简体中文"];
const DRIFT_HALF = `${NAMES.join("   ")}   ${NAMES.join("   ")}   `;
const DRIFT = DRIFT_HALF + DRIFT_HALF;

const CLAUSE =
  "7.2 Notwithstanding any provision of this Agreement to the contrary, the Lessee shall remit payment of the monthly rent of $1,200 no later than the 5th day of each calendar month, failing which a late fee equivalent to 5% of the outstanding amount shall accrue.";
const PLAIN =
  "Whatever else this agreement says, the tenant (the Lessee) must pay the monthly rent of $1,200 by the 5th day of each month. If they don't, a late fee of 5% of the unpaid amount is added.";
const BANGLA =
  "এই চুক্তিতে যা-ই থাকুক না কেন, ভাড়াটিয়াকে প্রতি মাসের ৫ তারিখের মধ্যে মাসিক ভাড়া $1,200 পরিশোধ করতে হবে; না হলে বকেয়া অর্থের 5% বিলম্ব ফি যোগ হবে।";

const LABEL = "Plainly translating a lease clause into Bangla. The translation stays on your computer.";

export function TranslateSection() {
  const ref = useScene(buildTranslate, { loop: true, repeatDelay: 0.6 });
  return (
    <section className="relative overflow-hidden">
      <div aria-hidden="true" className="pointer-events-none absolute inset-0 flex flex-col justify-center gap-2">
        <div className="lang-row lang-a">{DRIFT}</div>
        <div className="lang-row lang-b">{DRIFT}</div>
        <div className="lang-row lang-c">{DRIFT}</div>
      </div>
      <div className="relative z-10 mx-auto grid max-w-6xl items-center gap-14 px-5 py-24 md:px-8 md:py-32 lg:grid-cols-[1.05fr_0.95fr] lg:gap-10">
        <Reveal>
          <h2 className="max-w-[12ch] text-4xl leading-[1.05] font-semibold tracking-[-0.03em] md:text-5xl">
            Translate it, <span className="mark sweep">privately.</span>
          </h2>
          <p className="mt-6 text-[22px] leading-snug">
            Into <IntoWord />
          </p>
          <p className="mt-5 max-w-[38ch] text-[18px] leading-relaxed text-muted">
            The same AI translates on your computer, so private text never leaves it. Point at a sentence to see its partner.
          </p>
        </Reveal>
        <Stage width={500} height={580} label={LABEL}>
          <div ref={ref} className="absolute inset-0">
            <PlainlyCard
              quote={CLAUSE}
              tab="explain"
              mode="translate"
              source={CLAUSE}
              answerLabel="Bangla"
              answer={BANGLA}
              lead={{ label: "Plain English", text: PLAIN }}
              level="plain"
              grade={{ from: 17, to: 9 }}
            />
            <Cursor kind="arrow" />
          </div>
        </Stage>
      </div>
      <style>{DRIFT_CSS}</style>
    </section>
  );
}

function IntoWord() {
  const ref = useRef<HTMLSpanElement>(null);
  const [index, setIndex] = useState(0);
  const [shown, setShown] = useState(true);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    let timer = 0;
    let fade = 0;
    const tick = () => {
      setShown(false);
      fade = window.setTimeout(() => {
        setIndex((n) => (n + 1) % NAMES.length);
        setShown(true);
      }, 180);
    };
    const io = new IntersectionObserver(
      (entries) => {
        const last = entries[entries.length - 1];
        if (last && last.isIntersecting) {
          if (!timer) timer = window.setInterval(tick, 1400);
        } else if (timer) {
          window.clearInterval(timer);
          timer = 0;
          window.clearTimeout(fade);
        }
      },
      { threshold: 0.3 },
    );
    io.observe(el);
    return () => {
      io.disconnect();
      window.clearInterval(timer);
      window.clearTimeout(fade);
    };
  }, []);

  const name = NAMES[index] ?? NAMES[0];
  return (
    <span ref={ref} className="inline-block min-w-[7ch] align-bottom">
      <bdi
        className="inline-block font-medium"
        style={{
          opacity: shown ? 1 : 0,
          transform: shown ? "translateY(0)" : "translateY(0.3em)",
          transition: "opacity 180ms linear, transform 180ms linear",
        }}
      >
        {name}
      </bdi>
    </span>
  );
}

function token(name: string, fallback: string) {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

function box(el: HTMLElement, root: HTMLElement) {
  const k = root.getBoundingClientRect().width / root.offsetWidth || 1;
  const a = el.getBoundingClientRect();
  const b = root.getBoundingClientRect();
  return { x: (a.left - b.left) / k, y: (a.top - b.top) / k, w: a.width / k, h: a.height / k };
}

function buildTranslate(tl: SceneTimeline, root: HTMLElement) {
  const card = root.querySelector<HTMLElement>("[data-part='card']");
  const lead = root.querySelector<HTMLElement>("[data-part='lead']");
  const translated = root.querySelector<HTMLElement>("[data-part='translated']");
  const explainBtn = root.querySelector<HTMLElement>("[data-part='mode-explain']");
  const translateBtn = root.querySelector<HTMLElement>("[data-part='mode-translate']");
  const compare = root.querySelector<HTMLElement>("[data-part='compare']");
  const gradeTo = root.querySelector<HTMLElement>("[data-part='grade-to']");
  const arrow = root.querySelector<HTMLElement>("[data-cursor='arrow']");
  const leadWords = [...root.querySelectorAll<HTMLElement>("[data-part='lead-answer'] [data-word]")];
  const bangla = [...root.querySelectorAll<HTMLElement>("[data-part='answer'] [data-word]")];
  if (!card || !lead || !translated || !explainBtn || !translateBtn || !compare || !arrow) return;

  const surface = token("--surface", "#ffffff");
  const text = token("--text", "#1b1b1a");
  const muted = token("--muted", "#62625d");
  const line = token("--line", "rgb(27 27 26 / 0.1)");
  const on = { backgroundColor: surface, color: text, boxShadow: `0 0 0 1px ${line}` };
  const off = { backgroundColor: "transparent", color: muted, boxShadow: "0 0 0 0px transparent" };

  tl.set(card, { opacity: 1 }, 0);
  tl.set(lead, { display: "block" }, 0);
  tl.set(translated, { display: "none" }, 0);
  tl.set(leadWords, { opacity: 1 }, 0);
  tl.set(bangla, { opacity: 0 }, 0);
  tl.set(explainBtn, on, 0);
  tl.set(translateBtn, off, 0);
  tl.set(compare, { opacity: 0, height: 0, marginTop: 0, overflow: "hidden" }, 0);
  if (gradeTo) tl.set(gradeTo, { opacity: 1 }, 0);

  const btn = box(translateBtn, root);
  tl.set(arrow, { x: btn.x + 90, y: btn.y + 80, opacity: 0, scale: 1, transformOrigin: "4px 3px" }, 0);
  tl.to(arrow, { opacity: 1, x: btn.x + btn.w * 0.55, y: btn.y + btn.h * 0.45, duration: 0.55, ease: "power3.inOut" }, 0.35);
  tl.to(arrow, { scale: 0.86, duration: 0.08, yoyo: true, repeat: 1, transformOrigin: "4px 3px" }, 0.95);

  const swap = 1.1;
  tl.set(explainBtn, off, swap);
  tl.set(translateBtn, on, swap);
  tl.set(lead, { display: "none" }, swap);
  tl.set(translated, { display: "block" }, swap);
  tl.to(arrow, { opacity: 0, duration: 0.25 }, swap + 0.15);
  const stagger = 0.02;
  tl.to(bangla, { opacity: 1, duration: 0.28, stagger, ease: "power2.out" }, swap);
  const stream = 0.28 + Math.max(0, bangla.length - 1) * stagger;
  const shown = swap + stream + 0.08;
  tl.set(compare, { height: "auto", marginTop: 12, overflow: "visible" }, shown);
  tl.to(compare, { opacity: 1, duration: 0.3, ease: "power2.out" }, shown);
  tl.set(compare, { opacity: 1 }, shown + 2.5);
}

const DRIFT_CSS = `
.lang-row {
  width: max-content;
  font-size: 120px;
  font-weight: 600;
  line-height: 1;
  letter-spacing: -0.03em;
  white-space: nowrap;
  color: #e9e9e6;
}
.lang-a { animation: plainly-drift-left 72s linear infinite; }
.lang-b { animation: plainly-drift-right 86s linear infinite; }
.lang-c { animation: plainly-drift-left 104s linear infinite; }
@keyframes plainly-drift-left {
  from { transform: translateX(0); }
  to { transform: translateX(-50%); }
}
@keyframes plainly-drift-right {
  from { transform: translateX(-50%); }
  to { transform: translateX(0); }
}
@media (prefers-color-scheme: dark) {
  .lang-row { color: #222220; }
}
@media (prefers-reduced-motion: reduce) {
  .lang-a, .lang-b, .lang-c { animation: none; }
}
`;
