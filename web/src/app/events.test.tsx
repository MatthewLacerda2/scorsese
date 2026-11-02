// A stream the server closed for a newer one (`displaced`) is not reopened at
// once, which would close the newer one in turn, but when its page is shown.

import { afterAll, beforeAll, expect, test } from "bun:test";
import { act, render } from "@testing-library/react";
import type { ServerEvent } from "@/api/events";
import { useServerEvents } from "./events";

class FakeSource {
  static opened: FakeSource[] = [];
  closed = false;
  onopen: (() => void) | null = null;
  onmessage: ((message: { data: string }) => void) | null = null;
  constructor() {
    FakeSource.opened.push(this);
  }
  close() {
    this.closed = true;
  }
  say(event: ServerEvent) {
    this.onmessage?.({ data: JSON.stringify(event) });
  }
}

const real = globalThis.EventSource;
beforeAll(() => {
  globalThis.EventSource = FakeSource as unknown as typeof EventSource;
});
afterAll(() => {
  globalThis.EventSource = real;
});

function setVisibility(state: DocumentVisibilityState) {
  Object.defineProperty(document, "visibilityState", { configurable: true, value: state });
  document.dispatchEvent(new Event("visibilitychange"));
}

function Listening({ heard }: { heard: ServerEvent[] }) {
  useServerEvents((event) => heard.push(event));
  return null;
}

test("a displaced stream reopens when its page is next shown, and resyncs", () => {
  const heard: ServerEvent[] = [];
  setVisibility("hidden");
  const mounted = render(<Listening heard={heard} />);
  expect(FakeSource.opened).toHaveLength(1);
  const first = FakeSource.opened[0];

  act(() => first.say({ type: "displaced" }));
  expect(first.closed).toBe(true);
  expect(heard).toEqual([]);
  setVisibility("hidden");
  expect(FakeSource.opened).toHaveLength(1);

  act(() => setVisibility("visible"));
  expect(FakeSource.opened).toHaveLength(2);
  expect(heard).toEqual([{ type: "resync" }]);
  act(() => setVisibility("visible"));
  expect(FakeSource.opened).toHaveLength(2);
  mounted.unmount();
});
