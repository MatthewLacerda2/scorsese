// The timeline's arithmetic: frames, seconds and pixels, what a drag
// proposes and snaps to, and the tool call it becomes.

import { describe, expect, test } from "bun:test";
import type { Clip, DocumentAsset, ProjectDocument } from "@/api";
import {
  dropStart,
  limitsOf,
  placeArguments,
  propose,
  snapped,
  targets,
  toolCall,
  trimArguments,
} from "./drag";
import { framesToPx, pxToFrames, timecode, toFrames, toSeconds } from "./time";

const THIRTY = { num: 30, den: 1 };
const NTSC = { num: 30000, den: 1001 };
const clip = (extra: Partial<Clip> = {}): Clip => ({
  id: "c",
  asset: "a",
  start: 200,
  duration: 100,
  source_in: 50,
  ...extra,
});
const footage: DocumentAsset = { id: "a", kind: "video", media: { duration_seconds: 10 } };

describe("time", () => {
  test("a frame sent as seconds comes back as the same frame, on any grid", () => {
    for (const frames of [0, 1, 29, 30, 1001, 123_457]) {
      expect(toFrames(toSeconds(frames, NTSC), NTSC)).toBe(frames);
    }
  });

  test("pixels and frames are one transform, whatever the zoom", () => {
    const zoom = { pxPerSecond: 80 };
    expect(framesToPx(45, zoom, THIRTY)).toBe(120);
    expect(pxToFrames(120, zoom, THIRTY)).toBe(45);
    expect(pxToFrames(-40, zoom, THIRTY)).toBe(0);
  });

  test("a timecode is minutes, seconds and frames", () => {
    expect(timecode(75, THIRTY)).toBe("0:02.15");
    expect(timecode(30 * 61, THIRTY)).toBe("1:01.00");
  });
});

describe("a drag", () => {
  const limits = limitsOf(clip(), footage, THIRTY);

  test("reads its limits from the measured source", () => {
    // 300 frames of footage, 50 before the in-point, 150 after the out-point.
    expect(limits).toEqual({ head: 50, tail: 150 });
    expect(limitsOf(clip(), { id: "t", kind: "text" }, THIRTY)).toEqual({ head: null, tail: null });
  });

  test("moving the body keeps the length and the content", () => {
    expect(propose(clip(), "body", 30, limits)).toEqual({
      start: 230,
      duration: 100,
      sourceIn: 50,
    });
    expect(propose(clip(), "body", -9000, limits).start).toBe(0);
  });

  test("trimming the head moves where the source starts, and stops at its head", () => {
    expect(propose(clip(), "left", 20, limits)).toEqual({ start: 220, duration: 80, sourceIn: 70 });
    expect(propose(clip(), "left", -500, limits)).toEqual({
      start: 150,
      duration: 150,
      sourceIn: 0,
    });
    expect(propose(clip(), "left", 500, limits).duration).toBe(1);
  });

  test("a sped-up clip spends source twice as fast at its head", () => {
    const fast = clip({ speed: 2, source_in: 50 });
    expect(propose(fast, "left", 10, limitsOf(fast, footage, THIRTY)).sourceIn).toBe(70);
  });

  test("trimming the tail stops where the footage does", () => {
    expect(propose(clip(), "right", 1000, limits).duration).toBe(250);
    expect(propose(clip(), "right", -1000, limits).duration).toBe(1);
  });

  test("snaps an edge onto a neighbour's cut within reach, and not beyond it", () => {
    const document: ProjectDocument = {
      schema_version: 44,
      name: "t",
      timeline_fps: THIRTY,
      tracks: [{ id: "v1", kind: "video", clips: [clip(), clip({ id: "n", start: 400 })] }],
    };
    const at = targets(document, 0, "c");
    expect(at).toEqual([0, 0, 400, 500]);
    // Moved 97 frames, its end would be at 397: three short of the neighbour.
    expect(snapped(clip(), "body", 97, limits, at, 5).start).toBe(300);
    expect(snapped(clip(), "body", 90, limits, at, 5).start).toBe(290);
  });
});

describe("the tool call", () => {
  test("a move sends only a start; a trim sends what it changed", () => {
    expect(trimArguments(clip(), { start: 230, duration: 100, sourceIn: 50 }, THIRTY)).toEqual({
      clip: "c",
      start_seconds: 230 / 30,
    });
    expect(trimArguments(clip(), { start: 220, duration: 80, sourceIn: 70 }, THIRTY)).toEqual({
      clip: "c",
      start_seconds: 220 / 30,
      duration_seconds: 80 / 30,
      source_in_seconds: 70 / 30,
    });
    expect(trimArguments(clip(), { start: 200, duration: 100, sourceIn: 50 }, THIRTY)).toBeNull();
  });

  test("let go on another lane, a drag is one move carrying its new start", () => {
    const moved = { start: 230, duration: 100, sourceIn: 50 };
    expect(toolCall(clip(), "v1", "v2", moved, THIRTY)).toEqual({
      tool: "clip_move",
      args: { clip: "c", track: "v2", start_seconds: 230 / 30 },
    });
    expect(toolCall(clip(), "v1", "v1", moved, THIRTY)).toEqual({
      tool: "trim_clip",
      args: { clip: "c", start_seconds: 230 / 30 },
    });
    // Straight down a lane, not along it: still a move.
    const still = { start: 200, duration: 100, sourceIn: 50 };
    expect(toolCall(clip(), "v1", "v2", still, THIRTY)?.tool).toBe("clip_move");
    expect(toolCall(clip(), "v1", "v1", still, THIRTY)).toBeNull();
  });

  test("a drop runs a measured asset to its end, and gives anything else five seconds", () => {
    expect(placeArguments(footage, "a", "v1", 60, THIRTY)).toEqual({
      asset: "a",
      track: "v1",
      start_seconds: 2,
    });
    const title: DocumentAsset = { id: "t", kind: "text" };
    expect(placeArguments(title, "t", "v1", 0, THIRTY).duration_seconds).toBe(5);
    expect(dropStart(98, 150, [0, 100], 4)).toBe(100);
    expect(dropStart(3, 150, [0], 4)).toBe(0);
  });
});
