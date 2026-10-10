// The new-project modal (#1016): only the name is needed, Create with none
// creates nothing and says so, every step is reachable, and the request is
// exactly what the steps chose — a style never outliving a platform it was
// not made for.

import { expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { createQueryClient } from "@/app/queryClient";
import { UploadsProvider } from "@/files/uploads";
import { MENU } from "@/start/fixture";
import { bodyOf, EMPTY, onPlatform, toggled, Wizard } from "./NewProjectDialog";

function wizard() {
  const client = createQueryClient();
  client.setQueryData(["styles"], MENU);
  client.setQueryData(["library", "list", {}], []);
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter>
        <UploadsProvider>
          <Wizard />
        </UploadsProvider>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

test("a blank name makes no request; a name is sent trimmed with what was chosen", () => {
  expect(bodyOf({ ...EMPTY, name: "   " })).toBeNull();
  const draft = { name: " Beach ", assets: [3], platform: "youtube", style: "top_list" };
  expect(bodyOf(draft)).toEqual({ ...draft, name: "Beach" });
});

test("a file picked twice is unpicked", () => {
  const once = toggled(EMPTY, 7);
  expect(once.assets).toEqual([7]);
  expect(toggled(once, 7).assets).toEqual([]);
});

test("a platform the style is not made for drops the style", () => {
  const offer = { ...EMPTY, platform: "tiktok_ad", style: "flash_offer" };
  expect(onPlatform(offer, MENU, "youtube")).toMatchObject({ platform: "youtube", style: null });
  const kinetic = { ...offer, style: "kinetic_type" };
  expect(onPlatform(kinetic, MENU, "youtube").style).toBe("kinetic_type");
});

test("Create with no name stays on the name and says what is missing", () => {
  wizard();
  fireEvent.click(screen.getByRole("button", { name: /Style/ }));
  fireEvent.click(screen.getByRole("button", { name: "Create" }));
  const field = screen.getByLabelText("Project's name");
  expect(field.getAttribute("aria-invalid")).toBe("true");
  expect(screen.getByText("Write the name of the project")).toBeTruthy();
  fireEvent.change(field, { target: { value: "Beach" } });
  expect(screen.queryByText("Write the name of the project")).toBeNull();
});

test("the steps run name, files, platform, style, and the style step is filtered", () => {
  wizard();
  expect(screen.getByText("Step 1 of 4")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
  expect(screen.getByText(/Pick files from your library/)).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
  fireEvent.click(screen.getByRole("radio", { name: /Anúncio no TikTok/ }));
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
  expect(screen.getByText("Step 4 of 4")).toBeTruthy();
  expect(screen.getByRole("radio", { name: /Oferta relâmpago/ })).toBeTruthy();
  expect(screen.queryByRole("radio", { name: /Lista \/ Top N/ })).toBeNull();
  expect(screen.queryByRole("button", { name: "Next" })).toBeNull();
});
