"use client";

import { useEffect, useLayoutEffect, useState, type ReactNode } from "react";
import gsap from "gsap";
import { replayScene, useScene, type SceneTimeline } from "./engine";
import { Reveal } from "../reveal";

const SERIF = 'Georgia, "Iowan Old Style", Palatino, "Palatino Linotype", serif';
const SOURCE =
  "\u2026the Lessee shall remit payment of the monthly rent of $1,200 no later than the 5th day of each calendar month\u2026";
const PLAIN =
  "The tenant (the Lessee) must pay the monthly rent of $1,200 by the 5th day of each month.";
const FACTS = ["$1,200", "5th", "Lessee)"];

function normalize(word: string) {
  return word.toLowerCase().replace(/[^0-9a-z$%]/g, "");
}

/** Longest common subsequence, words compared lowercase with punctuation stripped ($ and % kept). */
function lcs(source: string[], plain: string[]) {
  const n = source.length;
  const m = plain.length;
  const sn = source.map(normalize);
  const pn = plain.map(normalize);
  const dp: number[][] = Array.from({ length: n + 1 }, () => Array(m + 1).fill(0));
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      dp[i][j] = sn[i] !== "" && sn[i] === pn[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1]);
    }
  }
  const pairs: Array<[number, number]> = [];
  let i = 0;
  let j = 0;
  while (i < n && j < m) {
    if (sn[i] !== "" && sn[i] === pn[j]) {
      pairs.push([i, j]);
      i++;
      j++;
    } else if (dp[i + 1][j] >= dp[i][j + 1]) i++;
    else j++;
  }
  return pairs;
}

function Words({ text, plain, ghost }: { text: string; plain?: boolean; ghost?: boolean }) {
  const words = text.split(/\s+/).filter(Boolean);
  const nodes: ReactNode[] = [];
  words.forEach((word, i) => {
    if (i) nodes.push(" ");
    const fact = plain && FACTS.includes(word);
    nodes.push(
      <span
        key={i}
        data-word={ghost ? undefined : ""}
        className={`inline-block${plain && !ghost ? " opacity-0" : ""}${fact ? " mark" : ""}`}
        style={fact && !ghost ? { backgroundSize: "0% 0.42em" } : undefined}
      >
        {word}
      </span>,
    );
  });
  return nodes;
}

function rel(el: HTMLElement, origin: HTMLElement) {
  const a = el.getBoundingClientRect();
  const b = origin.getBoundingClientRect();
  return { x: a.left - b.left, y: a.top - b.top };
}

function buildMorph(tl: SceneTimeline, root: HTMLElement) {
  const stage = root.querySelector<HTMLElement>("[data-morph='stage']");
  const source = root.querySelector<HTMLElement>("[data-layer='source']");
  const plain = root.querySelector<HTMLElement>("[data-layer='plain']");
  const grade = root.querySelector<HTMLElement>("[data-grade]");
  const fromKicker = root.querySelector<HTMLElement>("[data-kicker='from']");
  const toKicker = root.querySelector<HTMLElement>("[data-kicker='to']");
  const kept = root.querySelector<HTMLElement>("[data-kept]");
  if (!stage || !source || !plain || !grade || !fromKicker || !toKicker || !kept) return;

  const height = Math.max(source.offsetHeight, plain.offsetHeight);
  if (height) stage.style.minHeight = `${height}px`;

  const sourceWords = [...source.querySelectorAll<HTMLElement>("[data-word]")];
  const plainWords = [...plain.querySelectorAll<HTMLElement>("[data-word]")];
  const pairs = lcs(
    sourceWords.map((el) => el.textContent?.trim() ?? ""),
    plainWords.map((el) => el.textContent?.trim() ?? ""),
  );
  const matchedSource = new Set(pairs.map(([index]) => index));
  const matchedPlain = new Set(pairs.map(([, index]) => index));
  const text = getComputedStyle(document.documentElement).getPropertyValue("--text").trim() || "#1b1b1a";
  // The plain layer sits on top. Ignore it until the cross-fade, so the source text can be selected.
  tl.set(plain, { pointerEvents: "none" }, 0);

  pairs.forEach(([si, pi], i) => {
    const from = sourceWords[si];
    const to = plainWords[pi];
    const a = rel(from, stage);
    const b = rel(to, stage);
    const at = i * 0.015;
    tl.to(from, { x: b.x - a.x, y: b.y - a.y, color: text, duration: 1, ease: "expo.inOut" }, at);
    tl.to(from, { opacity: 0, duration: 0.22, ease: "power1.in" }, at + 0.78);
    tl.to(to, { opacity: 1, duration: 0.22, ease: "power1.out" }, at + 0.78);
  });

  let drop = 0;
  sourceWords.forEach((el, index) => {
    if (matchedSource.has(index)) return;
    tl.to(
      el,
      { y: 30, rotation: drop % 2 === 0 ? -6 : 5, opacity: 0, duration: 0.45, ease: "power2.in", transformOrigin: "center center" },
      drop * 0.01,
    );
    drop++;
  });

  let rise = 0;
  plainWords.forEach((el, index) => {
    if (matchedPlain.has(index)) return;
    tl.fromTo(el, { y: -24, opacity: 0 }, { y: 0, opacity: 1, duration: 0.7, ease: "back.out(2)" }, 0.45 + rise * 0.03);
    rise++;
  });

  const arrived = Math.max(0, pairs.length - 1) * 0.015 + 1;
  FACTS.forEach((label, i) => {
    const el = plainWords.find((word) => word.textContent?.trim() === label);
    if (!el) return;
    const at = arrived + 0.12 + i * 0.42;
    tl.fromTo(el, { backgroundSize: "0% 0.42em" }, { backgroundSize: "100% 0.42em", duration: 0.4, ease: "power3.out" }, at);
    tl.fromTo(
      el,
      { scale: 1 },
      { scale: 1.06, duration: 0.4, ease: "elastic.out(1, 0.45)", yoyo: true, repeat: 1, transformOrigin: "center center" },
      at,
    );
  });

  const atEnd = root.dataset.sceneEnd === "1";
  grade.textContent = atEnd ? "9" : "17";
  tl.call(() => {
    if (root.dataset.sceneEnd !== "1") grade.textContent = "17";
  }, undefined, 0);
  const counter = { n: 17 };
  tl.to(
    counter,
    {
      n: 9,
      duration: 1,
      ease: "none",
      onStart: () => {
        counter.n = 17;
        grade.textContent = "17";
      },
      onUpdate: () => {
        grade.textContent = String(Math.round(counter.n));
      },
      onComplete: () => {
        grade.textContent = "9";
      },
    },
    0.25,
  );
  tl.fromTo(
    grade,
    { scale: 1 },
    { scale: 1.06, duration: 0.45, ease: "elastic.out(1, 0.4)", yoyo: true, repeat: 1, transformOrigin: "center center" },
    1.25,
  );

  const endAt = arrived + 0.12 + FACTS.length * 0.42;
  tl.to(fromKicker, { opacity: 0, duration: 0.35 }, endAt);
  tl.to(toKicker, { opacity: 1, duration: 0.35 }, endAt);
  tl.fromTo(kept, { opacity: 0, y: 10 }, { opacity: 1, y: 0, duration: 0.45, ease: "power3.out" }, endAt);
  tl.set(source, { visibility: "hidden" }, endAt + 0.35);
  tl.set(plain, { pointerEvents: "auto" }, endAt + 0.35);
  tl.call(() => {
    grade.textContent = "9";
  }, undefined, endAt + 0.35);
}

export function MorphSection() {
  const [revision, setRevision] = useState(0);
  const ref = useScene(buildMorph, { once: true, threshold: 0.4, revision });

  useLayoutEffect(() => {
    const stage = ref.current?.querySelector<HTMLElement>("[data-morph='stage']");
    const source = ref.current?.querySelector<HTMLElement>("[data-layer='source']");
    const plain = ref.current?.querySelector<HTMLElement>("[data-layer='plain']");
    if (!stage || !source || !plain) return;
    const fit = () => {
      const height = Math.max(source.offsetHeight, plain.offsetHeight);
      if (height) stage.style.minHeight = `${height}px`;
    };
    fit();
    let cancel = false;
    if (document.fonts && document.fonts.status !== "loaded") {
      void document.fonts.ready.then(() => {
        if (!cancel) fit();
      });
    }
    return () => {
      cancel = true;
    };
  }, [ref, revision]);

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
    <section id="how" className="scroll-mt-16 border-t border-line bg-surface">
      <div className="mx-auto max-w-6xl px-5 py-24 md:px-8 md:py-32">
        <Reveal>
          <h2 className="max-w-[14ch] text-4xl leading-[1.05] font-semibold tracking-[-0.03em] md:text-5xl">
            Same meaning. <span className="mark sweep">Plainer words.</span>
          </h2>
        </Reveal>

        <div ref={ref} className="mt-14">
          <div className="flex flex-col gap-10 md:flex-row md:items-start md:justify-between md:gap-12">
            <div className="min-w-0 max-w-[900px] flex-1">
              <div className="relative mb-5 h-4">
                <span data-kicker="from" className="absolute inset-x-0 top-0 font-mono text-[12px] tracking-[0.14em] text-muted uppercase">
                  A clause from a lease
                </span>
                <span data-kicker="to" className="absolute inset-x-0 top-0 font-mono text-[12px] tracking-[0.14em] text-muted uppercase opacity-0">
                  Plain English · same facts
                </span>
              </div>
              <div data-morph="stage" className="relative">
                {/* In-flow copy of both paragraphs, so the box is the taller one before JS measures. */}
                <div className="invisible grid" aria-hidden="true">
                  <p className="col-start-1 row-start-1 text-[22px] leading-[1.4] text-muted md:text-[30px]" style={{ fontFamily: SERIF }}>
                    <Words text={SOURCE} ghost />
                  </p>
                  <p className="col-start-1 row-start-1 text-[26px] leading-[1.35] font-medium tracking-[-0.02em] text-text md:text-[36px]">
                    <Words text={PLAIN} plain ghost />
                  </p>
                </div>
                <p data-layer="source" className="absolute inset-x-0 top-0 text-[22px] leading-[1.4] text-muted md:text-[30px]" style={{ fontFamily: SERIF }}>
                  <Words text={SOURCE} />
                </p>
                <p data-layer="plain" className="absolute inset-x-0 top-0 z-10 text-[26px] leading-[1.35] font-medium tracking-[-0.02em] text-text md:text-[36px]">
                  <Words text={PLAIN} plain />
                </p>
              </div>
            </div>
            <div className="shrink-0 md:w-[220px] md:pt-9 md:text-right">
              <div className="font-mono text-[12px] tracking-[0.14em] text-muted uppercase">Reading grade</div>
              <div data-grade="" className="text-[120px] leading-none font-semibold tracking-[-0.04em] tabular-nums">
                17
              </div>
            </div>
          </div>
          <p data-kept="" className="mt-8 text-[18px] text-text opacity-0">
            Every number, date and name kept.
          </p>
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
