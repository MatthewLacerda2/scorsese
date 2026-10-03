// The library modal's body draws the library page's own grid in picking
// mode: a file in the project and a MIDI file are greyed with the reason, the
// rest are offered, and the page's details panel is not there. Rendered to a
// string, as App.test.tsx does, so the modal's portal (and Radix's Esc and
// focus trap) are not what is checked here — the content is.

import { expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { renderToString } from "react-dom/server";
import { MemoryRouter } from "react-router";
import type { LibraryTile } from "@/api";
import { createQueryClient } from "@/app/queryClient";
import { UploadsProvider } from "@/files/uploads";
import type { EditOutcome } from "../project";
import { Picker } from "./LibraryModal";

const tile = (id: number, name: string, kind: LibraryTile["kind"]): LibraryTile => ({
  id,
  name,
  kind,
  size_bytes: 1000,
  thumbnail: `/api/library/${id}/thumbnail`,
});

const edit: EditOutcome = {
  run: async () => null,
  refused: null,
  dismiss: () => {},
  pending: false,
};

test("the picker offers the library, greying what cannot be added", () => {
  const client = createQueryClient();
  client.setQueryData(
    ["library", "list", {}],
    [tile(1, "beach.mp4", "video"), tile(2, "song.mid", "midi"), tile(3, "voice.wav", "audio")],
  );
  client.setQueryData(["library", "list", { project: 4 }], [tile(3, "voice.wav", "audio")]);
  const html = renderToString(
    <QueryClientProvider client={client}>
      <MemoryRouter>
        <UploadsProvider>
          <Picker projectId={4} edit={edit} />
        </UploadsProvider>
      </MemoryRouter>
    </QueryClientProvider>,
  );
  expect(html).toContain('title="Add beach.mp4"');
  expect(html).toContain("can&#x27;t go on a track");
  expect(html).toContain("in this project");
  expect(html.match(/disabled=""/g)?.length).toBe(2);
  expect(html).toContain("Upload");
});
