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
export interface Extent {
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

/** The kinds that play a stretch of source, so a trim can run out of it. An
 * image sequence's head can; its tail never runs out — it loops or holds. */
const TIMED = new Set([
  "video",
  "audio",
  "image_sequence",
  "generated_video",
  "generated_audio",
  "synth_audio",
]);

/** The kinds with no length by nature — `core`'s `AssetKind::is_timeless`. */
const TIMELESS = new Set(["image", "generated_image", "html", "text", "color", "shape", "icon"]);

/** How long a placement that says nothing gives a timeless asset — `core`'s
 * `placing::HOLD_SECONDS`, which the server applies, not this. */
export const HOLD_SECONDS = 5;

/** How close, in pixels, an edge has to come to something to snap onto it. */
export const SNAP_PX = 8;

export function extentOf(clip: Clip): Extent {
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

/** The extent a drag of `delta` frames on `handle` proposes. */
export function propose(clip: Clip, handle: Handle, delta: number, limits: Limits): Extent {
  const extent = extentOf(clip);
  const speed = clip.speed ?? 1;
  if (handle === "body") return { ...extent, start: Math.max(0, extent.start + delta) };
  if (handle === "right") {
    const forward = limits.tail === null ? Infinity : Math.floor(limits.tail / speed);
    return { ...extent, duration: Math.max(1, extent.duration + Math.min(delta, forward)) };
  }
  const back =
    limits.head === null ? extent.start : Math.min(Math.floor(limits.head / speed), extent.start);
  const moved = Math.min(Math.max(delta, -back), extent.duration - 1);
  const consumed = Math.sign(moved) * Math.round(Math.abs(moved) * speed);
  return {
    start: extent.start + moved,
    duration: extent.duration - moved,
    sourceIn: Math.max(0, extent.sourceIn + consumed),
  };
}

/** The edges of a extent that snap, for the handle being dragged. */
export function edges(extent: Extent, handle: Handle): number[] {
  const end = extent.start + extent.duration;
  if (handle === "left") return [extent.start];
  if (handle === "right") return [end];
  return [extent.start, end];
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
): Extent {
  const free = propose(clip, handle, delta, limits);
  const shift = snap(at, edges(free, handle), reach);
  return shift === null ? free : propose(clip, handle, delta + shift, limits);
}

/** `clip_move`'s arguments for moving `clip` to `extent`: only the fields that
 * changed, in seconds — a start alone moves it and leaves the rest. `null`
 * when the drag ended where it began. */
export function trimArguments(
  clip: Clip,
  extent: Extent,
  fps: Fps,
): Record<string, unknown> | null {
  const was = extentOf(clip);
  const args: Record<string, unknown> = { clip: clip.id };
  if (extent.start !== was.start) args.start_seconds = toSeconds(extent.start, fps);
  if (extent.duration !== was.duration) args.duration_seconds = toSeconds(extent.duration, fps);
  if (extent.sourceIn !== was.sourceIn) args.source_in_seconds = toSeconds(extent.sourceIn, fps);
  return Object.keys(args).length > 1 ? args : null;
}

/** A drag let go, as the one tool call it becomes. */
export interface Release {
  tool: "clip_move";
  args: Record<string, unknown>;
}

/** What a drag of `clip` from lane `from`, let go over lane `onto` in `extent`,
 * asks the server for: one `clip_move`. On another lane it carries the track
 * and the new start — never a move and then a trim, which would pass through a
 * document nobody asked for; on its own lane, what changed.
 * `null` when the drag ended where it began. */
export function toolCall(
  clip: Clip,
  from: string,
  onto: string,
  extent: Extent,
  fps: Fps,
): Release | null {
  if (onto !== from) {
    const args = { clip: clip.id, track: onto, start_seconds: toSeconds(extent.start, fps) };
    return { tool: "clip_move", args };
  }
  const args = trimArguments(clip, extent, fps);
  return args && { tool: "clip_move", args };
}

/** How long a dropped asset runs, for snapping — `core`'s
 * `placing::unsaid_duration`, which the server applies to the `place_clip` a
 * drop sends: measured media whole, a sequence's one pass, a group to its last
 * member, a timeless asset `HOLD_SECONDS`. `null` for a length nobody has
 * measured: the server refuses that drop, and its refusal says why. */
export function dropLength(asset: DocumentAsset | undefined, fps: Fps): number | null {
  if (!asset) return null;
  let length: number | null;
  if (asset.kind === "image_sequence") {
    const sequence = asset.sequence;
    length = sequence ? sequence.stills.length * (sequence.hold ?? 1) : null;
  } else if (asset.kind === "group") {
    const ends = (asset.group?.tracks ?? []).flatMap((track) =>
      track.clips.map((clip) => clip.start + clip.duration),
    );
    length = ends.length > 0 ? Math.max(...ends) : null;
  } else {
    length = lengthOf(asset, fps);
  }
  if (length !== null) return length > 0 ? length : null;
  return TIMELESS.has(asset.kind) ? toFrames(HOLD_SECONDS, fps) : null;
}

/** Where an asset dropped at frame `pointed` lands, snapped by either edge —
 * by its head alone when it has no length to give. */
export function dropStart(
  pointed: number,
  length: number | null,
  at: number[],
  reach: number,
): number {
  const shift = snap(at, length === null ? [pointed] : [pointed, pointed + length], reach);
  return Math.max(0, pointed + (shift ?? 0));
}

/** `place_clip`'s arguments for `assetId` landing on `track` at `start`. No
 * duration: how long a placed clip runs is the server's rule (`dropLength`),
 * so a drop and an assistant's call cannot disagree. */
export function placeArguments(
  assetId: string,
  track: string,
  start: number,
  fps: Fps,
): Record<string, unknown> {
  return { asset: assetId, track, start_seconds: toSeconds(start, fps) };
}

/** Whether `kind` goes on a video track (picture) or an audio one (sound) —
 * `core`'s `TrackKind::taking`, which validation holds every clip to. */
export function laneFor(kind: string): "video" | "audio" {
  return kind === "audio" || kind === "generated_audio" || kind === "synth_audio"
    ? "audio"
    : "video";
}
