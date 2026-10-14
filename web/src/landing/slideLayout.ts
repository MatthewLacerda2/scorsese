// The landing page as a run of slides: the scroll no longer carries a section
// up and off the screen, it holds it in place and cross-fades it into the
// next. Each slide owns a stretch of the scroll: first as much as it needs to
// show all of itself (nothing, when it fits the window), then one crossfade
// into the next slide. The page snaps (`Slides.tsx`), so a crossfade let go
// of halfway finishes on its own, forwards or back.
//
// Everything here is a pure function of the slides' heights and the scroll
// position, so any scroll position is a frame of its own.

/** How much scroll, in windows, one crossfade takes. */
export const CROSSFADE = 0.6;

/** One slide's stretch of the scroll. */
export interface Span {
  /** The scroll position where it is first whole and at its top. */
  start: number;
  /** How far it scrolls within itself, when it is taller than the window. */
  extra: number;
}

/** Each slide's stretch, and the height of the scroll that holds them all. */
export function layout(heights: readonly number[], viewport: number) {
  const fade = viewport * CROSSFADE;
  let at = 0;
  const spans: Span[] = heights.map((height) => {
    const span = { start: at, extra: Math.max(0, height - viewport) };
    at += span.extra + fade;
    return span;
  });
  const last = spans.at(-1);
  const total = last ? last.start + last.extra + viewport : 0;
  return { spans, total, fade };
}

/** One slide as it stands at scroll position `y`. */
export interface SlideFrame {
  /** How far it has scrolled up within itself, in pixels. */
  shift: number;
  opacity: number;
  /** Its entrance's clock: 0 as it starts to fade in, 1 once it is whole. */
  time: number;
}

const unit = (value: number) => Math.max(0, Math.min(1, value));

/** Where slide `span` is at scroll position `y`, with crossfades `fade` long. */
export function frameAt(span: Span, y: number, fade: number): SlideFrame {
  const local = y - span.start;
  const shift = Math.max(0, Math.min(local, span.extra));
  const opacity =
    fade <= 0 ? 1 : local < 0 ? unit(1 + local / fade) : unit(1 - (local - span.extra) / fade);
  const time = fade <= 0 ? 1 : 1 + Math.min(local, 0) / fade;
  return { shift, opacity, time };
}
