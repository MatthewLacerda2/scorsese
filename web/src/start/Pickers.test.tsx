// The two choices as a person meets them: every placement and "none", and
// the styles made for the placement chosen, each with a picture — a neutral
// card until it has a preview (#1017), never a broken image.

import { expect, test } from "bun:test";
import { fireEvent, render, screen } from "@testing-library/react";
import { MENU } from "./fixture";
import { PlatformPicker, StylePicker } from "./Pickers";

test("the platforms are offered after none, and a click chooses one", () => {
  const chosen: (string | null)[] = [];
  render(<PlatformPicker menu={MENU} value={null} onChange={(id) => chosen.push(id)} />);
  const radios = screen.getAllByRole("radio");
  expect(radios.map((radio) => radio.textContent)).toEqual([
    "No platform",
    "YouTube1920×1080",
    "Anúncio no TikTok1080×1920",
  ]);
  expect(radios[0]?.getAttribute("aria-checked")).toBe("true");
  expect(screen.getByText("You can choose or change the platform later.")).toBeTruthy();
  fireEvent.click(screen.getByRole("radio", { name: /YouTube/ }));
  expect(chosen).toEqual(["youtube"]);
});

test("a platform shows only the styles made for it", () => {
  render(<StylePicker menu={MENU} platform="tiktok_ad" value="flash_offer" onChange={() => {}} />);
  const names = screen.getAllByRole("radio").map((radio) => radio.textContent ?? "");
  expect(names).toHaveLength(3);
  expect(names.some((name) => name.includes("Lista / Top N"))).toBe(false);
  const offer = screen.getByRole("radio", { name: /Oferta relâmpago/ });
  expect(offer.getAttribute("aria-checked")).toBe("true");
  expect(screen.getByText("You can choose or change the style later.")).toBeTruthy();
});

test("a style with no preview is a placeholder, and one whose preview fails becomes one", () => {
  render(<StylePicker menu={MENU} platform={null} value={null} onChange={() => {}} />);
  expect(screen.getAllByTestId("style-placeholder")).toHaveLength(2);
  expect(screen.getByText(/choose a platform to see the ones made for it/)).toBeTruthy();
  const picture = document.querySelector('img[src="/previews/top_list.gif"]');
  expect(picture).not.toBeNull();
  if (picture) fireEvent.error(picture);
  expect(screen.getAllByTestId("style-placeholder")).toHaveLength(3);
  expect(document.querySelector("img")).toBeNull();
});
