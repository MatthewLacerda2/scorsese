// The menu's plain decisions: which styles a platform offers, which survive
// a change of platform, and the frame shape a placement means.

import { expect, test } from "bun:test";
import { MENU } from "./fixture";
import { keptStyle, nameOf, shapeOf, stylesFor } from "./menu";

const menu = MENU;

test("a platform narrows the styles to the ones made for it, and none shows them all", () => {
  expect(stylesFor(menu, "youtube").map((style) => style.id)).toEqual(["kinetic_type", "top_list"]);
  expect(stylesFor(menu, null)).toHaveLength(3);
});

test("a style not made for the new platform is dropped, never kept as a mismatch", () => {
  expect(keptStyle(menu, "youtube", "flash_offer")).toBeNull();
  expect(keptStyle(menu, "youtube", "kinetic_type")).toBe("kinetic_type");
  expect(keptStyle(menu, null, "flash_offer")).toBe("flash_offer");
});

test("a placement's shape follows its size", () => {
  expect(shapeOf(menu, "youtube")).toBe("landscape");
  expect(shapeOf(menu, "tiktok_ad")).toBe("portrait");
  expect(shapeOf(menu, null)).toBeNull();
});

test("names come from the menu", () => {
  expect(nameOf(menu, "platforms", "tiktok_ad")).toBe("Anúncio no TikTok");
  expect(nameOf(menu, "styles", "nope")).toBeNull();
});
