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

/** `75` frames at 30 fps → `0:02.15` (minutes, seconds, frames). */
export function timecode(frames: number, fps: Fps): string {
  const perSecond = Math.max(1, Math.round(rate(fps)));
  const whole = Math.floor(toSeconds(frames, fps));
  const minutes = Math.floor(whole / 60);
  const seconds = String(whole % 60).padStart(2, "0");
  const rest = String(Math.max(0, frames - toFrames(whole, fps)) % perSecond).padStart(2, "0");
  return `${minutes}:${seconds}.${rest}`;
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

/** The zooms offered, from a whole minute on screen to single frames. */
export const ZOOMS = [10, 20, 40, 80, 160, 320] as const;
