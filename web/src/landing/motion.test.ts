// The scroll as the kit's clock: a section's time, and what the marked
// elements look like at a given time — before, during and after.

import { expect, test } from "bun:test";
import { draw, SCROLLED, viewTime } from "@/landing/motion";

test("a section's time is how far its top has risen into the window, in windows", () => {
  expect(viewTime(800, 800)).toBe(0);
  expect(viewTime(400, 800)).toBe(0.5);
  expect(viewTime(0, 800)).toBe(1);
  expect(viewTime(1600, 800)).toBe(-1);
  expect(viewTime(-400, 800)).toBe(1.5);
  // A window with no height has seen nothing.
  expect(viewTime(0, 0)).toBe(0);
});

function section() {
  const root = document.createElement("section");
  root.innerHTML =
    '<h2 data-rise="0.1">title</h2>' +
    '<p data-rise="0.4">later</p>' +
    '<span data-count="21.6" data-decimals="1" data-at="0.3">21.6</span>';
  const [title, later, figure] = [...root.children] as HTMLElement[];
  return { root, title, later, figure } as Record<string, HTMLElement> & { root: HTMLElement };
}

test("before its time an element is hidden and lowered, after it in place", () => {
  const { root, title, later } = section();
  draw(root, 0, SCROLLED, "en");
  expect(Number(title?.style.opacity)).toBe(0);
  expect(title?.style.transform).toBe(`translateY(${SCROLLED.distance}px)`);

  draw(root, 0.25, SCROLLED, "en");
  const halfway = Number(title?.style.opacity);
  expect(halfway).toBeGreaterThan(0);
  expect(halfway).toBeLessThan(1);
  expect(Number(later?.style.opacity)).toBe(0);

  draw(root, 2, SCROLLED, "en");
  expect(Number(title?.style.opacity)).toBe(1);
  expect(Number(later?.style.opacity)).toBe(1);
  expect(title?.style.transform).toBe("translateY(0px)");
});

test("scrolling back plays it backwards: a time is a frame, not a step", () => {
  const { root, title } = section();
  draw(root, 2, SCROLLED, "en");
  draw(root, 0, SCROLLED, "en");
  expect(Number(title?.style.opacity)).toBe(0);
});

test("a figure counts up to its value, written in the visitor's language", () => {
  const { root, figure } = section();
  draw(root, 0, SCROLLED, "pt-BR");
  expect(figure?.textContent).toBe("0,0");
  draw(root, 5, SCROLLED, "pt-BR");
  expect(figure?.textContent).toBe("21,6");
  draw(root, 5, SCROLLED, "en");
  expect(figure?.textContent).toBe("21.6");
});

test("for a visitor who asked for less motion, the end of every timeline at once", () => {
  const { root, title, later, figure } = section();
  draw(root, Number.POSITIVE_INFINITY, SCROLLED, "en");
  expect(Number(title?.style.opacity)).toBe(1);
  expect(Number(later?.style.opacity)).toBe(1);
  expect(figure?.textContent).toBe("21.6");
});
