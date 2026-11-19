"use client";

import { useScene, type SceneTimeline } from "./engine";

const BARS: Array<[number, number, number, number, string]> = [
  [120, 146, 272, 38, "120 165"],
  [120, 236, 80, 38, "120 255"],
  [120, 328, 194, 38, "120 347"],
];

export function LogoDraw() {
  const ref = useScene(buildLogo, { once: true, threshold: 0.5 });
  return (
    <div ref={ref} className="mx-auto h-[72px] w-[72px]">
      <svg data-logo="" viewBox="0 0 512 512" width="72" height="72" aria-hidden="true" className="overflow-visible">
        <g data-plate="">
          <rect x="32" y="32" width="448" height="448" rx="104" fill="#1b1b1a" />
        </g>
        {BARS.map(([x, y, w, h]) => (
          <rect key={`${x}-${y}`} data-bar="" x={x} y={y} width={w} height={h} rx="19" fill="#7c7c75" />
        ))}
        <rect data-saffron="" x="226" y="228" width="166" height="56" rx="28" fill="#e7b46a" />
      </svg>
    </div>
  );
}

function buildLogo(tl: SceneTimeline, root: HTMLElement) {
  const plate = root.querySelector<SVGGElement>("[data-plate]");
  const bars = [...root.querySelectorAll<SVGRectElement>("[data-bar]")];
  const saffron = root.querySelector<SVGRectElement>("[data-saffron]");
  if (!plate || !saffron || bars.length === 0) return;

  tl.set(plate, { scale: 0.6, rotation: -7, svgOrigin: "256 256" }, 0);
  bars.forEach((bar, i) => tl.set(bar, { scaleX: 0, svgOrigin: BARS[i]?.[4] ?? "120 165" }, 0));
  tl.set(saffron, { scaleX: 0, svgOrigin: "226 256" }, 0);
  tl.to(plate, { scale: 1, rotation: 0, duration: 0.7, ease: "back.out(1.8)" }, 0.05);
  tl.to(bars, { scaleX: 1, duration: 0.4, stagger: 0.1, ease: "power3.out" }, 0.62);
  tl.to(saffron, { scaleX: 1, duration: 0.45, ease: "power3.out" }, 1.02);
}
