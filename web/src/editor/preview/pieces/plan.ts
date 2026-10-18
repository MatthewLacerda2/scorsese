// What a preview played in pieces asks for next (#1448): the arithmetic of
// pieces, kept apart from the player so it can be read and tested alone.
//
// The cut is cut on a fixed grid — piece k is frames k·n up to (k+1)·n, n
// being about five seconds of frames — so a piece is the same range however
// the playhead reached it, and seeking back finds the one already made
// (the server keeps it by revision, quality and range).

import type { Fps } from "@/api";
import { rate, toSeconds } from "../../timeline/time";

/** How long a piece is: short enough to land soon, long enough to be worth a job. */
const PIECE_SECONDS = 5;

/** How much is kept loaded past the playhead while playing. */
const AHEAD_SECONDS = 10;

/** The most it stretches to when pieces are slow to make. */
const MOST_AHEAD_SECONDS = 30;

/** A cut this long or shorter is previewed whole, in one go. */
const WHOLE_SECONDS = 30;

/** Whether a cut of `total` frames is previewed whole rather than in pieces. */
export function playedWhole(total: number, fps: Fps): boolean {
  return toSeconds(total, fps) <= WHOLE_SECONDS;
}

/** How many frames a piece is at `fps`. */
export function pieceLength(fps: Fps): number {
  return Math.max(1, Math.round(PIECE_SECONDS * rate(fps)));
}

/** One piece: its index on the grid, its frames, and the range a render is asked for. */
export interface Piece {
  index: number;
  start: number;
  end: number;
  /** `start:end`, or `start:` for the last, so it runs to wherever the cut ends. */
  range: string;
}

/** Piece `index` of a cut `total` frames long, cut every `length` frames. */
export function pieceOf(index: number, length: number, total: number): Piece {
  const start = index * length;
  const end = Math.min(start + length, total);
  return { index, start, end, range: end >= total ? `${start}:` : `${start}:${end}` };
}

/**
 * The first piece not yet `had` among those covering `ahead` seconds from
 * frame `at` — the one to ask for next — or `null` when every one is in hand.
 */
export function nextPiece(
  at: number,
  total: number,
  fps: Fps,
  had: ReadonlySet<number>,
  ahead = AHEAD_SECONDS,
): Piece | null {
  if (total <= 0) return null;
  const length = pieceLength(fps);
  const from = Math.floor(Math.min(Math.max(0, at), total - 1) / length);
  const until = Math.floor((Math.min(at + ahead * rate(fps), total) - 1) / length);
  for (let index = from; index <= Math.max(from, until); index++)
    if (!had.has(index)) return pieceOf(index, length, total);
  return null;
}

/** How far ahead to keep loaded, given how long the last piece took to make. */
export function aheadFor(lastTookSeconds: number | null): number {
  if (lastTookSeconds === null) return AHEAD_SECONDS;
  return Math.min(MOST_AHEAD_SECONDS, Math.max(AHEAD_SECONDS, 2 * lastTookSeconds));
}

/** A frame's worth of slack: where two loaded stretches meet, they are one. */
const JOIN_SECONDS = 0.05;

/**
 * How many seconds are loaded without a break from `at`, given the loaded
 * stretches as `[start, end]` seconds — 0 when `at` itself is not loaded.
 */
export function secondsAhead(loaded: readonly (readonly [number, number])[], at: number): number {
  const sorted = [...loaded].sort((a, b) => a[0] - b[0]);
  let reach: number | null = null;
  for (const [start, end] of sorted) {
    if (reach === null) {
      if (start <= at + JOIN_SECONDS && end > at) reach = end;
    } else if (start <= reach + JOIN_SECONDS) reach = Math.max(reach, end);
  }
  return reach === null ? 0 : reach - at;
}

/**
 * Whether the player is short of picture: less than `AHEAD_SECONDS` loaded
 * past `at` seconds, where the cut (`totalSeconds` long) has that much left.
 */
export function short(
  loaded: readonly (readonly [number, number])[],
  at: number,
  totalSeconds: number,
): boolean {
  const wanted = Math.min(AHEAD_SECONDS, totalSeconds - at) - JOIN_SECONDS;
  return secondsAhead(loaded, at) < wanted;
}
