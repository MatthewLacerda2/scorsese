// A sound's peaks (#1572): read from the bytes the server sends, and cut to
// the slice a clip plays, one level per bar.

import { describe, expect, test } from "bun:test";
import { barsOf, parsePeaks, sliceOf } from "./peaks";

function bytes(perSecond: number, levels: number[]): ArrayBuffer {
  const buffer = new ArrayBuffer(4 + levels.length);
  new DataView(buffer).setFloat32(0, perSecond, true);
  new Uint8Array(buffer, 4).set(levels);
  return buffer;
}

describe("a sound's peaks", () => {
  test("are the rate, then a byte a level", () => {
    const peaks = parsePeaks(bytes(40, [0, 128, 255]));
    expect(peaks?.perSecond).toBe(40);
    expect([...(peaks?.levels ?? [])]).toEqual([0, 128, 255]);
  });

  test("too short to hold a rate are none", () => {
    expect(parsePeaks(new ArrayBuffer(3))).toBeUndefined();
  });

  test("a bar is the loudest level in its stretch", () => {
    const peaks = { perSecond: 4, levels: Uint8Array.from([0, 51, 255, 102, 0, 0, 204, 0]) };
    // Two seconds from the start in two bars: a second, four levels, each.
    expect(sliceOf(peaks, 0, 2, 2)).toEqual([1, 0.8]);
  });

  test("a trimmed clip draws only the slice it plays", () => {
    const peaks = { perSecond: 4, levels: Uint8Array.from([0, 51, 255, 102, 0, 0, 204, 0]) };
    // In at one second, half a second long, one bar per level.
    expect(sliceOf(peaks, 1, 0.5, 2)).toEqual([0, 0]);
    expect(sliceOf(peaks, 0.25, 0.5, 2)).toEqual([0.2, 1]);
  });

  test("a bar narrower than a level repeats it, and past the end is silent", () => {
    const peaks = { perSecond: 2, levels: Uint8Array.from([255, 51]) };
    expect(sliceOf(peaks, 0, 1, 4)).toEqual([1, 1, 0.2, 0.2]);
    expect(sliceOf(peaks, 0.5, 1, 4)).toEqual([0.2, 0.2, 0, 0]);
  });

  test("a clip gets a bar every three pixels, at least one", () => {
    expect(barsOf(90)).toBe(30);
    expect(barsOf(2)).toBe(1);
  });
});
