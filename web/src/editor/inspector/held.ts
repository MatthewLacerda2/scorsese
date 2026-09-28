// What a clip's position, rotation and scale read as, by the desktop
// inspector's rule (`app/src/inspector/transform.rs`): a property with no
// keyframe track, or one track of one point, has exactly one value and gets a
// field; anything more is a ramp, shown as animated and offered no value,
// because typing a number over it would flatten work nobody asked to lose.

import type { Clip } from "@/api";

export const POSITION_X = "transform.position.x";
export const POSITION_Y = "transform.position.y";
export const ROTATION = "transform.rotation";
export const SCALE_X = "transform.scale.x";
export const SCALE_Y = "transform.scale.y";

/** One value for the whole clip, a stretch, or a ramp. */
export type Held =
  | { kind: "value"; value: number }
  | { kind: "stretched"; wide: number; tall: number }
  | { kind: "animated" };

/** What `path` reads across `clip`, `rest` when nothing sets it. */
export function held(clip: Clip, path: string, rest: number): Held {
  const tracks = (clip.keyframes ?? []).filter((track) => track.property === path);
  if (tracks.length === 0) return { kind: "value", value: rest };
  const only = tracks.length === 1 ? tracks[0]?.keyframes : undefined;
  if (only?.length === 1 && only[0]) return { kind: "value", value: only[0].value };
  return { kind: "animated" };
}

/** A picture clip's placement in the frame, as the inspector shows it. */
export interface Transform {
  x: Held;
  y: Held;
  rotation: Held;
  scale: Held;
}

export function transformOf(clip: Clip): Transform {
  const wide = held(clip, SCALE_X, 1);
  const tall = held(clip, SCALE_Y, 1);
  let scale: Held = { kind: "animated" };
  if (wide.kind === "value" && tall.kind === "value") {
    scale =
      wide.value === tall.value ? wide : { kind: "stretched", wide: wide.value, tall: tall.value };
  }
  return {
    x: held(clip, POSITION_X, 0),
    y: held(clip, POSITION_Y, 0),
    rotation: held(clip, ROTATION, 0),
    scale,
  };
}

/** How a value is shown in its field, and turned back into the document's. */
export type Unit = "percent" | "degrees";

/** The document's value as the field shows it: fractions as percentages. */
export function shown(value: number, unit: Unit): number {
  return unit === "percent" ? Math.round(value * 100 * 100) / 100 : value;
}

/** The field's value as the document stores it — rounded to a hundredth of
 * what is shown, so typing 25 writes `0.25` and not `0.25000000000000006`. */
export function stored(value: number, unit: Unit): number {
  const tidy = Math.round(value * 100) / 100;
  return unit === "percent" ? Math.round(tidy * 100) / 10000 : tidy;
}

/** What animates the clip beyond the values shown: each property with a ramp. */
export function animated(clip: Clip): { property: string; points: number; by?: string }[] {
  const tracks = clip.keyframes ?? [];
  return tracks
    .filter(
      (track) =>
        !(
          track.keyframes.length === 1 &&
          tracks.filter((t) => t.property === track.property).length === 1
        ),
    )
    .map((track) => ({ property: track.property, points: track.keyframes.length, by: track.by }));
}
