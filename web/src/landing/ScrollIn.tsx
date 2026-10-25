// A part of the landing page that moves in as it is scrolled to: what carries
// `data-rise` rises, on the scroll's clock (`motion.ts`). The part's time is
// how far its top has come up the window — 0 as it enters at the bottom, 1
// once it is past the middle — so scrolling back plays it backwards. A visitor
// who asked for less motion gets it all there at once. Each is a section the
// A/B tests count the visit's depth by (`data-ab-section`, `ab/useLandingTests.ts`).

import { type ReactNode, useCallback, useRef } from "react";
import { draw, SCROLLED, useOnScroll } from "@/landing/motion";
import { prefersStill } from "@/lib/reducedMotion";

export function ScrollIn({ children }: { children: ReactNode }) {
  const root = useRef<HTMLDivElement>(null);
  const still = prefersStill();

  const frame = useCallback(() => {
    const element = root.current;
    if (!element) return;
    const top = element.getBoundingClientRect().top;
    const t = still ? Number.POSITIVE_INFINITY : (innerHeight - top) / (innerHeight * 0.5);
    draw(element, Math.max(0, t), SCROLLED);
  }, [still]);
  useOnScroll(frame);

  return (
    <div ref={root} data-ab-section>
      {children}
    </div>
  );
}
