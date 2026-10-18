// The arithmetic of pieces (#1448): a fixed grid, asked for from the playhead
// forward about ten seconds ahead, and how much is loaded past the playhead.

import { describe, expect, test } from "bun:test";
import {
  aheadFor,
  nextPiece,
  pieceLength,
  pieceOf,
  playedWhole,
  secondsAhead,
  short,
  stillLoaded,
} from "./plan";

const fps = { num: 30, den: 1 };

describe("pieces", () => {
  test("a cut of thirty seconds or less is played whole", () => {
    expect(playedWhole(900, fps)).toBe(true);
    expect(playedWhole(901, fps)).toBe(false);
  });

  test("are five seconds on a fixed grid, the last running to the end", () => {
    expect(pieceLength(fps)).toBe(150);
    expect(pieceLength({ num: 30000, den: 1001 })).toBe(150);
    expect(pieceOf(0, 150, 1800)).toEqual({ index: 0, start: 0, end: 150, range: "0:150" });
    expect(pieceOf(11, 150, 1800).range).toBe("1650:");
    expect(pieceOf(2, 150, 400)).toEqual({ index: 2, start: 300, end: 400, range: "300:" });
  });

  test("are asked for from the playhead forward, ten seconds ahead", () => {
    expect(nextPiece(0, 1800, fps, new Set())?.index).toBe(0);
    // Mid-piece, the piece under the playhead comes first.
    expect(nextPiece(200, 1800, fps, new Set())?.index).toBe(1);
    expect(nextPiece(200, 1800, fps, new Set([1]))?.index).toBe(2);
    // Ten seconds from frame 200 reach frame 500: pieces 1 to 3, and no more.
    expect(nextPiece(200, 1800, fps, new Set([1, 2, 3]))).toBeNull();
    expect(nextPiece(200, 1800, fps, new Set([1, 2, 3]), 20)?.index).toBe(4);
  });

  test("never ask past the end", () => {
    expect(nextPiece(1790, 1800, fps, new Set())?.range).toBe("1650:");
    expect(nextPiece(1790, 1800, fps, new Set([11]))).toBeNull();
    expect(nextPiece(5000, 1800, fps, new Set())?.index).toBe(11);
    expect(nextPiece(0, 0, fps, new Set())).toBeNull();
  });

  test("keep further ahead when they are slow to make, within a bound", () => {
    expect(aheadFor(null)).toBe(10);
    expect(aheadFor(2)).toBe(10);
    expect(aheadFor(8)).toBe(16);
    expect(aheadFor(60)).toBe(30);
  });
});

describe("what is loaded ahead", () => {
  test("runs on across pieces that meet, and stops at a gap", () => {
    expect(
      secondsAhead(
        [
          [0, 5],
          [5, 10],
          [12, 15],
        ],
        3,
      ),
    ).toBe(7);
    expect(
      secondsAhead(
        [
          [5, 10],
          [0, 5.02],
        ],
        1,
      ),
    ).toBe(9);
    expect(secondsAhead([[5, 10]], 3)).toBe(0);
  });

  test("forgets a piece the browser dropped, so it is asked for again", () => {
    const had = new Set([0, 1, 2]);
    // Pieces 0 and 2 are loaded; piece 1 was evicted, and 2 begins a frame late.
    const loaded: [number, number][] = [
      [0, 5],
      [10.03, 15],
    ];
    expect([...stillLoaded(had, loaded, 1800, fps)]).toEqual([0, 2]);
    expect(stillLoaded(had, [], 1800, fps).size).toBe(0);
    // The last piece runs only to the end of the cut.
    expect([...stillLoaded(new Set([2]), [[10, 13.4]], 400, fps)]).toEqual([2]);
  });

  test("is short under ten seconds, unless the cut ends sooner", () => {
    expect(short([[0, 5]], 0, 60)).toBe(true);
    expect(short([[0, 15]], 2, 60)).toBe(false);
    expect(short([[50, 60]], 55, 60)).toBe(false);
    expect(short([], 55, 60)).toBe(true);
  });
});
