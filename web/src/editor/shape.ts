// The frame's shape is the project's own (#1278): the document's `aspect`,
// `across:down`, which the preview is drawn in and every render defaults to.
// Changing it is `project_reframe`, never a setting kept in the browser — the
// re-fit sets footage that filled the old frame to fill the new one, and says
// what it could not settle.

import type { ProjectDocument } from "@/api/document";

/** The shapes the editor offers: the ones a person with an idea reaches for. */
export const ASPECTS = ["16:9", "9:16", "1:1", "4:5"] as const;

/** What a document that names no shape is: `scorsese_core::Ratio::WIDE`. */
export const WIDE = "16:9";

/** The project's shape, `across:down`. */
export function aspectOf(document: Pick<ProjectDocument, "aspect">): string {
  return document.aspect ?? WIDE;
}

/** The shapes to offer for a project now in `aspect`: the usual four, and its own if it is none of them. */
export function offered(aspect: string): string[] {
  return (ASPECTS as readonly string[]).includes(aspect) ? [...ASPECTS] : [...ASPECTS, aspect];
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
