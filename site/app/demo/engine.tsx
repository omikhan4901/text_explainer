"use client";

import { useLayoutEffect, useRef, useState, type ReactNode, type RefObject } from "react";
import gsap from "gsap";

export type SceneTimeline = gsap.core.Timeline;

/** Design-size box that scales to the parent's width, never past 1.25, and reserves the scaled height. */
export function Stage({
  width,
  height,
  label,
  children,
}: {
  width: number;
  height: number;
  label: string;
  children: ReactNode;
}) {
  const frame = useRef<HTMLDivElement>(null);
  const [k, setK] = useState<number | null>(null);

  useLayoutEffect(() => {
    const el = frame.current;
    const parent = el?.parentElement;
    if (!el || !parent) return;
    const apply = () => {
      const w = parent.clientWidth;
      if (!w) return;
      const next = Math.min(1.25, w / width);
      setK((prev) => (prev !== null && Math.abs(prev - next) < 0.001 ? prev : next));
    };
    apply();
    const observer = new ResizeObserver(apply);
    observer.observe(parent);
    return () => observer.disconnect();
  }, [width]);

  return (
    <div
      ref={frame}
      role="img"
      aria-label={label}
      className="stage-frame relative"
      style={{
        ["--stage-w" as string]: String(width),
        ["--stage-h" as string]: String(height),
        height: k === null ? undefined : height * k,
      }}
    >
      <div
        aria-hidden="true"
        className="stage-canvas"
        style={k === null ? undefined : { transform: `scale(${k})` }}
      >
        {children}
      </div>
    </div>
  );
}

type SceneRecord = {
  tl: gsap.core.Timeline;
  done: { current: boolean };
  reduce: boolean;
};

const scenes = new WeakMap<HTMLElement, SceneRecord>();

/** Restart a one-shot scene. Reduced motion stays on the final frame. */
export function replayScene(root: HTMLElement | null) {
  if (!root) return;
  const scene = scenes.get(root);
  if (!scene) return;
  if (scene.reduce) {
    scene.tl.progress(1);
    scene.done.current = true;
    return;
  }
  scene.done.current = false;
  scene.tl.restart();
}

/**
 * Owns one paused timeline. Reduced motion rests on the last frame.
 * By default the timeline plays while at least 35% of the root is on screen.
 * `once` plays a single time (it still pauses off-screen until that pass finishes).
 * `revision` tears the scene down and rebuilds it, so a resize can re-measure.
 * A finished one-shot rebuilds already on its last frame.
 */
export function useScene(
  build: (tl: SceneTimeline, root: HTMLElement) => void,
  options?: { loop?: boolean; repeatDelay?: number; threshold?: number; once?: boolean; revision?: number },
): RefObject<HTMLDivElement | null> {
  const ref = useRef<HTMLDivElement>(null);
  const buildRef = useRef(build);
  buildRef.current = build;
  const doneRef = useRef(false);
  const loop = options?.loop ?? false;
  const repeatDelay = options?.repeatDelay ?? 0;
  const threshold = options?.threshold ?? 0.35;
  const once = options?.once ?? false;
  const revision = options?.revision ?? 0;

  useLayoutEffect(() => {
    const root = ref.current;
    if (!root) return;
    let ctx: gsap.Context | null = null;
    let io: IntersectionObserver | null = null;
    let cancelled = false;

    const setup = () => {
      if (cancelled || !ref.current) return;
      const el = ref.current;
      const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
      const atEnd = reduce || (once && doneRef.current);
      if (atEnd) el.dataset.sceneEnd = "1";
      else delete el.dataset.sceneEnd;

      ctx = gsap.context(() => {
        const tl = gsap.timeline({ paused: true });
        if (once) {
          tl.eventCallback("onComplete", () => {
            doneRef.current = true;
          });
        }
        buildRef.current(tl, el);
        scenes.set(el, { tl, done: doneRef, reduce });

        if (atEnd) {
          tl.progress(1);
          doneRef.current = true;
        }
        if (reduce) return;

        if (loop) {
          tl.repeat(-1);
          tl.repeatDelay(repeatDelay);
        }
        io = new IntersectionObserver(
          (entries) => {
            const last = entries[entries.length - 1];
            if (once && doneRef.current) return;
            if (last && last.intersectionRatio >= threshold) tl.play();
            else tl.pause();
          },
          { threshold: [0, threshold, 1] },
        );
        io.observe(el);
      }, el);
    };

    if (document.fonts && document.fonts.status !== "loaded") void document.fonts.ready.then(setup);
    else setup();

    return () => {
      cancelled = true;
      io?.disconnect();
      ctx?.revert();
      if (ref.current) scenes.delete(ref.current);
    };
  }, [loop, repeatDelay, threshold, once, revision]);

  return ref;
}
