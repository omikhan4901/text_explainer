"use client";

import { Icon } from "../icons";
import { Stage, useScene, type SceneTimeline } from "./engine";
import { Cursor } from "./parts";
import { Reveal } from "../reveal";

const TABS = ["Google", "Bing", "DuckDuckGo", "Wikipedia", "Dictionary"] as const;
const SAVED = [
  ["carte blanche", "complete freedom to act as you wish"],
  ["accrue", "grow by addition"],
  ["notwithstanding", "despite anything to the contrary"],
] as const;

const LABEL = "Looking up the word ubiquitous, then saving it to the Words list.";

const BOOKMARK_LINE =
  "M184,32H72A16,16,0,0,0,56,48V224a8,8,0,0,0,12.24,6.78L128,193.43l59.77,37.35A8,8,0,0,0,200,224V48A16,16,0,0,0,184,32Zm0,177.57-51.77-32.35a8,8,0,0,0-8.48,0L72,209.57V48H184Z";
const BOOKMARK_FILL =
  "M184,32H72A16,16,0,0,0,56,48V224a8,8,0,0,0,12.24,6.78L128,193.43l59.77,37.35A8,8,0,0,0,200,224V48A16,16,0,0,0,184,32Z";
const SPEAKER =
  "M155.51,24.81a8,8,0,0,0-8.42.88L77.25,80H32A16,16,0,0,0,16,96v64a16,16,0,0,0,16,16H77.25l69.84,54.31A8,8,0,0,0,160,224V32A8,8,0,0,0,155.51,24.81ZM32,96H72v64H32ZM144,207.64,88,164.09V91.91l56-43.55Zm54-106.08a40,40,0,0,1,0,52.88,8,8,0,0,1-12-10.58,24,24,0,0,0,0-31.72,8,8,0,0,1,12-10.58ZM248,128a79.9,79.9,0,0,1-20.37,53.34,8,8,0,0,1-11.92-10.67,64,64,0,0,0,0-85.33,8,8,0,1,1,11.92-10.67A79.83,79.83,0,0,1,248,128Z";

export function WordsSection() {
  const ref = useScene(buildWords, { loop: true, repeatDelay: 0.7 });
  return (
    <section className="border-y border-line bg-surface">
      <div className="mx-auto max-w-6xl px-5 py-24 md:px-8 md:py-32">
        <Reveal>
          <h2 className="max-w-[16ch] text-4xl leading-[1.05] font-semibold tracking-[-0.03em] md:text-5xl">
            Look up any word. <span className="mark sweep">Keep it.</span>
          </h2>
          <p className="mt-6 max-w-[46ch] text-[18px] leading-relaxed text-muted">
            Heard a word you don&apos;t know? Press the shortcut with nothing selected and type it. Save the ones worth remembering, hear them read aloud, export them to Excel or Anki.
          </p>
        </Reveal>
        <div className="mt-14">
          <Stage width={900} height={480} label={LABEL}>
            <div ref={ref} className="absolute inset-0">
              <div
                data-lookup=""
                className="absolute overflow-hidden rounded-xl bg-surface"
                style={{ left: 24, top: 186, width: 560, height: 96, boxShadow: "var(--shadow), 0 0 0 1px var(--line)" }}
              >
                <div className="flex h-[58px] items-center gap-3 px-4">
                  <Icon name="search" size={18} className="shrink-0 text-muted" />
                  <span className="relative min-w-0 flex-1 text-[16px]">
                    <span data-placeholder="" className="text-muted">
                      Look up a word or phrase
                    </span>
                    <span data-typed="" className="absolute inset-0 flex items-center text-text">
                      {"ubiquitous".split("").map((letter, i) => (
                        <span key={i} data-letter="" className="inline-block overflow-hidden" style={{ width: 0 }}>
                          {letter}
                        </span>
                      ))}
                      <span data-caret="" className="ml-px inline-block h-[1.05em] w-px shrink-0 bg-text" />
                    </span>
                  </span>
                </div>
                <div className="flex h-[38px] items-center gap-4 border-t border-line px-4 text-[12px] text-muted">
                  <span className="inline-flex items-center gap-1.5">
                    <kbd data-enter="" className="inline-grid h-5 min-w-5 place-items-center rounded border border-line bg-bg px-1 text-[11px] font-medium text-text">
                      Enter
                    </kbd>
                    Look up
                  </span>
                  <span className="inline-flex items-center gap-1.5">
                    <kbd className="inline-grid h-5 min-w-5 place-items-center rounded border border-line bg-bg px-1 text-[11px] font-medium text-text">
                      Esc
                    </kbd>
                    Close
                  </span>
                </div>
              </div>

              <div
                data-wordcard=""
                className="absolute overflow-hidden rounded-xl bg-surface opacity-0"
                style={{ left: 24, top: 28, width: 530, height: 400, boxShadow: "var(--shadow), 0 0 0 1px var(--line)" }}
              >
                <div className="flex h-12 items-center gap-1 border-b border-line pr-1.5 pl-4">
                  <span className="min-w-0 flex-1 truncate text-[16px] font-medium">ubiquitous</span>
                  <span className="grid size-8 place-items-center text-muted">
                    <svg viewBox="0 0 256 256" width="18" height="18" fill="currentColor" aria-hidden="true">
                      <path d={SPEAKER} />
                    </svg>
                  </span>
                  <span data-save="" className="grid size-8 place-items-center text-text">
                    <svg viewBox="0 0 256 256" width="18" height="18" aria-hidden="true">
                      <path data-bookmark-line="" d={BOOKMARK_LINE} fill="currentColor" />
                      <path data-bookmark-fill="" d={BOOKMARK_FILL} fill="var(--saffron)" opacity="0" />
                    </svg>
                  </span>
                  <span className="grid size-8 place-items-center text-muted">
                    <Icon name="x" size={16} />
                  </span>
                </div>
                <div className="flex items-center gap-1 border-b border-line px-2 py-2">
                  {TABS.map((name) => {
                    const on = name === "Dictionary";
                    return (
                      <span
                        key={name}
                        className={`grid h-7 shrink-0 place-items-center rounded-md px-2 text-[12.5px] font-medium whitespace-nowrap ${
                          on ? "bg-sunken text-text" : "text-muted"
                        }`}
                      >
                        {name}
                      </span>
                    );
                  })}
                </div>
                <div className="px-4 pt-3 pb-12">
                  <div className="text-[12.5px] text-muted italic">adjective</div>
                  <Sense n="1" def="being present everywhere at once" example={"\u201Cubiquitous computing\u201D"} also="omnipresent" />
                  <Sense
                    n="2"
                    def="seeming to be everywhere, often to the point of being tiresome"
                    example={"\u201Cthe ubiquitous advertising on buses\u201D"}
                    also="pervasive, universal"
                  />
                </div>
                <div className="absolute inset-x-0 bottom-0 flex h-9 items-center border-t border-line px-4 text-[12px] text-muted">
                  WordNet, on this computer
                </div>
              </div>

              <div
                data-list=""
                className="absolute overflow-hidden rounded-xl bg-surface"
                style={{ left: 612, top: 48, width: 264, boxShadow: "var(--shadow), 0 0 0 1px var(--line)" }}
              >
                <div className="border-b border-line px-3 py-2.5 text-[13px] font-medium">Words</div>
                <div data-slot="" className="overflow-hidden" style={{ height: 0 }}>
                  <div className="relative border-b border-line">
                    <span data-flash="" className="absolute inset-0 bg-[var(--saffron)] opacity-0" />
                    <WordRow word="ubiquitous" mean="being present everywhere at once" />
                  </div>
                </div>
                {SAVED.map(([word, mean]) => (
                  <div key={word} className="border-b border-line last:border-b-0">
                    <WordRow word={word} mean={mean} />
                  </div>
                ))}
              </div>
              <Cursor kind="arrow" />
            </div>
          </Stage>
        </div>
      </div>
    </section>
  );
}

function Sense({ n, def, example, also }: { n: string; def: string; example: string; also: string }) {
  return (
    <div className="mt-2.5">
      <div className="text-[14px] leading-relaxed">
        <span className="mr-2 font-medium text-muted">{n}</span>
        {def}
      </div>
      <div className="mt-0.5 pl-5 text-[13px] text-muted">{example}</div>
      <div className="mt-0.5 pl-5 text-[12.5px] text-muted">Also: {also}</div>
    </div>
  );
}

function WordRow({ word, mean }: { word: string; mean: string }) {
  return (
    <div className="relative px-3 py-2 text-[12.5px] leading-snug">
      <span className="font-medium">{word}</span>
      <span className="text-muted"> / {mean}</span>
    </div>
  );
}

function box(el: HTMLElement, root: HTMLElement) {
  const k = root.getBoundingClientRect().width / root.offsetWidth || 1;
  const a = el.getBoundingClientRect();
  const b = root.getBoundingClientRect();
  return { x: (a.left - b.left) / k, y: (a.top - b.top) / k, w: a.width / k, h: a.height / k };
}

function buildWords(tl: SceneTimeline, root: HTMLElement) {
  const lookup = root.querySelector<HTMLElement>("[data-lookup]");
  const typed = root.querySelector<HTMLElement>("[data-typed]");
  const placeholder = root.querySelector<HTMLElement>("[data-placeholder]");
  const caret = root.querySelector<HTMLElement>("[data-caret]");
  const enter = root.querySelector<HTMLElement>("[data-enter]");
  const card = root.querySelector<HTMLElement>("[data-wordcard]");
  const save = root.querySelector<HTMLElement>("[data-save]");
  const line = root.querySelector<SVGPathElement>("[data-bookmark-line]");
  const fill = root.querySelector<SVGPathElement>("[data-bookmark-fill]");
  const slot = root.querySelector<HTMLElement>("[data-slot]");
  const flash = root.querySelector<HTMLElement>("[data-flash]");
  const arrow = root.querySelector<HTMLElement>("[data-cursor='arrow']");
  const inner = slot?.firstElementChild as HTMLElement | null;
  if (!lookup || !typed || !placeholder || !caret || !enter || !card || !save || !line || !fill || !slot || !flash || !arrow || !inner) return;

  const letters = [...typed.querySelectorAll<HTMLElement>("[data-letter]")];
  const letterWidths = letters.map((letter) => {
    letter.style.width = "auto";
    const width = letter.offsetWidth;
    letter.style.width = "0px";
    return width;
  });
  const rowH = inner.offsetHeight || 52;
  const cardBox = box(card, root);
  const slotBox = box(slot, root);
  const saveBox = box(save, root);
  const scale = Math.min(0.55, slotBox.w / cardBox.w || 0.4);

  tl.set(lookup, { opacity: 1, y: 0, scale: 1, transformOrigin: "top left" }, 0);
  tl.set(letters, { width: 0 }, 0);
  tl.set(placeholder, { opacity: 1 }, 0);
  tl.set(caret, { opacity: 1 }, 0);
  tl.set(card, { opacity: 0, x: 0, y: 14, scale: 0.96, transformOrigin: "top left" }, 0);
  tl.set(line, { opacity: 1 }, 0);
  tl.set(fill, { opacity: 0 }, 0);
  tl.set(slot, { height: 0 }, 0);
  tl.set(flash, { opacity: 0 }, 0);
  tl.set(arrow, { opacity: 0, scale: 1, x: saveBox.x + 70, y: saveBox.y + 80, transformOrigin: "4px 3px" }, 0);

  tl.to(placeholder, { opacity: 0, duration: 0.15 }, 0.35);
  letters.forEach((letter, i) => {
    tl.to(letter, { width: letterWidths[i] ?? 0, duration: 0.06, ease: "none" }, 0.35 + i * 0.1);
  });
  tl.to(caret, { opacity: 0, duration: 0.32, repeat: 3, yoyo: true, ease: "steps(1)" }, 0.35);

  const press = 1.7;
  tl.to(enter, { y: 1, duration: 0.08, yoyo: true, repeat: 1 }, press);
  tl.to(lookup, { opacity: 0, y: 10, scale: 0.98, duration: 0.28, ease: "power2.in" }, press + 0.16);
  tl.to(card, { opacity: 1, y: 0, scale: 1, duration: 0.45, ease: "power3.out" }, press + 0.28);

  const aim = press + 1.05;
  tl.to(arrow, { opacity: 1, x: saveBox.x + saveBox.w * 0.35, y: saveBox.y + saveBox.h * 0.4, duration: 0.5, ease: "power3.inOut" }, aim);
  tl.to(arrow, { scale: 0.86, duration: 0.08, yoyo: true, repeat: 1, transformOrigin: "4px 3px" }, aim + 0.52);
  const saved = aim + 0.58;
  tl.to(fill, { opacity: 1, duration: 0.2 }, saved);
  tl.to(line, { opacity: 0, duration: 0.2 }, saved);
  tl.to(arrow, { opacity: 0, duration: 0.2 }, saved + 0.15);

  const fly = saved + 0.28;
  tl.to(
    card,
    {
      x: slotBox.x - cardBox.x,
      y: slotBox.y - cardBox.y,
      scale,
      duration: 0.7,
      ease: "power3.inOut",
    },
    fly,
  );
  tl.to(card, { opacity: 0, duration: 0.18 }, fly + 0.55);
  tl.to(slot, { height: rowH, duration: 0.35, ease: "power3.out" }, fly + 0.5);
  tl.fromTo(flash, { opacity: 1 }, { opacity: 0, duration: 0.85, ease: "power2.out" }, fly + 0.5);
  tl.set(flash, { opacity: 0 }, fly + 2.6);
}
