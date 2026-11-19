"use client";

import { PlainlyCard } from "./card";
import { Stage, useScene, type SceneTimeline } from "./engine";
import { Cursor, DocWindow, Keycaps } from "./parts";

const SERIF = 'Georgia, "Iowan Old Style", Palatino, "Palatino Linotype", serif';
const CLAUSE =
  "7.2 Notwithstanding any provision of this Agreement to the contrary, the Lessee shall remit payment of the monthly rent of $1,200 no later than the 5th day of each calendar month, failing which a late fee equivalent to 5% of the outstanding amount shall accrue.";
const ANSWER =
  "Whatever else this agreement says, the tenant (the Lessee) must pay the monthly rent of $1,200 by the 5th day of each month. If they don't, a late fee of 5% of the unpaid amount is added.";
const LABEL =
  "Plainly explaining a lease clause: text is selected in a PDF, Ctrl+Shift+Space opens the Plainly card beside it, and Explain with AI rewrites it in plain English, grade 17 to 9.";

export function HeroDemo() {
  const ref = useScene(buildHero, { loop: true, repeatDelay: 0.6 });
  return (
    <Stage width={620} height={560} label={LABEL}>
      <div ref={ref} className="relative h-full w-full">
        <DocWindow title="Lease Agreement.pdf" className="absolute top-10 left-0 z-0 h-[460px] w-[470px]">
          <p className="mb-3 text-[15px] leading-snug font-semibold" style={{ fontFamily: SERIF }}>
            7. Rent and payment
          </p>
          <p className="text-[14px] leading-[1.6] text-[#6f6f69]" style={{ fontFamily: SERIF }}>
            7.1 The term of this Lease shall commence on the Commencement Date and continue for twelve (12) months thereafter.
          </p>
          <p data-clause="hot" className="mt-3 text-[14px] leading-[1.6]" style={{ fontFamily: SERIF }}>
            <Clause words={CLAUSE} />
          </p>
          <p className="mt-3 text-[14px] leading-[1.6] text-[#6f6f69]" style={{ fontFamily: SERIF }}>
            7.3 All payments shall be made by electronic transfer to the account designated by the Lessor in writing.
          </p>
        </DocWindow>

        <div data-scale="0.86" className="absolute top-0 left-[200px] z-10 origin-top-left" style={{ transform: "scale(0.86)" }}>
          <PlainlyCard
            className="origin-top-left"
            quote={CLAUSE}
            tab="google"
            mode="explain"
            source={CLAUSE}
            answerLabel="Plain English"
            answer={ANSWER}
            level="plain"
            grade={{ from: 17, to: 9 }}
          />
        </div>

        <div className="absolute inset-x-0 z-20 flex justify-center" style={{ top: 470 }}>
          <Keycaps keys={["Ctrl", "Shift", "Space"]} />
        </div>
        <Cursor kind="ibeam" />
        <Cursor kind="arrow" />
      </div>
    </Stage>
  );
}

function Clause({ words }: { words: string }) {
  const list = words.split(/\s+/).filter(Boolean);
  return list.map((word, i) => (
    <span key={`${word}-${i}`} data-word="">
      <span data-hit="" className="relative inline-block rounded-[2px]">
        <span data-sel="" className="absolute inset-0 rounded-[2px] bg-[var(--mark)] opacity-0" />
        <span className="relative">{word}</span>
      </span>
      {i < list.length - 1 ? " " : ""}
    </span>
  ));
}

/** Layout position in the stage, in design pixels. Ancestor scales (the card's 0.86) are applied. */
function stageOffset(el: HTMLElement, stage: HTMLElement) {
  const chain: HTMLElement[] = [];
  let n: HTMLElement | null = el;
  while (n && n !== stage) {
    chain.push(n);
    n = n.offsetParent as HTMLElement | null;
  }
  if (n !== stage) {
    const k = stage.getBoundingClientRect().width / stage.offsetWidth || 1;
    const a = el.getBoundingClientRect();
    const b = stage.getBoundingClientRect();
    return { x: (a.left - b.left) / k, y: (a.top - b.top) / k, w: a.width / k, h: a.height / k };
  }
  let x = 0;
  let y = 0;
  let scale = 1;
  for (const node of chain.reverse()) {
    x += node.offsetLeft * scale;
    y += node.offsetTop * scale;
    const s = Number(node.dataset.scale);
    if (s) scale *= s;
  }
  return { x, y, w: el.offsetWidth * scale, h: el.offsetHeight * scale };
}

function token(name: string, fallback: string) {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

function buildHero(tl: SceneTimeline, root: HTMLElement) {
  const card = root.querySelector<HTMLElement>("[data-part='card']");
  const web = root.querySelector<HTMLElement>("[data-part='web']");
  const explain = root.querySelector<HTMLElement>("[data-part='explain']");
  const googleTab = root.querySelector<HTMLElement>("[data-part='google-tab']");
  const explainTab = root.querySelector<HTMLElement>("[data-part='explain-tab']");
  const ibeam = root.querySelector<HTMLElement>("[data-cursor='ibeam']");
  const arrow = root.querySelector<HTMLElement>("[data-cursor='arrow']");
  const gradeTo = root.querySelector<HTMLElement>("[data-part='grade-to']");
  const hits = [...root.querySelectorAll<HTMLElement>("[data-clause='hot'] [data-hit]")];
  const sels = hits.map((hit) => hit.querySelector<HTMLElement>("[data-sel]")).filter((el): el is HTMLElement => !!el);
  const answerWords = [...root.querySelectorAll<HTMLElement>("[data-part='answer'] [data-word]")];
  const keys = ["Ctrl", "Shift", "Space"]
    .map((name) => root.querySelector<HTMLElement>(`[data-key="${name}"]`))
    .filter((el): el is HTMLElement => !!el);
  if (!card || !web || !explain || !googleTab || !explainTab || !ibeam || !arrow || !gradeTo || hits.length === 0) return;

  const lip = token("--key-lip", "#d3d3cd");
  const sunken = token("--sunken", "#ececee");
  const text = token("--text", "#1b1b1a");
  const muted = token("--muted", "#62625d");
  const ink = token("--ink", "#1b1b1a");
  const onInk = token("--on-ink", "#fafaf9");
  const surface = token("--surface", "#ffffff");
  const line = token("--line", "rgb(27 27 26 / 0.1)");
  const first = stageOffset(hits[0], root);
  const tab = stageOffset(explainTab, root);
  const beamW = 16;
  const beamH = 26;

  tl.set(card, { opacity: 0, scale: 0.92, y: 16, transformOrigin: "top left" }, 0);
  tl.set(web, { display: "block" }, 0);
  tl.set(explain, { display: "none" }, 0);
  tl.set(googleTab, { backgroundColor: sunken, color: text }, 0);
  tl.set(explainTab, { backgroundColor: surface, color: text, borderColor: line }, 0);
  tl.set(ibeam, { x: first.x - beamW / 2, y: first.y + (first.h - beamH) / 2, opacity: 0 }, 0);
  tl.set(arrow, { x: 560, y: 500, scale: 1, opacity: 0, transformOrigin: "4px 3px" }, 0);
  tl.set(gradeTo, { autoAlpha: 0 }, 0);
  tl.to(ibeam, { opacity: 1, duration: 0.25, ease: "power3.out" }, 0.5);

  const step = 1 / hits.length;
  hits.forEach((hit, i) => {
    const at = 0.7 + i * step;
    const box = stageOffset(hit, root);
    const sel = sels[i];
    if (sel) tl.to(sel, { opacity: 1, duration: Math.min(0.08, step), ease: "power3.out" }, at);
    tl.to(ibeam, { x: box.x + box.w - beamW / 2, y: box.y + (box.h - beamH) / 2, duration: step, ease: "none" }, at);
  });

  tl.to(ibeam, { opacity: 0, duration: 0.15, ease: "power3.out" }, 1.9);

  keys.forEach((key, i) => {
    const at = 1.9 + i * 0.3;
    tl.fromTo(key, { y: -30, opacity: 0 }, { y: 5, opacity: 1, duration: 0.36, ease: "back.out(1.6)" }, at);
    tl.fromTo(key, { boxShadow: `0px 6px 0px 0px ${lip}` }, { boxShadow: `0px 1px 0px 0px ${lip}`, duration: 0.36, ease: "power3.out" }, at);
  });
  tl.to(keys, { opacity: 0, y: 20, duration: 0.4, ease: "power3.out" }, 2.9);

  tl.to(card, { opacity: 1, scale: 1, y: 0, duration: 0.6, ease: "back.out(1.5)" }, 2.9);

  tl.to(arrow, { opacity: 1, duration: 0.15, ease: "power3.out" }, 3.4);
  tl.to(arrow, { x: tab.x + tab.w * 0.55, y: tab.y + tab.h * 0.45, duration: 0.6, ease: "power3.out" }, 3.4);
  tl.to(arrow, { scale: 0.8, duration: 0.09, yoyo: true, repeat: 1, ease: "power3.out" }, 4.1);

  tl.set(googleTab, { backgroundColor: "transparent", color: muted }, 4.1);
  tl.set(explainTab, { backgroundColor: ink, color: onInk, borderColor: "transparent" }, 4.1);
  tl.set(web, { display: "none" }, 4.1);
  tl.set(explain, { display: "flex" }, 4.1);

  tl.to(answerWords, { opacity: 1, duration: 0.2, stagger: 0.05, ease: "power3.out" }, 4.3);

  const gradeNum = gradeTo.querySelector<HTMLElement>("[data-grade-num]");
  const counter = { n: 16 };
  tl.fromTo(
    gradeTo,
    { autoAlpha: 0 },
    {
      autoAlpha: 1,
      duration: 0.2,
      ease: "power3.out",
      onStart: () => {
        if (gradeNum) gradeNum.textContent = "16";
      },
    },
    6.5,
  );
  tl.fromTo(
    counter,
    { n: 16 },
    {
      n: 9,
      duration: 0.5,
      ease: "none",
      immediateRender: false,
      onUpdate: () => {
        if (gradeNum) gradeNum.textContent = String(Math.round(counter.n));
      },
      onComplete: () => {
        if (gradeNum) gradeNum.textContent = "9";
      },
    },
    6.5,
  );

  const marks = answerWords
    .filter((word) => {
      const text = word.textContent?.trim();
      return text === "$1,200" || text === "5th";
    })
    .map((word) => word.querySelector<HTMLElement>("[data-underline]"))
    .filter((el): el is HTMLElement => !!el);
  marks.forEach((mark, i) => {
    tl.fromTo(mark, { scaleX: 0 }, { scaleX: 1, duration: 0.4, ease: "power3.out", transformOrigin: "left center" }, 7 + i * 0.45);
  });

  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (reduce) {
    tl.set(arrow, { opacity: 0 }, 8);
    tl.set(ibeam, { opacity: 0 }, 8);
    return;
  }

  tl.to(card, { opacity: 0, scale: 0.96, duration: 0.6, ease: "power3.out" }, 10);
  tl.to(sels, { opacity: 0, duration: 0.45, ease: "power3.out" }, 10);
  tl.to(arrow, { opacity: 0, duration: 0.3, ease: "power3.out" }, 10);
}
