"use client";

import { PlainlyCard } from "./card";
import { Stage, useScene, type SceneTimeline } from "./engine";
import { Cursor } from "./parts";
import { Reveal } from "../reveal";

const LABEL =
  "Plainly explaining selected text in a browser, an email, and a chat. Copy it twice, or press Ctrl+Shift+Space.";

const SCENES: Array<{
  id: string;
  kind: "browser" | "mail" | "chat";
  keys: string[];
  sentence: string;
  answer: string;
  tab?: string;
  url?: string;
  title?: string;
  from?: string;
  name?: string;
}> = [
  {
    id: "browser",
    kind: "browser",
    keys: ["Ctrl", "C", "C"],
    tab: "Cash transfers and savings",
    url: "journal.example.org/articles/cash-transfers",
    title: "Unconditional cash transfers and household savings",
    sentence:
      "Heteroscedasticity-robust standard errors, clustered at the household level, attenuate the estimated effect by approximately 40%.",
    answer:
      "When the researchers allowed for differences between households, the program's measured effect on savings got about 40% smaller.",
  },
  {
    id: "mail",
    kind: "mail",
    keys: ["Ctrl", "Shift", "Space"],
    title: "Notice of assessment",
    from: "Revenue Office",
    sentence:
      "Pursuant to Section 12(b)(iii), you are required to furnish the requisite particulars within thirty (30) days of receipt of this notice.",
    answer: "Under Section 12(b)(iii), you must send the details they asked for within 30 days of getting this letter.",
  },
  {
    id: "chat",
    kind: "chat",
    keys: ["Ctrl", "Shift", "Space"],
    name: "Dr. Rahman",
    sentence:
      "Your results suggest idiopathic thrombocytopenic purpura; we'll start corticosteroids and monitor your platelet count.",
    answer:
      "Your tests point to a condition where your immune system lowers your platelets for no known reason. You'll start steroid medicine, and they'll keep checking your platelet count.",
  },
];

export function AppsSection() {
  const ref = useScene(buildApps, { loop: true, repeatDelay: 0.45 });
  return (
    <section className="border-y border-line bg-surface">
      <div className="mx-auto max-w-6xl px-5 py-24 md:px-8 md:py-32">
        <Reveal>
          <h2 className="max-w-[16ch] text-4xl leading-[1.05] font-semibold tracking-[-0.03em] md:text-5xl">
            Works wherever you <span className="mark sweep">can select text.</span>
          </h2>
          <p className="mt-6 max-w-[46ch] text-[18px] leading-relaxed text-muted">
            PDFs, Word, email, chat, websites. Select, then press the shortcut, or copy it twice.
          </p>
        </Reveal>
        <div className="mt-14">
          <Stage width={980} height={520} label={LABEL}>
            <div ref={ref} className="absolute inset-0 overflow-hidden">
              {SCENES.map((scene) => (
                <div key={scene.id} data-app="" className="absolute inset-0">
                  {scene.kind === "browser" && (
                    <BrowserWindow tab={scene.tab ?? ""} url={scene.url ?? ""} title={scene.title ?? ""} sentence={scene.sentence} />
                  )}
                  {scene.kind === "mail" && (
                    <MailWindow title={scene.title ?? ""} from={scene.from ?? ""} sentence={scene.sentence} />
                  )}
                  {scene.kind === "chat" && <ChatWindow name={scene.name ?? ""} sentence={scene.sentence} />}
                  <div className="absolute top-2 z-10" style={{ left: 588 }}>
                    <div style={{ transform: "scale(0.78)", transformOrigin: "top left" }}>
                      <PlainlyCard
                        quote={scene.sentence}
                        tab="explain"
                        mode="explain"
                        source={scene.sentence}
                        answerLabel="Plain English"
                        answer={scene.answer}
                        level="plain"
                        grade={{ from: 17 }}
                        showGrade={false}
                      />
                    </div>
                  </div>
                  <div className="absolute bottom-3 left-0 z-20 flex w-[600px] items-end justify-center gap-3">
                    {scene.keys.map((key, i) => (
                      <div
                        key={`${scene.id}-${i}`}
                        data-keycap=""
                        className="grid h-16 min-w-[72px] place-items-center rounded-[14px] border border-line bg-surface px-4 text-[22px] font-medium text-text opacity-0 shadow-[0_6px_0_var(--key-lip)]"
                      >
                        {key}
                      </div>
                    ))}
                  </div>
                  <Cursor kind="ibeam" />
                </div>
              ))}
            </div>
          </Stage>
        </div>
        <div className="mt-8 flex flex-wrap gap-x-14 gap-y-6">
          <div>
            <p className="mb-3 text-[14px] text-muted">Select, then press</p>
            <StaticKeys keys={["Ctrl", "Shift", "Space"]} />
          </div>
          <div>
            <p className="mb-3 text-[14px] text-muted">Or copy it twice</p>
            <StaticKeys keys={["Ctrl", "C", "C"]} />
          </div>
        </div>
      </div>
    </section>
  );
}

function StaticKeys({ keys }: { keys: string[] }) {
  return (
    <span className="inline-flex items-center gap-1.5">
      {keys.map((key, i) => (
        <kbd
          key={i}
          className="inline-grid h-8 min-w-8 place-items-center rounded-md border border-line bg-bg px-2 text-[13px] font-medium text-text shadow-[0_1px_0_var(--line)]"
        >
          {key}
        </kbd>
      ))}
    </span>
  );
}

function Sentence({ text }: { text: string }) {
  const words = text.split(/\s+/).filter(Boolean);
  return (
    <p className="text-[15px] leading-[1.65]">
      {words.map((word, i) => (
        <span key={`${word}-${i}`} data-hit="">
          <span className="relative inline-block rounded-[2px]">
            <span data-sel="" className="absolute inset-0 rounded-[2px] bg-[var(--mark)] opacity-0" />
            <span className="relative">{word}</span>
          </span>
          {i < words.length - 1 ? " " : ""}
        </span>
      ))}
    </p>
  );
}

function BrowserWindow({ tab, url, title, sentence }: { tab: string; url: string; title: string; sentence: string }) {
  return (
    <div className="absolute top-2 left-0 h-[440px] w-[600px] overflow-hidden rounded-xl border border-line bg-sunken shadow-[var(--shadow)]">
      <div className="flex h-10 items-end border-b border-line bg-surface px-3">
        <div className="flex h-7 max-w-[230px] items-center rounded-t-md border border-b-0 border-line bg-bg px-3 text-[12.5px] text-text">
          <span className="truncate">{tab}</span>
        </div>
      </div>
      <div className="flex h-9 items-center border-b border-line bg-surface px-3">
        <div className="flex h-6 min-w-0 flex-1 items-center rounded-md bg-sunken px-2.5 text-[12px] text-muted">
          <span className="truncate">{url}</span>
        </div>
      </div>
      <div className="h-[calc(100%-76px)] overflow-hidden bg-white px-6 py-5 text-[#1b1b1a]">
        <div className="mb-2 font-mono text-[11px] tracking-[0.14em] text-[#8a8a84] uppercase">Abstract</div>
        <div className="mb-3 text-[20px] leading-snug font-semibold" style={{ fontFamily: SERIF }}>
          {title}
        </div>
        <Sentence text={sentence} />
      </div>
    </div>
  );
}

function MailWindow({ title, from, sentence }: { title: string; from: string; sentence: string }) {
  return (
    <div className="absolute top-2 left-0 h-[440px] w-[600px] overflow-hidden rounded-xl border border-line bg-sunken shadow-[var(--shadow)]">
      <div className="flex h-11 items-center border-b border-line bg-surface px-4 text-[13px] font-medium text-text">
        <span className="truncate">{title}</span>
      </div>
      <div className="h-[calc(100%-44px)] overflow-hidden bg-white px-6 py-5 text-[#1b1b1a]">
        <div className="mb-4 text-[14px] text-[#62625d]">
          From <span className="font-medium text-[#1b1b1a]">{from}</span>
        </div>
        <div className="mb-4 text-[20px] leading-snug font-semibold" style={{ fontFamily: SERIF }}>
          {title}
        </div>
        <Sentence text={sentence} />
      </div>
    </div>
  );
}

function ChatWindow({ name, sentence }: { name: string; sentence: string }) {
  return (
    <div className="absolute top-2 left-0 flex h-[440px] w-[600px] flex-col overflow-hidden rounded-xl border border-line bg-sunken shadow-[var(--shadow)]">
      <div className="flex h-12 items-center border-b border-line bg-surface px-4 text-[14px] font-medium text-text">{name}</div>
      <div className="min-h-0 flex-1 bg-white px-5 py-5">
        <div className="mb-1.5 text-[12px] text-[#8a8a84]">{name}</div>
        <div className="max-w-[460px] rounded-2xl rounded-tl-md bg-[#f3f3f1] px-4 py-3 text-[#1b1b1a]">
          <Sentence text={sentence} />
        </div>
      </div>
    </div>
  );
}

const SERIF = 'Georgia, "Iowan Old Style", Palatino, "Palatino Linotype", serif';

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

function buildApps(tl: SceneTimeline, root: HTMLElement) {
  const scenes = [...root.querySelectorAll<HTMLElement>("[data-app]")];
  if (scenes.length === 0) return;
  const lip = token("--key-lip", "#d3d3cd");
  scenes.forEach((scene, i) => {
    tl.set(scene, { x: i === 0 ? 0 : 1040 }, 0);
  });
  let at = 0.15;
  scenes.forEach((scene, i) => {
    if (i > 0) {
      const prev = scenes[i - 1];
      if (prev) tl.to(prev, { x: -1040, duration: 0.6, ease: "power3.inOut" }, at);
      tl.fromTo(scene, { x: 1040, immediateRender: false }, { x: 0, duration: 0.6, ease: "power3.inOut" }, at);
      at += 0.6;
    }
    at = playApp(tl, scene, at, lip);
  });
}

function playApp(tl: SceneTimeline, scene: HTMLElement, at: number, lip: string) {
  const hits = [...scene.querySelectorAll<HTMLElement>("[data-hit]")];
  const sels = hits
    .map((hit) => hit.querySelector<HTMLElement>("[data-sel]"))
    .filter((el): el is HTMLElement => !!el);
  const ibeam = scene.querySelector<HTMLElement>("[data-cursor='ibeam']");
  const keys = [...scene.querySelectorAll<HTMLElement>("[data-keycap]")];
  const card = scene.querySelector<HTMLElement>("[data-part='card']");
  const words = [...scene.querySelectorAll<HTMLElement>("[data-part='answer'] [data-word]")];
  if (!ibeam || !card || hits.length === 0) return at + 1;

  const first = box(hits[0]!, scene);
  const beamW = 16;
  const beamH = 26;
  tl.set(ibeam, { x: first.x, y: first.y + (first.h - beamH) / 2, opacity: 0 }, at);
  tl.set(sels, { opacity: 0 }, at);
  tl.set(keys, { opacity: 0, y: -30 }, at);
  tl.set(card, { opacity: 0, y: 16, scale: 0.96, transformOrigin: "top left" }, at);
  tl.set(words, { opacity: 0 }, at);

  let t = at + 0.12;
  tl.to(ibeam, { opacity: 1, duration: 0.18, ease: "power3.out" }, t);
  const sweep = 1.05;
  const step = sweep / hits.length;
  hits.forEach((hit, i) => {
    const spot = box(hit, scene);
    const sel = sels[i];
    const when = t + i * step;
    if (sel) tl.to(sel, { opacity: 1, duration: Math.min(0.08, step), ease: "power3.out" }, when);
    tl.to(ibeam, { x: spot.x + spot.w - beamW / 2, y: spot.y + (spot.h - beamH) / 2, duration: step, ease: "none" }, when);
  });
  t += sweep;
  tl.to(ibeam, { opacity: 0, duration: 0.15, ease: "power3.out" }, t);

  keys.forEach((key, i) => {
    const when = t + i * 0.26;
    tl.fromTo(key, { y: -28, opacity: 0 }, { y: 5, opacity: 1, duration: 0.32, ease: "back.out(1.6)" }, when);
    tl.fromTo(
      key,
      { boxShadow: `0px 6px 0px 0px ${lip}` },
      { boxShadow: `0px 1px 0px 0px ${lip}`, duration: 0.32, ease: "power3.out" },
      when,
    );
  });
  t += keys.length * 0.26 + 0.28;
  tl.to(card, { opacity: 1, y: 0, scale: 1, duration: 0.45, ease: "power3.out" }, t);
  t += 0.32;
  const stagger = 0.018;
  tl.to(words, { opacity: 1, duration: 0.22, stagger, ease: "power2.out" }, t);
  const stream = 0.22 + Math.max(0, words.length - 1) * stagger;
  const end = t + stream + 2.5;
  tl.set(card, { opacity: 1 }, end);
  return end;
}
