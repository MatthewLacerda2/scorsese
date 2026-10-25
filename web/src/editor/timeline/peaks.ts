// A sound's peaks (#1572): the levels the server reads once per file, and the
// slice of them a clip plays, cut into the bars its waveform is drawn with.
// Pure, so trimming, zooming or scrolling only cuts the same levels again.

import { ApiError } from "@/api";
import type { ServerEvent } from "@/api/events";

/** `GET /api/library/{id}/peaks`, read: levels over time, 0–255 each. */
export interface Peaks {
  perSecond: number;
  levels: Uint8Array;
}

/** A bar of the waveform every this many pixels: two drawn, one a gap. */
const BAR_PX = 3;

/** The server's bytes: a little-endian `f32` of levels per second, then a
 * byte a level. Anything shorter than the rate is none. */
export function parsePeaks(buffer: ArrayBuffer): Peaks | undefined {
  if (buffer.byteLength < 4) return undefined;
  return {
    perSecond: new DataView(buffer).getFloat32(0, true),
    levels: new Uint8Array(buffer, 4),
  };
}

/** The most bars one clip is drawn with, however long or close up: a
 * ten-minute bed at the closest zoom would otherwise be 64,000 of them. */
export const MOST_BARS = 4096;

/** How many bars `length` seconds of `peaks` drawn `width` pixels wide get:
 * one every few pixels, never more than the levels the slice holds, and never
 * more than {@link MOST_BARS}. At least one. */
export function barsOf(peaks: Peaks, length: number, width: number): number {
  const levels = Math.ceil(length * peaks.perSecond - 1e-9);
  return Math.max(1, Math.min(Math.floor(width / BAR_PX), levels, MOST_BARS));
}

/**
 * `bars` levels, 0–1, for the `length` seconds of the sound from `from`: each
 * the loudest level its stretch touches, silence past the end of the file.
 */
export function sliceOf(peaks: Peaks, from: number, length: number, bars: number): number[] {
  const each = length / bars;
  const rate = peaks.perSecond;
  return Array.from({ length: bars }, (_, bar) => {
    const first = Math.floor((from + bar * each) * rate + 1e-9);
    const end = Math.max(first + 1, Math.ceil((from + (bar + 1) * each) * rate - 1e-9));
    let loudest = 0;
    for (let at = Math.max(0, first); at < Math.min(end, peaks.levels.length); at++) {
      loudest = Math.max(loudest, peaks.levels[at] ?? 0);
    }
    return loudest / 255;
  });
}

/**
 * Whether `event` says to ask for library item `item`'s peaks again: the
 * thumbnail job that reads them is done, or the stream reconnected and one
 * may have been missed. A sound queued behind a batch of uploads is read
 * minutes later; waiting on its job, rather than retrying to a stop, means
 * its waveform still arrives.
 */
export function readAgain(event: ServerEvent, item: number | undefined): boolean {
  if (item === undefined) return false;
  if (event.type === "resync") return true;
  return (
    event.type === "job" &&
    event.kind === "thumbnail" &&
    event.state === "done" &&
    event.result?.item === item
  );
}

/** Retry a failed read of peaks a few times — except a `404`, which says they
 * are still being read and is answered by {@link readAgain}. */
export function retryPeaks(count: number, error: unknown): boolean {
  if (error instanceof ApiError && error.status === 404) return false;
  return count < 3;
}
