"use client";

import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent } from "react";
import gsap from "gsap";
import { PlainlyCard } from "./card";
import { Stage } from "./engine";
import { Reveal } from "../reveal";

const CLAUSE =
  "7.2 Notwithstanding any provision of this Agreement to the contrary, the Lessee shall remit payment of the monthly rent of $1,200 no later than the 5th day of each calendar month, failing which a late fee equivalent to 5% of the outstanding amount shall accrue.";

type LevelId = "simpler" | "plain" | "clearer";

const LEVELS: Array<{ id: LevelId; label: string; grade: number; answerLabel: string; answer: string }> = [
  {
    id: "simpler",
    label: "Simpler",
    grade: 5,
    answerLabel: "Simpler English",
    answer:
      "The tenant (the Lessee) must pay $1,200 rent every month. They must pay it by the 5th day of the month. If they pay late, they owe an extra fee. The fee is 5% of the money they still owe. This rule comes before any other rule in the agreement.",
  },
  {
    id: "plain",
    label: "Plain",
    grade: 9,
    answerLabel: "Plain English",
    answer:
      "Whatever else this agreement says, the tenant (the Lessee) must pay the monthly rent of $1,200 by the 5th day of each month. If they don't, a late fee of 5% of the unpaid amount is added.",
  },
  {
    id: "clearer",
    label: "Clearer",
    grade: 11,
    answerLabel: "Clearer English",
    answer:
      "Despite anything else in this Agreement, the Lessee must pay the monthly rent of $1,200 by the 5th day of each calendar month. If the Lessee pays late, a late fee of 5% of the outstanding amount accrues.",
  },
];

const CYCLE: LevelId[] = ["plain", "simpler", "clearer"];
const ORDER: LevelId[] = ["simpler", "plain", "clearer"];

export function LevelsSection() {
  const [level, setLevel] = useState<LevelId>("plain");
  const [focus, setFocus] = useState<LevelId>("plain");
  const rootRef = useRef<HTMLDivElement>(null);
  const hostRef = useRef<HTMLDivElement>(null);
  const readoutRef = useRef<HTMLSpanElement>(null);
  const buttons = useRef<Record<LevelId, HTMLButtonElement | null>>({ simpler: null, plain: null, clearer: null });
  const touched = useRef(false);
  const inView = useRef(false);
  const levelRef = useRef(level);
  const shownRef = useRef(9);
  const [shown, setShown] = useState(9);
  levelRef.current = level;

  const current = LEVELS.find((item) => item.id === level) ?? LEVELS[1];

  useEffect(() => {
    const el = rootRef.current;
    if (!el) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const observer = new IntersectionObserver(
      ([entry]) => {
        inView.current = !!entry && entry.intersectionRatio >= 0.35;
      },
      { threshold: [0, 0.35, 1] },
    );
    observer.observe(el);
    const id = window.setInterval(() => {
      if (touched.current || !inView.current) return;
      const next = CYCLE[(CYCLE.indexOf(levelRef.current) + 1) % CYCLE.length];
      setLevel(next);
      setFocus(next);
    }, 4500);
    return () => {
      observer.disconnect();
      window.clearInterval(id);
    };
  }, []);

  useLayoutEffect(() => {
    const host = hostRef.current;
    const readout = readoutRef.current;
    if (!host) return;
    const card = host.querySelector<HTMLElement>("[data-part='card']");
    const gradeTo = host.querySelector<HTMLElement>("[data-part='grade-to']");
    if (card) card.style.opacity = "1";
    if (gradeTo) gradeTo.style.opacity = "1";

    const words = [...host.querySelectorAll<HTMLElement>("[data-part='answer'] [data-word]")];
    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (reduce) {
      gsap.set(words, { opacity: 1 });
    } else if (words.length) {
      const stream = gsap.to(words, { opacity: 1, duration: 0.2, stagger: 0.025, ease: "power3.out" });
      return () => {
        stream.kill();
      };
    }
  }, [level]);

  useEffect(() => {
    const target = current.grade;
    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const from = shownRef.current;
    if (reduce || from === target) {
      shownRef.current = target;
      setShown(target);
      return;
    }
    const counter = { n: from };
    const tween = gsap.to(counter, {
      n: target,
      duration: 0.55,
      ease: "power2.out",
      onUpdate: () => {
        const next = Math.round(counter.n);
        shownRef.current = next;
        setShown(next);
      },
      onComplete: () => {
        shownRef.current = target;
        setShown(target);
      },
    });
    return () => {
      tween.kill();
    };
  }, [current.grade]);

  function stop() {
    touched.current = true;
  }

  function choose(id: LevelId) {
    stop();
    setFocus(id);
    setLevel(id);
  }

  function onKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    const index = ORDER.indexOf(focus);
    const move = (dir: number) => {
      event.preventDefault();
      stop();
      const next = ORDER[(index + dir + ORDER.length) % ORDER.length];
      setFocus(next);
      buttons.current[next]?.focus();
    };
    if (event.key === "ArrowRight" || event.key === "ArrowDown") move(1);
    else if (event.key === "ArrowLeft" || event.key === "ArrowUp") move(-1);
  }

  return (
    <section className="bg-bg">
      <div ref={rootRef} className="mx-auto grid max-w-6xl items-center gap-14 px-5 py-24 md:px-8 md:py-32 lg:grid-cols-2 lg:gap-16">
        <div>
          <Reveal>
            <h2 className="text-4xl leading-[1.05] font-semibold tracking-[-0.03em] md:text-5xl">
              Pick how <span className="mark sweep">plain.</span>
            </h2>
          </Reveal>
          <p className="mt-6 max-w-[36ch] text-[18px] leading-relaxed text-muted">
            Simpler for anyone. Plain for most. Clearer keeps the terms and untangles them.
          </p>
          <div
            role="radiogroup"
            aria-label="Reading level"
            aria-orientation="horizontal"
            className="mt-8 flex w-full max-w-xl rounded-lg bg-sunken p-1"
            onKeyDown={onKeyDown}
            onPointerDown={stop}
            onFocus={stop}
          >
            {LEVELS.map((item) => {
              const on = item.id === level;
              return (
                <button
                  key={item.id}
                  ref={(node) => {
                    buttons.current[item.id] = node;
                  }}
                  type="button"
                  role="radio"
                  aria-checked={on}
                  tabIndex={focus === item.id ? 0 : -1}
                  className={`h-11 flex-1 rounded-md text-[16px] font-medium ${
                    on ? "bg-surface text-text shadow-[0_0_0_1px_var(--line)]" : "text-muted"
                  }`}
                  onClick={() => choose(item.id)}
                >
                  {item.label}
                </button>
              );
            })}
          </div>
          <div className="mt-8 text-[56px] leading-none font-semibold tracking-[-0.04em] md:text-[72px]">
            <span className="text-muted">Grade 17</span>
            <span className="mx-3 text-muted">{"\u2192"}</span>
            <span ref={readoutRef} className="tabular-nums">
              {shown}
            </span>
          </div>
        </div>
        <div ref={hostRef} className="min-w-0">
          <Stage width={480} height={560} label={`Plainly card at the ${current.label} reading level, grade 17 to ${current.grade}`}>
            <PlainlyCard
              className="!opacity-100"
              quote={CLAUSE}
              tab="explain"
              mode="explain"
              source={CLAUSE}
              answerLabel={current.answerLabel}
              answer={current.answer}
              level={current.id}
              grade={{ from: 17, to: current.grade }}
            />
          </Stage>
        </div>
      </div>
    </section>
  );
}
