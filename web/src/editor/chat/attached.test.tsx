// The chat's composer and the files attached to its next message (#1534): a
// message carries their library ids, the tray empties once it is sent, and
// sending waits while one is still on its way.

import { afterAll, beforeAll, expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { setFetch } from "@/api/client";
import { createQueryClient } from "@/app/queryClient";
import { I18nProvider } from "@/i18n/I18nProvider";
import type { Attachments } from "@/prompt/attach/useAttachments";
import { noAttachments } from "@/test/attachments";
import { Composer } from "./Composer";

const sent: unknown[] = [];
let previous: Parameters<typeof setFetch>[0];
beforeAll(() => {
  previous = setFetch(async (_input, init) => {
    if (init?.method === "POST") sent.push(JSON.parse(String(init.body)));
    return new Response(JSON.stringify({ id: 1 }), { status: 202 });
  });
});
afterAll(() => {
  setFetch(previous);
});

function mount(attachments: Attachments) {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <I18nProvider initial="en">
        <Composer
          projectId={4}
          column=""
          runningTurn={null}
          askingTurn={null}
          fresh={false}
          onFresh={() => {}}
          showFresh={false}
          onStarted={() => {}}
          attachments={attachments}
        />
      </I18nProvider>
    </QueryClientProvider>,
  );
}

const picture = new File(["x"], "logo.png");

test("a message carries the attachments' library ids, and the tray empties once it is sent", async () => {
  sent.length = 0;
  let cleared = 0;
  mount({
    ...noAttachments,
    list: [
      { key: "a", file: picture, kind: "image", status: { phase: "ready", progress: 1, item: 7 } },
    ],
    items: [7],
    clear: () => {
      cleared += 1;
    },
  });
  expect(screen.getByText("logo.png")).toBeTruthy();
  await userEvent.type(screen.getByRole("textbox"), "use the logo{Enter}");
  await waitFor(() => expect(cleared).toBe(1));
  expect(sent).toEqual([{ prompt: "use the logo", fresh: false, attachments: [7] }]);
});

test("sending waits while a file is still on its way", async () => {
  sent.length = 0;
  mount({
    ...noAttachments,
    list: [
      { key: "a", file: picture, kind: "image", status: { phase: "uploading", progress: 0.4 } },
    ],
    busy: true,
  });
  await userEvent.type(screen.getByRole("textbox"), "use the logo{Enter}");
  expect((screen.getByRole("button", { name: "Send" }) as HTMLButtonElement).disabled).toBe(true);
  expect(screen.getByRole("progressbar").getAttribute("aria-valuenow")).toBe("40");
  expect(sent).toEqual([]);
});
