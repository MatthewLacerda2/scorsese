// The preview's maximize button (#1447): there for a pointer on hover, for
// touch always,. Drawn alone through
// `react-dom/server`; that fullscreen itself engages is the browser's.

import { expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { renderToString } from "react-dom/server";
import { createQueryClient } from "@/app/queryClient";
import { I18nProvider } from "@/i18n/I18nProvider";
import { Preview } from "./Preview";

const html = renderToString(
  <QueryClientProvider client={createQueryClient()}>
    <I18nProvider>
      <Preview
        projectId={1}
        revision={1}
        document={{ timeline_fps: { num: 30, den: 1 }, tracks: [] } as never}
        playhead={0}
        onSeek={() => {}}
        deliver="1080x1920"
      />
    </I18nProvider>
  </QueryClientProvider>,
);

test("the maximize button is hidden until hover, and always shown on touch", () => {
  expect(html).toContain('aria-label="Full screen"');
  expect(html).toContain("group-hover:opacity-100");
  expect(html).toContain("pointer-coarse:opacity-100");
});
