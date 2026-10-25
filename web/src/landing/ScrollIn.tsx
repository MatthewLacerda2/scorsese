// A part of the landing page that moves in as it is scrolled to: what carries
// `data-rise` rises, on the scroll's clock (`motion.ts`). The part's time is
// how far its top has come up the window — 0 as it enters at the bottom, 1
// once it is past the middle — so scrolling back plays it backwards. A visitor
// who asked for less motion gets it all there at once.

import { type ReactNode, useCallback, useRef } from "react";
import { useLanguage } from "@/i18n/I18nProvider";
import { draw, prefersStill, SCROLLED, useOnScroll } from "@/landing/motion";

export function ScrollIn({ children }: { children: ReactNode }) {
  const { language } = useLanguage();
  const root = useRef<HTMLDivElement>(null);
  const still = prefersStill();

  const frame = useCallback(() => {
    const element = root.current;
    if (!element) return;
    const top = element.getBoundingClientRect().top;
    const t = still ? Number.POSITIVE_INFINITY : (innerHeight - top) / (innerHeight * 0.5);
    draw(element, Math.max(0, t), SCROLLED, language);
  }, [language, still]);
  useOnScroll(frame);

  return <div ref={root}>{children}</div>;
}
