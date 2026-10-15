"use client";

import { useEffect, useRef, type ElementType, type ReactNode } from "react";

/** Adds `in` once the element scrolls into view (CSS does the motion, and does none at
 * all under reduced motion). One observer per element, disconnected after it fires. */
export function Reveal({
  as: Tag = "div",
  className = "",
  children,
  threshold = 0.25,
  style,
}: {
  as?: ElementType;
  className?: string;
  children: ReactNode;
  threshold?: number;
  style?: React.CSSProperties;
}) {
  const ref = useRef<HTMLElement>(null);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    if (!("IntersectionObserver" in window)) {
      el.classList.add("in");
      return;
    }
    const watch = new IntersectionObserver(
      ([entry]) => {
        if (entry?.isIntersecting) {
          el.classList.add("in");
          watch.disconnect();
        }
      },
      { threshold },
    );
    watch.observe(el);
    return () => watch.disconnect();
  }, [threshold]);
  return (
    <Tag ref={ref} className={className} style={style}>
      {children}
    </Tag>
  );
}
