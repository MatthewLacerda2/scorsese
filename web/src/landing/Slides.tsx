// The landing page's sections as slides (`slideLayout.ts` has the layout). The
// slides are stacked on a stage fixed to the window; under it, a track as tall
// as the scroll they take, with one snap point per slide, so the browser
// scrolls the page and finishes a crossfade that was let go of halfway.
//
// A snap point covers its slide's whole stretch, so a slide taller than the
// window scrolls freely through itself and snaps only between slides. The
// first slide plays its own entrance on the clock (`Hero`); every later one is
// drawn by its crossfade's clock. A visitor who asked for less motion gets the
// sections one under the other, as plain scrolling.

import { Children, type MouseEvent, type ReactNode, useCallback, useEffect, useRef } from "react";
import { useLanguage } from "@/i18n/I18nProvider";
import { draw, prefersStill, SCROLLED, useOnScroll } from "@/landing/motion";
import { frameAt, layout } from "@/landing/slideLayout";

export function Slides({ children }: { children: ReactNode }) {
  const { language } = useLanguage();
  const items = Children.toArray(children);
  const slides = useRef<(HTMLDivElement | null)[]>([]);
  const points = useRef<(HTMLDivElement | null)[]>([]);
  const track = useRef<HTMLDivElement>(null);
  const plan = useRef(layout([], 0));
  const still = prefersStill();

  const frame = useCallback(() => {
    const { spans, fade } = plan.current;
    spans.forEach((span, index) => {
      const slide = slides.current[index];
      if (!slide) return;
      const { shift, opacity, time } = frameAt(span, scrollY, fade);
      slide.style.transform = `translateY(${-shift}px)`;
      slide.style.opacity = String(opacity);
      slide.style.visibility = opacity > 0 ? "" : "hidden";
      slide.style.pointerEvents = opacity >= 0.5 ? "" : "none";
      if (index > 0) draw(slide, time, SCROLLED, language);
    });
  }, [language]);
  useOnScroll(frame);

  // Lay the track out again whenever a slide or the window changes size.
  useEffect(() => {
    if (still) return;
    const measure = () => {
      const heights = slides.current.map((slide) => slide?.offsetHeight ?? 0);
      plan.current = layout(heights, innerHeight);
      const { spans, total } = plan.current;
      if (track.current) track.current.style.height = `${total}px`;
      spans.forEach((span, index) => {
        const point = points.current[index];
        if (!point) return;
        point.style.top = `${span.start}px`;
        point.style.height = `${span.extra + innerHeight}px`;
      });
      frame();
    };
    measure();
    const observer = typeof ResizeObserver === "function" ? new ResizeObserver(measure) : null;
    for (const slide of slides.current) if (slide) observer?.observe(slide);
    addEventListener("resize", measure);
    document.documentElement.classList.add("landing-snap");
    return () => {
      observer?.disconnect();
      removeEventListener("resize", measure);
      document.documentElement.classList.remove("landing-snap");
    };
  }, [still, frame]);

  // An in-page link scrolls to the slide that holds its target.
  const follow = (event: MouseEvent) => {
    const link = (event.target as Element).closest<HTMLAnchorElement>('a[href^="#"]');
    const target = link && document.getElementById(link.hash.slice(1));
    const index = target ? slides.current.findIndex((slide) => slide?.contains(target)) : -1;
    const span = plan.current.spans[index];
    if (!span) return;
    event.preventDefault();
    scrollTo({ top: span.start, behavior: "smooth" });
  };

  if (still) return <>{children}</>;
  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: a delegated listener for the links inside, which stay the focusable elements.
    // biome-ignore lint/a11y/useKeyWithClickEvents: the links handle their own keys; Enter on one fires this click.
    <div onClick={follow}>
      <div className="landing-stage">
        {items.map((item, index) => (
          <div
            key={(item as { key?: string }).key ?? index}
            ref={(node) => {
              slides.current[index] = node;
            }}
            className="landing-slide"
          >
            {item}
          </div>
        ))}
      </div>
      <div ref={track} className="relative" aria-hidden>
        {items.map((item, index) => (
          <div
            key={(item as { key?: string }).key ?? index}
            ref={(node) => {
              points.current[index] = node;
            }}
            className="landing-snap-point"
          />
        ))}
      </div>
    </div>
  );
}
