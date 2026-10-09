// The toast a finished render raises (#958), fed through the live stream as
// the server sends it: a stand-in `EventSource` carries the `job` messages.

import { afterAll, afterEach, beforeAll, describe, expect, test } from "bun:test";
import { act, render, screen } from "@testing-library/react";
import { toast } from "sonner";
import type { JobView, ServerEvent } from "@/api/events";
import { I18nProvider } from "@/i18n/I18nProvider";
import { RenderToaster, rememberRender } from "./RenderToasts";

/** The one stream `useServerEvents` opens, held so a test can speak on it. */
class FakeSource {
  static last: FakeSource | null = null;
  onopen: (() => void) | null = null;
  onmessage: ((message: { data: string }) => void) | null = null;
  constructor() {
    FakeSource.last = this;
  }
  close() {}
}

const real = globalThis.EventSource;
beforeAll(() => {
  globalThis.EventSource = FakeSource as unknown as typeof EventSource;
});
afterAll(() => {
  globalThis.EventSource = real;
});
// Sonner keeps its toasts in a module-level store that outlives a mount, and
// hands a new toaster every one not dismissed.
afterEach(() => {
  toast.dismiss();
});

const job = (changed: Partial<JobView>): ServerEvent => ({
  type: "job",
  id: 1,
  kind: "render",
  state: "done",
  attempts: 1,
  result: null,
  error: null,
  created_at: 0,
  started_at: 1,
  finished_at: 2,
  interrupted_at: null,
  ...changed,
});

/** The toaster, with project `open` in the editor (or none). */
function mount(open: number | null = null) {
  render(
    <I18nProvider initial="en">
      <RenderToaster open={open} />
    </I18nProvider>,
  );
}

function send(event: ServerEvent) {
  act(() => FakeSource.last?.onmessage?.({ data: JSON.stringify(event) }));
}

describe("a finished render", () => {
  test("says the video is ready, names its project and links the download", async () => {
    rememberRender(7, { id: 3, name: "Holiday cut" });
    mount(5);
    send(job({ id: 7, result: { render: 9, size: 10, file: "/api/renders/9/file" } }));
    expect(await screen.findByText("Your video is ready")).toBeTruthy();
    expect(screen.getByText("Holiday cut")).toBeTruthy();
    const link = screen.getByRole("link", { name: "Download" });
    expect(link.getAttribute("href")).toBe("/api/renders/9/file");
  });

  test("leaves the name out on the project it is of", async () => {
    rememberRender(8, { id: 3, name: "Holiday cut" });
    mount(3);
    send(job({ id: 8, result: { render: 9, size: 10, file: "/api/renders/9/file" } }));
    expect(await screen.findByText("Your video is ready")).toBeTruthy();
    expect(screen.queryByText("Holiday cut")).toBeNull();
  });

  test("says why a failed render failed", async () => {
    mount();
    send(job({ id: 10, state: "failed", error: "the project has nothing to render" }));
    expect(await screen.findByText("The render failed")).toBeTruthy();
    expect(screen.getByText("the project has nothing to render")).toBeTruthy();
  });

  test("raises nothing for a preview, nor for a render the person stopped", async () => {
    mount();
    send(job({ id: 11, kind: "preview", result: { file: "/api/renders/4/file" } }));
    send(job({ id: 12, state: "cancelled", error: "stopped at 40%" }));
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(screen.queryByText("Your video is ready")).toBeNull();
    expect(screen.queryByText("stopped at 40%")).toBeNull();
    expect(screen.queryByRole("listitem")).toBeNull();
  });
});
