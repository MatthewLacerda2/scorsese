// The strip of films downloads nothing but its stills until it is near the
// screen (#1566): a phone that opens the page and leaves never fetches them.
// Once near, its films play; scrolled away, they pause.

import { afterEach, beforeEach, expect, test } from "bun:test";
import { act, cleanup, render } from "@testing-library/react";
import { Carousel } from "@/landing/Carousel";

type Callback = (entries: { isIntersecting: boolean }[]) => void;
let observed: { callback: Callback; options?: IntersectionObserverInit }[] = [];
let calls: string[] = [];
const media = HTMLMediaElement.prototype;
const original = { play: media.play, pause: media.pause };
const originalObserver = globalThis.IntersectionObserver;

beforeEach(() => {
  observed = [];
  calls = [];
  globalThis.IntersectionObserver = class {
    constructor(callback: Callback, options?: IntersectionObserverInit) {
      observed.push({ callback, options });
    }
    observe() {}
    disconnect() {}
  } as unknown as typeof IntersectionObserver;
  media.play = () => {
    calls.push("play");
    return Promise.resolve();
  };
  media.pause = () => void calls.push("pause");
});
afterEach(() => {
  cleanup();
  media.play = original.play;
  media.pause = original.pause;
  globalThis.IntersectionObserver = originalObserver;
});

test("until it is near, the strip asks for its stills and none of its films", () => {
  render(<Carousel label="Films" />);
  const films = [...document.querySelectorAll("video")];
  expect(films.length).toBeGreaterThan(0);
  for (const film of films) {
    expect(film.getAttribute("preload")).toBe("none");
    expect(film.autoplay).toBe(false);
    expect(film.getAttribute("poster")).toBeTruthy();
  }
  expect(calls).toEqual([]);
  // Near means a little before it shows, so the first film is ready as it does.
  expect(observed[0]?.options?.rootMargin).toBeTruthy();
});

test("near the screen its films play, and away from it they pause", () => {
  render(<Carousel label="Films" />);
  const count = document.querySelectorAll("video").length;
  act(() => observed[0]?.callback([{ isIntersecting: true }]));
  expect(calls).toEqual(Array(count).fill("play"));
  calls = [];
  act(() => observed[0]?.callback([{ isIntersecting: false }]));
  expect(calls).toEqual(Array(count).fill("pause"));
});
