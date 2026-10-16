// A crash reaches the server once (#1428): a thrown error is posted to the
// crash route a single time, however many handlers see it, and a loop of
// crashes stops being told.

import { afterEach, beforeEach, expect, test } from "bun:test";
import { setFetch } from "@/api/client";
import type { ClientError } from "@/api/types";
import { listenForCrashes, reporter } from "./crashes";

let posted: { url: string; method?: string; body: ClientError }[] = [];
let previous: Parameters<typeof setFetch>[0];

beforeEach(() => {
  posted = [];
  previous = setFetch(async (url, init) => {
    posted.push({ url, method: init?.method, body: JSON.parse(String(init?.body)) });
    return new Response(null, { status: 204 });
  });
});

afterEach(() => {
  setFetch(previous);
});

/** Let the posts the reporter started go out. */
const settled = () => new Promise((resolve) => setTimeout(resolve, 0));

test("a thrown error is posted to the crash route once, with where it happened", async () => {
  const report = reporter();
  const error = new TypeError("busy.includes is not a function");
  report(error, "\n    in Busy\n    in ChatPanel");
  report(error);
  await settled();
  expect(posted.length).toBe(1);
  const { url, method, body } = posted[0] ?? { url: "", body: { message: "" } };
  expect({ url, method }).toEqual({ url: "/api/client-errors", method: "POST" });
  expect(body.message).toBe("TypeError: busy.includes is not a function");
  expect(body.component_stack).toBe("in Busy\n    in ChatPanel");
  expect(body.stack).toContain("busy.includes");
  expect(body.path).toBe(window.location.pathname);
  expect(body.build).toBe("dev");
});

test("the window's errors and rejections are reported, each once", async () => {
  const report = reporter();
  const stop = listenForCrashes(window, report);
  try {
    const error = new Error("thrown outside React");
    window.dispatchEvent(new ErrorEvent("error", { error, message: error.message }));
    // React's boundary and the window can both see one error.
    report(error);
    const reason = "a promise nobody awaited";
    const rejection = new Event("unhandledrejection") as PromiseRejectionEvent;
    Object.assign(rejection, { reason });
    window.dispatchEvent(rejection);
  } finally {
    stop();
  }
  window.dispatchEvent(new ErrorEvent("error", { error: new Error("after stopping") }));
  await settled();
  expect(posted.map(({ body }) => body.message)).toEqual([
    "Error: thrown outside React",
    "a promise nobody awaited",
  ]);
});

test("a crash in a loop stops being told, and a failed post throws nothing", async () => {
  const report = reporter(async () => {
    throw new Error("the server is down too");
  }, 2);
  const sent: string[] = [];
  const counting = reporter(async (body) => sent.push(body.message), 2);
  for (let i = 0; i < 5; i++) {
    report(new Error(`crash ${i}`));
    counting(new Error(`crash ${i}`));
  }
  await settled();
  expect(sent).toEqual(["Error: crash 0", "Error: crash 1"]);
});

test("a long stack is cut to its first lines", async () => {
  const error = new Error("deep");
  error.stack = Array.from({ length: 100 }, (_, i) => `at frame${i}`).join("\n");
  reporter()(error);
  await settled();
  expect(posted[0]?.body.stack?.split("\n").length).toBe(20);
});
