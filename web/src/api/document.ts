// The parts of `project.json` the editor reads, as docs/project-format.md
// spells them. Read-only on purpose: the browser draws the document and never
// writes it — every change is a tool call (`api.editor.tool`), so the edit is
// `core`'s and validated there. Fields the editor does not draw are left out;
// the server keeps the whole document.

/** `timeline_fps`: a rational, so 29.97 is exactly 30000/1001. */
export interface Fps {
  num: number;
  den: number;
}

/**
 * How a value travels to the next keyframe: a bare word for a preset, or CSS's
 * four `cubic-bezier()` numbers. The back curves, `spring` and a bezier with a
 * `y` outside 0..1 overshoot — the value goes past the keyframe on the way.
 */
export type Easing =
  | "linear"
  | "ease_in"
  | "ease_out"
  | "ease_in_out"
  | "hold"
  | "back_in"
  | "back_out"
  | "back_in_out"
  | "spring"
  | { cubic_bezier: [number, number, number, number] };

/** A property path and the points that animate it, times relative to the clip. */
export interface KeyframeTrack {
  property: string;
  /** The tool that wrote it, when one did; absent means by hand. */
  by?: string;
  keyframes: { t: number; value: number; easing?: Easing }[];
}

export type FitMode = "fit" | "fill" | "native";

/** A clip on a track, in whole timeline frames. */
export interface Clip {
  id: string;
  asset: string;
  start: number;
  duration: number;
  source_in?: number;
  speed?: number;
  fit?: FitMode;
  keyframes?: KeyframeTrack[];
}

export interface Track {
  id: string;
  kind: "video" | "audio";
  name?: string;
  clips: Clip[];
}

export type AssetKind =
  | "video"
  | "image"
  | "audio"
  | "text"
  | "color"
  | "shape"
  | "icon"
  | "group"
  | "generated_video"
  | "generated_image"
  | "generated_audio"
  | "synth_audio";

export interface DocumentAsset {
  id: string;
  kind: AssetKind;
  path?: string;
  sha256?: string;
  /** What a generated asset is in its lifecycle: sketch, queued, generated, stale. */
  state?: string;
  text?: string;
  media?: { duration_seconds?: number; width?: number; height?: number };
}

/** `scorsese_core::Project`, as far as the editor reads it. */
export interface ProjectDocument {
  schema_version: number;
  name: string;
  timeline_fps: Fps;
  assets?: DocumentAsset[];
  tracks?: Track[];
}
