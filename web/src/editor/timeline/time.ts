// Time on the timeline: whole frames on the project's grid, seconds for the
// tools, and pixels for the screen — and the one conversion between each pair.
//
// The document counts in frames (docs/project-format.md: *Times are whole
// frames on `timeline_fps`*), the tools take seconds and round them back onto
// the grid themselves, and the screen is pixels. Keeping every conversion here
// is what makes a drag land on the frame it was drawn at.

import type { Fps } from "@/api";

/** Frames per second, as a number: 30000/1001 is 29.97… */
export function rate(fps: Fps): number {
  return fps.num / fps.den;
}

/** A frame count as seconds — what a tool is handed. Exact enough that the
 * server's `round(seconds × fps)` lands back on the same frame. */
export function toSeconds(frames: number, fps: Fps): number {
  return (frames * fps.den) / fps.num;
}

/** Seconds as the nearest whole frame, never below zero. */
export function toFrames(seconds: number, fps: Fps): number {
  return Math.max(0, Math.round(seconds * rate(fps)));
}

/** A frame count as the preview's clock (#1006): `02:59` under an hour,
 * `1:02:59` from one — whole seconds, rounded down. No frames: a part after a
 * dot reads as hundredths, which frames are not. */
export function clock(frames: number, fps: Fps): string {
  const whole = Math.floor(toSeconds(frames, fps));
  const hours = Math.floor(whole / 3600);
  const minutes = String(Math.floor(whole / 60) % 60).padStart(2, "0");
  const seconds = String(whole % 60).padStart(2, "0");
  return hours > 0 ? `${hours}:${minutes}:${seconds}` : `${minutes}:${seconds}`;
}

/** How the timeline is drawn: how many pixels one second takes. */
export interface Zoom {
  pxPerSecond: number;
}

/** Where frame `frames` is drawn, in pixels from the head of the timeline. */
export function framesToPx(frames: number, zoom: Zoom, fps: Fps): number {
  return toSeconds(frames, fps) * zoom.pxPerSecond;
}

/** The frame under pixel `px` — the nearest one, never before the head. */
export function pxToFrames(px: number, zoom: Zoom, fps: Fps): number {
  return toFrames(px / zoom.pxPerSecond, fps);
}

/** A span of pixels as a count of frames: a snap's reach, a drag's delta. */
export function pxSpan(px: number, zoom: Zoom, fps: Fps): number {
  return Math.round((px / zoom.pxPerSecond) * rate(fps));
}
