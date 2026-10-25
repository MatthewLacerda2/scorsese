// A file dropped or picked on the library page is held to the list the server
// answers (`GET /api/library/formats`), like a prompt's tray: one the library
// does not take is refused by name at once, never hashed or announced.

import { afterAll, beforeAll, expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import type { Formats } from "@/api";
import { setFetch } from "@/api/client";
import { createQueryClient } from "@/app/queryClient";
import { queryKeys } from "@/app/queryKeys";
import { I18nProvider } from "@/i18n/I18nProvider";
import { FileBrowser } from "./FileBrowser";
import { UploadsProvider, useUploads } from "./uploads";

const formats: Formats = { video: ["mp4"], image: ["png"], audio: ["mp3"], html: [], text: [] };
const asked: string[] = [];
let previous: Parameters<typeof setFetch>[0];
beforeAll(() => {
  previous = setFetch(async (input) => {
    asked.push(String(input));
    return new Response(JSON.stringify(String(input).includes("formats") ? formats : []));
  });
});
afterAll(() => {
  setFetch(previous);
});

function Rows() {
  const { uploads } = useUploads();
  return (
    <ul aria-label="rows">
      {uploads.map((row) => (
        <li key={row.key}>{`${row.name} ${row.phase} ${row.message ?? ""}`}</li>
      ))}
    </ul>
  );
}

test("a drop on the library refuses by name what the library does not take", async () => {
  asked.length = 0;
  const client = createQueryClient();
  client.setQueryData(queryKeys.formats, formats);
  render(
    <QueryClientProvider client={client}>
      <I18nProvider initial="en">
        <MemoryRouter>
          <UploadsProvider>
            <FileBrowser />
            <Rows />
          </UploadsProvider>
        </MemoryRouter>
      </I18nProvider>
    </QueryClientProvider>,
  );
  const files = [new File(["x"], "song.mid")];
  fireEvent.drop(screen.getByRole("region", { name: "Files" }), {
    dataTransfer: { types: ["Files"], files },
  });
  const rows = within(screen.getByRole("list", { name: "rows" }));
  await waitFor(() =>
    expect(rows.getByText("song.mid failed scorsese can't use this kind of file")).toBeTruthy(),
  );
  expect(asked.filter((url) => url.includes("sha256"))).toEqual([]);
});
