// The editor's controls as the header draws them (#764): the name, Save as
// template, the frame shape and Render — and no revision. Drawn alone through
// `react-dom/server`, because the portal into the header needs a DOM; that the
// shell offers the place is App.test.tsx's.

import { expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { renderToString } from "react-dom/server";
import { HeaderSlotProvider } from "@/app/headerSlot";
import { createQueryClient } from "@/app/queryClient";
import { EditorActions, type EditorActionsProps, EditorHeader } from "./EditorHeader";

const props: EditorActionsProps = {
  projectId: 4,
  name: "teaser",
  selected: [],
  edit: { run: async () => null, refused: null, dismiss: () => {}, pending: false },
  shape: "portrait",
  onShape: () => {},
};

const draw = (node: React.ReactNode) =>
  renderToString(<QueryClientProvider client={createQueryClient()}>{node}</QueryClientProvider>);

test("the header's editor controls: name, Save as template, frame shape, Render", () => {
  const html = draw(<EditorActions {...props} />);
  expect(html).toContain(">teaser</h1>");
  expect(html).toContain("Save as template");
  expect(html).toContain('aria-label="Frame shape"');
  expect(html).toMatch(/<option value="portrait" selected="">9:16<\/option>/);
  expect(html).toContain("Render");
  expect(html).not.toMatch(/revision/i);
});

test("without a header to go into, the editor's controls draw nothing", () => {
  expect(draw(<EditorHeader {...props} />)).toBe("");
  expect(
    draw(
      <HeaderSlotProvider value={null}>
        <EditorHeader {...props} />
      </HeaderSlotProvider>,
    ),
  ).toBe("");
});
