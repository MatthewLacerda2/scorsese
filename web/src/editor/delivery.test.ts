import { expect, test } from "bun:test";
import { type Delivery, renderAsk, soundOnly } from "./delivery";

const untouched: Delivery = {
  container: "mp4",
  resolution: "1080x1920",
  fps: null,
  loudness: "mixed",
};

test("pressing Render untouched asks for what it always did: the size alone", () => {
  expect(renderAsk(untouched)).toEqual({ resolution: "1080x1920" });
});

test("each choice reaches the request", () => {
  expect(renderAsk({ ...untouched, container: "mkv" })).toEqual({
    container: "mkv",
    resolution: "1080x1920",
  });
  expect(renderAsk({ ...untouched, resolution: "720x1280", fps: "60" })).toEqual({
    resolution: "720x1280",
    fps: "60",
  });
  expect(renderAsk({ ...untouched, loudness: "streaming" })).toEqual({
    resolution: "1080x1920",
    loudness: -14,
  });
  expect(renderAsk({ ...untouched, loudness: "podcast" }).loudness).toBe(-16);
});

test("sound only sends no size and no rate, which the server would refuse", () => {
  const sound = renderAsk({ ...untouched, container: "mp3", fps: "30", loudness: "streaming" });
  expect(sound).toEqual({ container: "mp3", loudness: -14 });
  expect(soundOnly("wav")).toBe(true);
  expect(soundOnly("avi")).toBe(false);
});
