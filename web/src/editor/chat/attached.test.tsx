// The chat's composer and the files attached to its next message (#1534): a
// message carries their library ids, the tray lets go of what it sent (a
// file added while sending stays), and sending waits while one is still on
// its way.

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
// Holds the server's answer until released, for a file added while sending.
let answer: Promise<void> = Promise.resolve();
let previous: Parameters<typeof setFetch>[0];
beforeAll(() => {
  previous = setFetch(async (_input, init) => {
    if (init?.method === "POST") sent.push(JSON.parse(String(init.body)));
    await answer;
    return new Response(JSON.stringify({ id: 1 }), { status: 202 });
  });
});
afterAll(() => {
  setFetch(previous);
});

function mount(attachments: Attachments) {
  const client = createQueryClient();
  const tree = (given: Attachments) => (
    <QueryClientProvider client={client}>
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
          attachments={given}
        />
      </I18nProvider>
    </QueryClientProvider>
  );
  const { rerender } = render(tree(attachments));
  return (next: Attachments) => rerender(tree(next));
}

const picture = new File(["x"], "logo.png");
const ready = (key: string, item: number) => ({
  key,
  file: picture,
  kind: "image" as const,
  status: { phase: "ready" as const, progress: 1, item },
});

test("a message carries the attachments' library ids, and the tray lets go of them once it is sent", async () => {
  sent.length = 0;
  const cleared: (readonly string[])[] = [];
  mount({
    ...noAttachments,
    list: [ready("a", 7)],
    items: [7],
    clear: (keys) => void cleared.push(keys),
  });
  expect(screen.getByText("logo.png")).toBeTruthy();
  await userEvent.type(screen.getByRole("textbox"), "use the logo{Enter}");
  await waitFor(() => expect(cleared).toEqual([["a"]]));
  expect(sent).toEqual([{ prompt: "use the logo", fresh: false, attachments: [7] }]);
});

test("a file attached while the message is on its way stays for the next one", async () => {
  sent.length = 0;
  let release = () => {};
  answer = new Promise((resolve) => {
    release = resolve;
  });
  const cleared: (readonly string[])[] = [];
  const clear = (keys: readonly string[]) => void cleared.push(keys);
  const update = mount({ ...noAttachments, list: [ready("a", 7)], items: [7], clear });
  await userEvent.type(screen.getByRole("textbox"), "use the logo{Enter}");
  await waitFor(() => expect(sent).toHaveLength(1));
  update({ ...noAttachments, list: [ready("a", 7), ready("b", 8)], items: [7, 8], clear });
  release();
  answer = Promise.resolve();
  await waitFor(() => expect(cleared).toEqual([["a"]]));
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
