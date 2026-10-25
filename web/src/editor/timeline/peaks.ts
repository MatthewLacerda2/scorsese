// A sound's peaks (#1572): the levels the server reads once per file, and the
// slice of them a clip plays, cut into the bars its waveform is drawn with.
// Pure, so trimming, zooming or scrolling only cuts the same levels again.

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

/** How many bars a clip `width` pixels wide is drawn with. */
export function barsOf(width: number): number {
  return Math.max(1, Math.floor(width / BAR_PX));
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
