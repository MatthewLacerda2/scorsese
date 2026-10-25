// A sound clip's waveform (#1572), beyond the slice `clipfill.test.tsx`
// checks: peaks the server is still reading are asked for again when the job
// reading them is done, and a long sound drawn wide stays a few thousand bars.

import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { act, render, waitFor } from "@testing-library/react";
import { createQueryClient } from "@/app/queryClient";
import { ClipFill } from "./ClipFill";

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

describe("a sound's peaks still being read (#1572)", () => {
  const realSource = globalThis.EventSource;
  const realFetch = globalThis.fetch;
  beforeAll(() => {
    globalThis.EventSource = FakeSource as unknown as typeof EventSource;
  });
  afterAll(() => {
    globalThis.EventSource = realSource;
    globalThis.fetch = realFetch;
  });

  test("are asked for again when the job reading them is done", async () => {
    let read = false;
    let asked = 0;
    globalThis.fetch = (async () => {
      asked += 1;
      if (!read) return new Response('{"pending":true}', { status: 404 });
      const bytes = new Uint8Array([0, 0, 0x80, 0x40, 255, 51]); // 4 a second
      return new Response(bytes);
    }) as unknown as typeof fetch;
    const client = createQueryClient();
    client.setQueryData(
      ["library", "list", { sha256: "dd" }],
      [{ id: 9, name: "s", kind: "audio", size_bytes: 1, thumbnail: "/t/9", peaks: "/p/9" }],
    );
    const { container } = render(
      <QueryClientProvider client={client}>
        <ClipFill
          asset={{ id: "a", kind: "audio", sha256: "dd", path: "a.wav" }}
          span={{ from: 0, length: 0.5, width: 30 }}
        />
      </QueryClientProvider>,
    );
    await waitFor(() => expect(FakeSource.last).not.toBeNull());
    expect(asked).toBe(1);
    expect(container.querySelectorAll("[data-bar]").length).toBe(0);

    read = true;
    const done = { type: "job", id: 3, kind: "thumbnail", state: "done", result: { item: 9 } };
    act(() => FakeSource.last?.onmessage?.({ data: JSON.stringify(done) }));
    await waitFor(() => expect(container.querySelectorAll("[data-bar]").length).toBe(2));
  });
});

describe("a long sound drawn wide (#1572)", () => {
  test("draws no more bars than its levels", () => {
    const client = createQueryClient();
    client.setQueryData(
      ["library", "list", { sha256: "ee" }],
      [{ id: 9, name: "s", kind: "audio", size_bytes: 1, thumbnail: "/t/9", peaks: "/p/e" }],
    );
    client.setQueryData(["library", "peaks", "/p/e"], {
      perSecond: 8,
      levels: new Uint8Array(8 * 600),
    });
    const { container } = render(
      <QueryClientProvider client={client}>
        <ClipFill
          asset={{ id: "a", kind: "audio", sha256: "ee", path: "a.wav" }}
          span={{ from: 0, length: 600, width: 192_000 }}
        />
      </QueryClientProvider>,
    );
    expect(container.querySelectorAll("[data-bar]").length).toBe(2048);
  });
});
