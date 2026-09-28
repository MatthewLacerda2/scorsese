// The inspector's rule: one value gets a field, a ramp is shown as animated.

import { expect, test } from "bun:test";
import type { Clip, KeyframeTrack } from "@/api";
import {
  animated,
  POSITION_X,
  ROTATION,
  SCALE_X,
  SCALE_Y,
  shown,
  stored,
  transformOf,
} from "./held";

const track = (property: string, points: number[]): KeyframeTrack => ({
  property,
  keyframes: points.map((value, t) => ({ t, value })),
});
const clip = (keyframes: KeyframeTrack[] = []): Clip => ({
  id: "c",
  asset: "a",
  start: 0,
  duration: 30,
  keyframes,
});

test("an untouched clip reads where a layer naturally sits", () => {
  expect(transformOf(clip())).toEqual({
    x: { kind: "value", value: 0 },
    y: { kind: "value", value: 0 },
    rotation: { kind: "value", value: 0 },
    scale: { kind: "value", value: 1 },
  });
});

test("one held point is a value, and not listed as an animation", () => {
  const held = clip([track(ROTATION, [45])]);
  expect(transformOf(held).rotation).toEqual({ kind: "value", value: 45 });
  expect(animated(held)).toEqual([]);
});

test("a ramp is animated, offers no value, and is listed", () => {
  const ramp = clip([track(POSITION_X, [0, 0.5]), track("opacity", [0, 1])]);
  expect(transformOf(ramp).x).toEqual({ kind: "animated" });
  expect(animated(ramp).map((row) => row.property)).toEqual([POSITION_X, "opacity"]);
});

test("two tracks on one property are not a value anybody can name", () => {
  const twice = clip([track(ROTATION, [10]), track(ROTATION, [20])]);
  expect(transformOf(twice).rotation).toEqual({ kind: "animated" });
  expect(animated(twice)).toHaveLength(2);
});

test("scale is one value only when both axes agree", () => {
  expect(transformOf(clip([track(SCALE_X, [0.5]), track(SCALE_Y, [0.5])])).scale).toEqual({
    kind: "value",
    value: 0.5,
  });
  expect(transformOf(clip([track(SCALE_X, [2])])).scale).toEqual({
    kind: "stretched",
    wide: 2,
    tall: 1,
  });
});

test("a shown percentage is stored as a tidy fraction", () => {
  expect(stored(shown(0.25, "percent"), "percent")).toBe(0.25);
  expect(stored(33.333333, "percent")).toBe(0.3333);
  expect(stored(-90, "degrees")).toBe(-90);
});
