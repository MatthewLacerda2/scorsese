// The mark is drawn twice and the theme's `dark` class shows one: the light
// one hides under it, the dark one only shows under it.

import { expect, test } from "bun:test";
import { render } from "@testing-library/react";
import { Mark } from "@/components/Mark";

test("the icon comes in a light and a dark copy, each shown in its theme only", () => {
  const { container } = render(<Mark kind="icon" size={28} className="size-7" />);
  const [light, dark] = [...container.querySelectorAll("img")];
  expect(light?.getAttribute("src")).toBe("/icon-light.png");
  expect(light?.className).toContain("dark:hidden");
  expect(dark?.getAttribute("src")).toBe("/icon.png");
  expect(dark?.className).toContain("hidden dark:block");
  expect(light?.className).toContain("size-7");
});

test("the silhouette has a copy for each theme too, and both are decoration", () => {
  const { container } = render(<Mark kind="silhouette" size={86} />);
  const images = [...container.querySelectorAll("img")];
  expect(images.length).toBe(2);
  expect(new Set(images.map((image) => image.getAttribute("src"))).size).toBe(2);
  expect(images.every((image) => image.getAttribute("alt") === "")).toBe(true);
});
