// What the marked elements look like at a given time — before, during and
// after.

import { expect, test } from "bun:test";
import { draw, SCROLLED } from "@/landing/motion";

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
  // Padded to 21,6's columns with a figure space, so the figure keeps its width.
  expect(figure?.textContent).toBe("\u20070,0");
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

test("an emerging element starts a whole height below its edge and ends on it", () => {
  const root = document.createElement("div");
  root.innerHTML = '<span data-emerge="0.2">mark</span>';
  const mark = root.firstElementChild as HTMLElement;
  draw(root, 0, SCROLLED, "en");
  expect(mark.style.transform).toBe("translateY(100%)");
  draw(root, 5, SCROLLED, "en");
  expect(mark.style.transform).toBe("translateY(0%)");
});
