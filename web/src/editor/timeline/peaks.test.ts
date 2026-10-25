// A sound's peaks (#1572): read from the bytes the server sends, and cut to
// the slice a clip plays, one level per bar.

import { describe, expect, test } from "bun:test";
import { ApiError } from "@/api";
import type { ServerEvent } from "@/api/events";
import { barsOf, MOST_BARS, parsePeaks, readAgain, retryPeaks, sliceOf } from "./peaks";

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
    const peaks = { perSecond: 40, levels: new Uint8Array(4000) };
    expect(barsOf(peaks, 10, 90)).toBe(30);
    expect(barsOf(peaks, 10, 2)).toBe(1);
  });

  test("never more bars than the levels the slice holds", () => {
    const peaks = { perSecond: 4, levels: new Uint8Array(8) };
    // Two seconds at four levels a second, drawn 300 px wide: eight bars.
    expect(barsOf(peaks, 2, 300)).toBe(8);
    expect(barsOf(peaks, 0.1, 300)).toBe(1);
  });

  test("never more than a ceiling, however long the clip or close the zoom", () => {
    // Ten minutes at the closest zoom: 192,000 px and 24,000 levels.
    const peaks = { perSecond: 40, levels: new Uint8Array(24_000) };
    expect(barsOf(peaks, 600, 192_000)).toBe(MOST_BARS);
  });
});

describe("waiting for a sound's peaks", () => {
  const done = (item: number, kind = "thumbnail", state = "done") =>
    ({
      type: "job",
      id: 1,
      kind,
      state,
      attempts: 1,
      result: { item },
      error: null,
      created_at: 0,
      started_at: 0,
      finished_at: 0,
      interrupted_at: null,
    }) as ServerEvent;

  test("asks again when the job reading this item's peaks is done", () => {
    expect(readAgain(done(9), 9)).toBe(true);
  });

  test("and after the stream reconnects, when an event may have been missed", () => {
    expect(readAgain({ type: "resync" }, 9)).toBe(true);
  });

  test("not for another item's job, another kind, or one still running", () => {
    expect(readAgain(done(8), 9)).toBe(false);
    expect(readAgain(done(9, "proxy"), 9)).toBe(false);
    expect(readAgain(done(9, "thumbnail", "running"), 9)).toBe(false);
    expect(readAgain(done(9), undefined)).toBe(false);
  });

  test("a 404 is waited out on the job's event, never retried to a stop", () => {
    expect(retryPeaks(0, new ApiError(404, "pending"))).toBe(false);
    expect(retryPeaks(0, new ApiError(500, "down"))).toBe(true);
    expect(retryPeaks(3, new ApiError(500, "down"))).toBe(false);
  });
});
