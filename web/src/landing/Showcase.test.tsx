// The showcase video watches whether it is seen (#1310): it starts when first
// in view, pauses when left, and stays paused until the visitor asks.

import { afterEach, beforeEach, expect, test } from "bun:test";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { I18nProvider } from "@/i18n/I18nProvider";
import { Showcase } from "@/landing/Showcase";

type Callback = (entries: { intersectionRatio: number }[]) => void;
let callbacks: Callback[] = [];
let calls: string[] = [];
let paused = true;
let still = false;
const media = HTMLMediaElement.prototype;
const original = { play: media.play, pause: media.pause };
const originalObserver = globalThis.IntersectionObserver;
const originalMatchMedia = globalThis.matchMedia;

beforeEach(() => {
  callbacks = [];
  calls = [];
  paused = true;
  still = false;
  globalThis.IntersectionObserver = class {
    constructor(callback: Callback) {
      callbacks.push(callback);
    }
    observe() {}
    disconnect() {}
  } as unknown as typeof IntersectionObserver;
  globalThis.matchMedia = (() => ({ matches: still })) as unknown as typeof matchMedia;
  media.play = function play() {
    calls.push("play");
    paused = false;
    this.dispatchEvent(new Event("play"));
    return Promise.resolve();
  };
  media.pause = function pause() {
    calls.push("pause");
    paused = true;
    this.dispatchEvent(new Event("pause"));
  };
  Object.defineProperty(media, "paused", { configurable: true, get: () => paused });
});
afterEach(() => {
  cleanup();
  media.play = original.play;
  media.pause = original.pause;
  globalThis.IntersectionObserver = originalObserver;
  globalThis.matchMedia = originalMatchMedia;
});

function show() {
  render(
    <I18nProvider initial="en">
      <Showcase />
    </I18nProvider>,
  );
}
/** Report the frame as `ratio` visible. */
function see(ratio: number) {
  act(() => {
    for (const callback of callbacks) callback([{ intersectionRatio: ratio }]);
  });
}
function hideTab(state: "hidden" | "visible") {
  Object.defineProperty(document, "visibilityState", { configurable: true, value: state });
  act(() => {
    document.dispatchEvent(new Event("visibilitychange"));
  });
}

test("nothing plays until the video is first seen, and then it autoplays", () => {
  show();
  expect(calls).toEqual([]);
  see(1);
  expect(calls).toEqual(["play"]);
});

test("a visitor who prefers less motion gets no autoplay", () => {
  still = true;
  show();
  see(1);
  expect(calls).toEqual([]);
});

test("leaving the view pauses it, and coming back does not play it", () => {
  show();
  see(1);
  see(0.2);
  expect(calls).toEqual(["play", "pause"]);
  see(1);
  expect(calls).toEqual(["play", "pause"]);
});

test("the play button, and a click on the picture, start and stop it", () => {
  show();
  see(1);
  see(0);
  fireEvent.click(screen.getByRole("button", { name: "Play" }));
  expect(calls).toEqual(["play", "pause", "play"]);
  fireEvent.click(screen.getByRole("button", { name: "Pause" }));
  expect(calls.at(-1)).toBe("pause");
  fireEvent.click(screen.getByLabelText("Videos made with scorsese"));
  expect(calls.at(-1)).toBe("play");
});

test("another tab in front pauses it", () => {
  show();
  see(1);
  hideTab("hidden");
  expect(calls).toEqual(["play", "pause"]);
  hideTab("visible");
  expect(calls).toEqual(["play", "pause"]);
});

test("turning the sound on starts a paused video", () => {
  show();
  fireEvent.click(screen.getByRole("button", { name: "Turn the sound on" }));
  expect(calls).toEqual(["play"]);
});
