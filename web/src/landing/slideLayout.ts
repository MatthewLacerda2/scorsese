// The landing page as a run of slides: the scroll no longer carries a section
// up and off the screen, it holds it in place and cross-fades it into the
// next. Each slide owns a stretch of the scroll: first as much as it needs to
// show all of itself (nothing, when it fits the window), then one crossfade
// into the next slide. A crossfade let go of partway glides to an end on its
// own (`restingPoint`, played by `Slides.tsx`): onwards once it is a quarter
// of the way in the direction it was going, back otherwise.
//
// Everything here is a pure function of the slides' heights and the scroll
// position, so any scroll position is a frame of its own.

/** How much scroll, in windows, one crossfade takes. */
export const CROSSFADE = 0.8;

/** How far into a crossfade, in the direction of travel, it commits to going on. */
export const COMMIT = 0.25;

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
/** Eases a 0..1 value in and out, so a fade starts and ends gently. */
export const smooth = (value: number) => value * value * (3 - 2 * value);

/** Where slide `span` is at scroll position `y`, with crossfades `fade` long. */
export function frameAt(span: Span, y: number, fade: number): SlideFrame {
  const local = y - span.start;
  const shift = Math.max(0, Math.min(local, span.extra));
  const linear =
    fade <= 0 ? 1 : local < 0 ? unit(1 + local / fade) : unit(1 - (local - span.extra) / fade);
  const opacity = smooth(linear);
  const time = fade <= 0 ? 1 : 1 + Math.min(local, 0) / fade;
  return { shift, opacity, time };
}

/**
 * Where the scroll should come to rest from `y`, having last moved in
 * `direction` (+1 down, -1 up): the near end of the crossfade it stopped in,
 * or `null` when it stopped on a slide.
 */
export function restingPoint(
  spans: readonly Span[],
  fade: number,
  y: number,
  direction: number,
): number | null {
  for (const [index, span] of spans.entries()) {
    const next = spans[index + 1];
    const end = span.start + span.extra;
    if (!next || fade <= 0 || y <= end || y >= next.start) continue;
    const progress = (y - end) / fade;
    const onwards = direction >= 0 ? progress >= COMMIT : progress > 1 - COMMIT;
    return onwards ? next.start : end;
  }
  return null;
}
