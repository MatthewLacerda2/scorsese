// The frame's shape is the project's own (#1278): the document's `aspect`,
// `across:down`, which the preview is drawn in and every render defaults to.
// Changing it is the assistant's `project_reframe` (#1531), never a setting
// kept in the browser.

import type { ProjectDocument } from "@/api/document";

/** What a document that names no shape is: `scorsese_core::Ratio::WIDE`. */
const WIDE = "16:9";

/** The project's shape, `across:down`. */
export function aspectOf(document: Pick<ProjectDocument, "aspect">): string {
  return document.aspect ?? WIDE;
}

/**
 * The sizes a film in `aspect` is delivered at, `WIDTHxHEIGHT`, the default
 * first: the shorter side at 1080 lines (`Ratio::delivery`), then at 720,
 * the longer side rounded to even as `scorsese_core::frame::even` rounds it.
 */
export function deliveries(aspect: string): [string, ...string[]] {
  const [across, down] = parts(aspect);
  const size = (lines: number) =>
    down > across
      ? `${lines}x${even((lines * down) / across)}`
      : `${even((lines * across) / down)}x${lines}`;
  return [size(1080), size(720)];
}

/** A `width`×`height` picture's shape in lowest terms — `1080x1920` is `9:16`. */
export function aspectOfSize(width: number, height: number): string {
  const divisor = gcd(width, height);
  return `${width / divisor}:${height / divisor}`;
}

/** `across:down` as numbers; anything unreadable is the default's. */
function parts(aspect: string): [number, number] {
  const [across, down] = aspect.split(":").map(Number);
  return across && down ? [across, down] : [16, 9];
}

function even(side: number): number {
  return Math.max(1, Math.round(side / 2)) * 2;
}

function gcd(a: number, b: number): number {
  return b === 0 ? a : gcd(b, a % b);
}
