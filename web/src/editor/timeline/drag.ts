// What a drag on the timeline proposes, what it snaps to, and the tool call it
// becomes — the desktop app's `app/src/timeline/drag/` and `drop.rs`, as plain
// functions so they are tested without a pointer.
//
// Nothing here decides whether an edit is *legal*: two clips overlapping, a
// trim past the end of the footage, a title on a music track — the server's
// tools refuse those with `core`'s own reasons. This only works out what the
// hand asked for, clamped to what the gesture itself can mean (a clip does not
// start before the timeline, a trim does not invent source nobody measured).

import type { Clip, DocumentAsset, Fps, ProjectDocument } from "@/api";
import { toFrames, toSeconds } from "./time";

/** Which part of a clip the pointer took hold of. */
export type Handle = "body" | "left" | "right";

/** Where a clip sits: in timeline frames, as the document stores it. */
export interface Shape {
  start: number;
  duration: number;
  sourceIn: number;
}

/** How far a trim may go: frames of source before the in-point and after the
 * out-point. `null` is unbounded — nothing measured says where the source ends. */
export interface Limits {
  head: number | null;
  tail: number | null;
}

/** The kinds that play a stretch of source, so a trim can run out of it. */
const TIMED = new Set(["video", "audio", "generated_video", "generated_audio", "synth_audio"]);

/** A clip placed on nothing measured runs this long: a title, a still. */
export const UNMEASURED_SECONDS = 5;

/** How close, in pixels, an edge has to come to something to snap onto it. */
export const SNAP_PX = 8;

export function shapeOf(clip: Clip): Shape {
  return { start: clip.start, duration: clip.duration, sourceIn: clip.source_in ?? 0 };
}

/** An asset's own length on the grid, when something measured it. */
export function lengthOf(asset: DocumentAsset | undefined, fps: Fps): number | null {
  const seconds = asset?.media?.duration_seconds;
  return seconds && seconds > 0 ? toFrames(seconds, fps) : null;
}

/** How far each edge of `clip` may be trimmed back out, read once when a drag begins. */
export function limitsOf(clip: Clip, asset: DocumentAsset | undefined, fps: Fps): Limits {
  // An asset that resolves to nothing is treated as timed: the cautious
  // reading refuses a trim that might invent source.
  const timed = !asset || TIMED.has(asset.kind);
  const length = lengthOf(asset, fps);
  const speed = clip.speed ?? 1;
  const used = (clip.source_in ?? 0) + Math.round(clip.duration * speed);
  return {
    head: timed ? (clip.source_in ?? 0) : null,
    tail: length === null ? null : Math.max(0, length - used),
  };
}

/** The shape a drag of `delta` frames on `handle` proposes. */
export function propose(clip: Clip, handle: Handle, delta: number, limits: Limits): Shape {
  const shape = shapeOf(clip);
  const speed = clip.speed ?? 1;
  if (handle === "body") return { ...shape, start: Math.max(0, shape.start + delta) };
  if (handle === "right") {
    const forward = limits.tail === null ? Infinity : Math.floor(limits.tail / speed);
    return { ...shape, duration: Math.max(1, shape.duration + Math.min(delta, forward)) };
  }
  const back =
    limits.head === null ? shape.start : Math.min(Math.floor(limits.head / speed), shape.start);
  const moved = Math.min(Math.max(delta, -back), shape.duration - 1);
  const consumed = Math.sign(moved) * Math.round(Math.abs(moved) * speed);
  return {
    start: shape.start + moved,
    duration: shape.duration - moved,
    sourceIn: Math.max(0, shape.sourceIn + consumed),
  };
}

/** The edges of a shape that snap, for the handle being dragged. */
export function edges(shape: Shape, handle: Handle): number[] {
  const end = shape.start + shape.duration;
  if (handle === "left") return [shape.start];
  if (handle === "right") return [end];
  return [shape.start, end];
}

/** Everywhere a drag may come to rest: the playhead, the head of the
 * timeline, and both edges of every clip but the one moving. The playhead
 * comes first, so it wins a tie — it was put there on purpose. */
export function targets(document: ProjectDocument, playhead: number, moving?: string): number[] {
  const at = [playhead, 0];
  for (const track of document.tracks ?? []) {
    for (const clip of track.clips) {
      if (clip.id === moving) continue;
      at.push(clip.start, clip.start + clip.duration);
    }
  }
  return at;
}

/** How far to shift so the nearest of `from` lands on a target within
 * `reach` frames; `null` when nothing is that close. */
export function snap(at: number[], from: number[], reach: number): number | null {
  let best: number | null = null;
  for (const edge of from) {
    for (const target of at) {
      const shift = target - edge;
      if (Math.abs(shift) > reach) continue;
      if (best === null || Math.abs(shift) < Math.abs(best)) best = shift;
    }
  }
  return best;
}

/** A drag, snapped: the proposal, shifted onto the nearest target when one is in reach. */
export function snapped(
  clip: Clip,
  handle: Handle,
  delta: number,
  limits: Limits,
  at: number[],
  reach: number,
): Shape {
  const free = propose(clip, handle, delta, limits);
  const shift = snap(at, edges(free, handle), reach);
  return shift === null ? free : propose(clip, handle, delta + shift, limits);
}

/** `trim_clip`'s arguments for moving `clip` to `shape`: only the fields that
 * changed, in seconds — a start alone moves it and leaves the rest. `null`
 * when the drag ended where it began. */
export function trimArguments(clip: Clip, shape: Shape, fps: Fps): Record<string, unknown> | null {
  const was = shapeOf(clip);
  const args: Record<string, unknown> = { clip: clip.id };
  if (shape.start !== was.start) args.start_seconds = toSeconds(shape.start, fps);
  if (shape.duration !== was.duration) args.duration_seconds = toSeconds(shape.duration, fps);
  if (shape.sourceIn !== was.sourceIn) args.source_in_seconds = toSeconds(shape.sourceIn, fps);
  return Object.keys(args).length > 1 ? args : null;
}

/** A drag let go, as the one tool call it becomes. */
export interface Release {
  tool: "trim_clip" | "clip_move";
  args: Record<string, unknown>;
}

/** What a drag of `clip` from lane `from`, let go over lane `onto` in `shape`,
 * asks the server for. On another lane it is one `clip_move` carrying the new
 * start — never a move and then a trim, which would pass through a document
 * nobody asked for; on its own lane it is `trim_clip` with what changed.
 * `null` when the drag ended where it began. */
export function toolCall(
  clip: Clip,
  from: string,
  onto: string,
  shape: Shape,
  fps: Fps,
): Release | null {
  if (onto !== from) {
    const args = { clip: clip.id, track: onto, start_seconds: toSeconds(shape.start, fps) };
    return { tool: "clip_move", args };
  }
  const args = trimArguments(clip, shape, fps);
  return args && { tool: "trim_clip", args };
}

/** How long a dropped asset runs: its own measured length, or five seconds. */
export function dropLength(asset: DocumentAsset | undefined, fps: Fps): number {
  return lengthOf(asset, fps) ?? toFrames(UNMEASURED_SECONDS, fps);
}

/** Where an asset dropped at frame `pointed` lands, snapped by either edge. */
export function dropStart(pointed: number, length: number, at: number[], reach: number): number {
  const shift = snap(at, [pointed, pointed + length], reach);
  return Math.max(0, pointed + (shift ?? 0));
}

/** `place_clip`'s arguments for `asset` landing on `track` at `start`. A
 * measured asset runs the rest of its source; anything else gets a length. */
export function placeArguments(
  asset: DocumentAsset | undefined,
  assetId: string,
  track: string,
  start: number,
  fps: Fps,
): Record<string, unknown> {
  const args: Record<string, unknown> = {
    asset: assetId,
    track,
    start_seconds: toSeconds(start, fps),
  };
  if (lengthOf(asset, fps) === null) args.duration_seconds = UNMEASURED_SECONDS;
  return args;
}

/** Whether `kind` goes on a video track (picture) or an audio one (sound). */
export function laneFor(kind: string): "video" | "audio" {
  return kind === "audio" || kind === "generated_audio" || kind === "synth_audio"
    ? "audio"
    : "video";
}
