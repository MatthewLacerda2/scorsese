// The prompt box's attachments, signed out (the landing): the "+" and a drop
// put files in the tray above the text, a file the library does not take is
// refused by name, × takes one off, and sending hands the files over. A drag
// that is not files — the editor's own — is left alone.

import { afterAll, beforeAll, expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { Formats } from "@/api";
import { setFetch } from "@/api/client";
import { createQueryClient } from "@/app/queryClient";
import { I18nProvider } from "@/i18n/I18nProvider";
import { type Attached, PromptBox } from "@/prompt/PromptBox";

const formats: Formats = {
  video: ["mp4"],
  image: ["png"],
  audio: ["mp3"],
  html: ["html"],
  text: ["md"],
};

let previous: Parameters<typeof setFetch>[0];
beforeAll(() => {
  previous = setFetch(async () => new Response(JSON.stringify(formats)));
});
afterAll(() => {
  setFetch(previous);
});

async function setup() {
  const sent: [string, Attached][] = [];
  const client = createQueryClient();
  render(
    <QueryClientProvider client={client}>
      <I18nProvider initial="en">
        <PromptBox onSubmit={(text, attached) => void sent.push([text, attached])} />
      </I18nProvider>
    </QueryClientProvider>,
  );
  return { sent, client, input: screen.getByTestId("attach-input") as HTMLInputElement };
}

const file = (name: string) => new File(["x"], name);
const tray = () => within(screen.getByRole("list", { name: "Attached files" }));

test("the list is asked for only once someone reaches for the box", async () => {
  const { client, input } = await setup();
  expect(client.getQueryData(["formats"])).toBeUndefined();
  await userEvent.hover(screen.getByRole("textbox"));
  await waitFor(() => expect(input.accept).toBe(".mp4,.png,.mp3,.html,.md"));
});

test("the + picks files into the tray", async () => {
  const { input } = await setup();
  await userEvent.upload(input, [file("trip.mp4"), file("notes.md")]);
  expect(await tray().findByText("trip.mp4")).toBeTruthy();
  expect(tray().getByText("notes.md")).toBeTruthy();
});

test("a file the library does not take is refused by name, the others kept", async () => {
  const { input } = await setup();
  // The picker's hint is only a hint: "All files" lets anything through.
  await userEvent.upload(input, [file("tune.mid"), file("logo.png")], { applyAccept: false });
  expect((await screen.findByRole("alert")).textContent).toBe(
    "tune.mid: scorsese can't use this kind of file",
  );
  expect(tray().getByText("logo.png")).toBeTruthy();
  expect(tray().queryByText("tune.mid")).toBeNull();
});

test("× takes a file off the prompt", async () => {
  const { input } = await setup();
  await userEvent.upload(input, [file("a.png"), file("b.png")]);
  await userEvent.click(await screen.findByRole("button", { name: "Remove a.png" }));
  expect(tray().queryByText("a.png")).toBeNull();
  expect(tray().getByText("b.png")).toBeTruthy();
});

test("signed out, sending hands over the files held, with no library items", async () => {
  const { sent, input } = await setup();
  await userEvent.upload(input, [file("a.png")]);
  await screen.findByText("a.png");
  await userEvent.type(screen.getByRole("textbox"), "a trailer{Enter}");
  expect(sent).toHaveLength(1);
  const [text, attached] = sent[0] as [string, Attached];
  expect(text).toBe("a trailer");
  expect(attached.files.map((f) => f.name)).toEqual(["a.png"]);
  expect(attached.items).toEqual([]);
});

test("files dropped on the box are attached, with a drop-here look while dragged", async () => {
  await setup();
  const box = screen.getByRole("textbox").parentElement as HTMLElement;
  const files = [file("clip.mp4")];
  const dataTransfer = { types: ["Files"], files, dropEffect: "none" };
  fireEvent.dragEnter(box, { dataTransfer });
  expect(screen.getByText("Drop files to attach them")).toBeTruthy();
  fireEvent.drop(box, { dataTransfer });
  expect(screen.queryByText("Drop files to attach them")).toBeNull();
  expect(await screen.findByText("clip.mp4")).toBeTruthy();
});

test("a drag that is not files is neither taken nor prevented", async () => {
  await setup();
  const box = screen.getByRole("textbox").parentElement as HTMLElement;
  const dataTransfer = { types: ["application/x-scorsese-asset"], files: [] };
  fireEvent.dragEnter(box, { dataTransfer });
  expect(screen.queryByText("Drop files to attach them")).toBeNull();
  // `fireEvent` answers false when a handler called `preventDefault`.
  expect(fireEvent.dragOver(box, { dataTransfer })).toBe(true);
  expect(fireEvent.drop(box, { dataTransfer })).toBe(true);
  expect(screen.queryByRole("list", { name: "Attached files" })).toBeNull();
});
