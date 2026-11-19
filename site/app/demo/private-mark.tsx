"use client";

import { useScene, type SceneTimeline } from "./engine";

const ARCS = [22, 44, 66].map(arc);

/** Upper arc centred on the dot, from the right side through the top to the left. */
function arc(r: number) {
  const a0 = (38 * Math.PI) / 180;
  const a1 = (142 * Math.PI) / 180;
  const x0 = 80 + r * Math.cos(a0);
  const y0 = 116 - r * Math.sin(a0);
  const x1 = 80 + r * Math.cos(a1);
  const y1 = 116 - r * Math.sin(a1);
  return `M ${x0.toFixed(2)} ${y0.toFixed(2)} A ${r} ${r} 0 0 0 ${x1.toFixed(2)} ${y1.toFixed(2)}`;
}

export function PrivateMark() {
  const ref = useScene(buildMark, { once: true, threshold: 0.45 });
  return (
    <div ref={ref} className="mb-6 h-40 w-40" aria-hidden="true">
      <svg data-wifi="" width="160" height="160" viewBox="0 0 160 160" className="overflow-visible">
        <g fill="none" stroke="var(--text)" strokeWidth="16" strokeLinecap="round">
          <circle data-stroke="" cx="80" cy="116" r="2" pathLength="1" strokeDasharray="1" strokeDashoffset="1" />
          {ARCS.map((d) => (
            <path key={d} data-stroke="" d={d} pathLength="1" strokeDasharray="1" strokeDashoffset="1" />
          ))}
        </g>
        <path
          data-halo=""
          d="M126 36 L38 126"
          fill="none"
          stroke="var(--bg)"
          strokeWidth="26"
          strokeLinecap="round"
          pathLength="1"
          strokeDasharray="1"
          strokeDashoffset="1"
        />
        <path
          data-strike=""
          d="M126 36 L38 126"
          fill="none"
          stroke="var(--saffron)"
          strokeWidth="10"
          strokeLinecap="round"
          pathLength="1"
          strokeDasharray="1"
          strokeDashoffset="1"
        />
      </svg>
    </div>
  );
}

function buildMark(tl: SceneTimeline, root: HTMLElement) {
  const ink = [...root.querySelectorAll<SVGGeometryElement>("[data-stroke]")];
  const halo = root.querySelector<SVGGeometryElement>("[data-halo]");
  const strike = root.querySelector<SVGGeometryElement>("[data-strike]");
  const svg = root.querySelector<SVGSVGElement>("[data-wifi]");
  if (!halo || !strike || !svg || ink.length === 0) return;

  tl.set(ink, { strokeDashoffset: 1 }, 0);
  tl.set([halo, strike], { strokeDashoffset: 1 }, 0);
  tl.to(ink, { strokeDashoffset: 0, duration: 0.45, stagger: 0.12, ease: "power2.inOut" }, 0.1);
  tl.to([halo, strike], { strokeDashoffset: 0, duration: 0.4, ease: "power3.in" }, 0.85);
  tl.fromTo(svg, { x: 0 }, { x: 2, duration: 0.05, repeat: 5, yoyo: true, ease: "power1.inOut" }, 1.3);
}
