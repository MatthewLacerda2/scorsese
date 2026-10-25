// The public header's globe on a phone (#1566): the language and the theme,
// both reachable from the one button, and each choice taking at once.

import { expect, test } from "bun:test";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { I18nProvider } from "@/i18n/I18nProvider";
import { PageSettings } from "@/landing/PageSettings";

test("the globe opens the languages and the themes, and a pick takes at once", async () => {
  render(
    <I18nProvider initial="en">
      <PageSettings />
    </I18nProvider>,
  );
  await userEvent.click(screen.getByRole("button", { name: "Settings" }));
  expect(await screen.findByRole("menuitemradio", { name: "Dark" })).toBeDefined();
  await userEvent.click(screen.getByRole("menuitemradio", { name: "Español" }));
  // The button names itself in the language just picked.
  expect(await screen.findByRole("button", { name: "Ajustes" })).toBeDefined();
  await userEvent.click(screen.getByRole("button", { name: "Ajustes" }));
  await userEvent.click(await screen.findByRole("menuitemradio", { name: "Oscuro" }));
  expect(document.documentElement.classList.contains("dark")).toBe(true);
  document.documentElement.classList.remove("dark");
});
