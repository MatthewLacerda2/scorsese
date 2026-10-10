// Every tile carries a download link, on the library page and in the editor's
// picking modal alike, outside the tile's own button — so saving a file is
// never selecting or picking it — and pointing at the attachment form of the
// file (#711). Rendered to a string, as picker.test.tsx does.

import { expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { renderToString } from "react-dom/server";
import { MemoryRouter } from "react-router";
import { api, type LibraryTile } from "@/api";
import { createQueryClient } from "@/app/queryClient";
import { FileBrowser, type Picking } from "./FileBrowser";
import { UploadsProvider } from "./uploads";

const tiles: LibraryTile[] = [
  { id: 7, name: "beach.mp4", kind: "video", size_bytes: 10, thumbnail: "/t/7" },
  { id: 8, name: "theme.mp3", kind: "audio", size_bytes: 10, thumbnail: "/t/8" },
];

function render(picking?: Picking) {
  const client = createQueryClient();
  client.setQueryData(["library", "list", {}], tiles);
  return renderToString(
    <QueryClientProvider client={client}>
      <MemoryRouter>
        <UploadsProvider>
          <FileBrowser picking={picking} />
        </UploadsProvider>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

/** Each download link, and whether it sits inside a `<button>`. */
function downloads(html: string) {
  return [...html.matchAll(/<a [^>]*href="([^"]+)"[^>]*download/g)].map((m) => ({
    href: m[1],
    insideButton:
      (html.slice(0, m.index).match(/<button/g)?.length ?? 0) >
      (html.slice(0, m.index).match(/<\/button>/g)?.length ?? 0),
  }));
}

test("the download URL asks for the file as an attachment", () => {
  expect(api.library.downloadUrl(7)).toBe("/api/library/7/file?download=1");
});

test("every tile on the library page can be downloaded", () => {
  const links = downloads(render());
  // Newest first, so the higher id leads.
  expect(links.map((l) => l.href)).toEqual([
    "/api/library/8/file?download=1",
    "/api/library/7/file?download=1",
  ]);
  expect(links.every((l) => !l.insideButton)).toBe(true);
  expect(render()).toContain('aria-label="Download theme.mp3"');
});

test("in the picking modal a refused file still downloads, and not as a pick", () => {
  const picking: Picking = {
    pick: () => {},
    refuse: (tile) => (tile.kind === "audio" ? "in this project" : null),
  };
  const links = downloads(render(picking));
  expect(links).toHaveLength(2);
  expect(links.every((l) => !l.insideButton)).toBe(true);
});
