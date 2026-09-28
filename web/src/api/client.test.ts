// The API client against a fake transport: what it sends, and how each kind
// of answer — JSON, empty, refused with fields, unreachable — comes back.

import { afterEach, expect, test } from "bun:test";
import { api } from "@/api";
import { ApiError, type Fetch, query, request, setFetch } from "./client";

let restore: Fetch | null = null;
const sent: { url: string; init?: RequestInit }[] = [];

function answer(status: number, body?: unknown) {
  sent.length = 0;
  restore = setFetch(async (url, init) => {
    sent.push({ url, init });
    const text = body === undefined ? null : JSON.stringify(body);
    return new Response(text, { status });
  });
}

afterEach(() => {
  if (restore) setFetch(restore);
  restore = null;
});

test("a request goes under /api with a JSON body, and its JSON comes back", async () => {
  answer(200, { id: 1, email: "ana@example.com", created_at: 0 });
  const account = await api.account.login("ana@example.com", "secret");
  expect(account.email).toBe("ana@example.com");
  expect(sent[0]?.url).toBe("/api/login");
  expect(sent[0]?.init?.method).toBe("POST");
  expect(JSON.parse(String(sent[0]?.init?.body))).toEqual({
    email: "ana@example.com",
    password: "secret",
  });
});

test("an empty answer resolves to undefined", async () => {
  answer(204);
  expect(await request<void>("DELETE", "/library/3")).toBeUndefined();
});

test("a refusal carries the server's words and the fields beside them", async () => {
  answer(409, { error: "it is used by “teaser”", projects: [{ id: 2, name: "teaser" }] });
  const error = await api.library.remove(3).catch((e) => e);
  expect(error).toBeInstanceOf(ApiError);
  expect(error.status).toBe(409);
  expect(error.message).toBe("it is used by “teaser”");
  expect(error.body.projects).toEqual([{ id: 2, name: "teaser" }]);
});

test("a refusal without a body still says something", async () => {
  answer(401);
  const error = await api.account.me().catch((e) => e);
  expect(error.status).toBe(401);
  expect(error.message).toBe("you are not logged in");
});

test("an unreachable server is status 0, not a crash", async () => {
  restore = setFetch(async () => {
    throw new TypeError("Failed to fetch");
  });
  const error = await api.projects.list().catch((e) => e);
  expect(error).toBeInstanceOf(ApiError);
  expect(error.status).toBe(0);
});

test("filters become a query string, and cleared ones disappear", async () => {
  expect(query({})).toBe("");
  expect(query({ kind: "video", search: "", project: undefined, before: null })).toBe(
    "?kind=video",
  );
  answer(200, []);
  await api.library.list({ search: "intro clip", project: 4 });
  expect(sent[0]?.url).toBe("/api/library?search=intro+clip&project=4");
  answer(200, { rows: [] });
  await api.credits.history({ kind: "veo_shot", since: "2026-09-01", before: 90, limit: 100 });
  expect(sent[0]?.url).toBe(
    "/api/credits/history?kind=veo_shot&since=2026-09-01&before=90&limit=100",
  );
});
