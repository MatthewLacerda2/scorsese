// The landing page's sections as slides (`slideLayout.ts` has the layout). The
// slides are stacked on a stage fixed to the window; under it, an empty track
// as tall as the scroll they take, so the browser's own scrolling (wheel,
// touch, keys, scrollbar) drives them. When the scroll comes to rest inside a
// crossfade, `useSettle` glides it to an end, slowly enough to be watched,
// rather than leaving two slides half drawn.
//
// The slides are laid out against the window's *largest* height (`100lvh`),
// never `innerHeight`: on a phone the address bar slides away and back as the
// page scrolls, changing `innerHeight` mid-gesture, and laying out again then
// moved every slide under the visitor's finger.
//
// The first slide plays its own entrance on the clock (`Hero`); every later
// one is drawn by its crossfade's clock. A visitor who asked for less motion
// gets the sections one under the other, as plain scrolling.

import {
  Children,
  type MouseEvent,
  type ReactNode,
  type RefObject,
  useCallback,
  useEffect,
  useRef,
} from "react";
import { useLanguage } from "@/i18n/I18nProvider";
import { draw, prefersStill, SCROLLED, useOnScroll } from "@/landing/motion";
import { frameAt, layout, restingPoint, smooth } from "@/landing/slideLayout";

type Plan = ReturnType<typeof layout>;

/** How long the glide to a crossfade's end takes, in milliseconds. */
const GLIDE_MS = 900;
/** How long the scroll must be still before it counts as let go of. */
const REST_MS = 140;

/** Glides the page to `top` over `GLIDE_MS`; returns a function that stops it. */
function glide(top: number, done: () => void): () => void {
  const from = scrollY;
  const began = performance.now();
  let id = requestAnimationFrame(function step(now) {
    const progress = Math.min(1, (now - began) / GLIDE_MS);
    scrollTo(0, from + (top - from) * smooth(progress));
    if (progress < 1) id = requestAnimationFrame(step);
    else done();
  });
  return () => cancelAnimationFrame(id);
}

/** Finishes a crossfade the visitor let go of partway, onwards or back. */
function useSettle(plan: RefObject<Plan>, enabled: boolean) {
  useEffect(() => {
    if (!enabled) return;
    let last = scrollY;
    let direction = 1;
    let timer = 0;
    let touching = false;
    let stop: (() => void) | null = null;
    const settle = () => {
      if (touching || stop) return;
      const { spans, fade } = plan.current;
      const target = restingPoint(spans, fade, scrollY, direction);
      if (target === null) return;
      stop = glide(target, () => {
        stop = null;
      });
    };
    const onScroll = () => {
      if (scrollY !== last) direction = Math.sign(scrollY - last);
      last = scrollY;
      if (stop) return;
      clearTimeout(timer);
      timer = window.setTimeout(settle, REST_MS);
    };
    // The visitor's hand always wins over the glide.
    const interrupt = () => {
      stop?.();
      stop = null;
    };
    const onTouchStart = () => {
      touching = true;
      interrupt();
    };
    const onTouchEnd = () => {
      touching = false;
      onScroll();
    };
    addEventListener("scroll", onScroll, { passive: true });
    addEventListener("wheel", interrupt, { passive: true });
    addEventListener("keydown", interrupt);
    addEventListener("touchstart", onTouchStart, { passive: true });
    addEventListener("touchend", onTouchEnd, { passive: true });
    return () => {
      interrupt();
      clearTimeout(timer);
      removeEventListener("scroll", onScroll);
      removeEventListener("wheel", interrupt);
      removeEventListener("keydown", interrupt);
      removeEventListener("touchstart", onTouchStart);
      removeEventListener("touchend", onTouchEnd);
    };
  }, [plan, enabled]);
}

export function Slides({ children }: { children: ReactNode }) {
  const { language } = useLanguage();
  const items = Children.toArray(children);
  const slides = useRef<(HTMLDivElement | null)[]>([]);
  const track = useRef<HTMLDivElement>(null);
  const tall = useRef<HTMLDivElement>(null);
  const plan = useRef<Plan>(layout([], 0));
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
  useSettle(plan, !still);

  // Lay the track out again whenever a slide or the window changes size.
  useEffect(() => {
    if (still) return;
    const measure = () => {
      const heights = slides.current.map((slide) => slide?.offsetHeight ?? 0);
      plan.current = layout(heights, tall.current?.offsetHeight || innerHeight);
      if (track.current) track.current.style.height = `${plan.current.total}px`;
      frame();
    };
    measure();
    const observer = typeof ResizeObserver === "function" ? new ResizeObserver(measure) : null;
    for (const slide of slides.current) if (slide) observer?.observe(slide);
    addEventListener("resize", measure);
    return () => {
      observer?.disconnect();
      removeEventListener("resize", measure);
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
      <div ref={track} aria-hidden />
      <div ref={tall} className="landing-tall" aria-hidden />
    </div>
  );
}
