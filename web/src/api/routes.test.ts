// Every server route the front-end names is one the server has (#1227).
// docs/web.md's route tables are held to the server's router in both
// directions (`crates/server/src/http/routes.rs`, #994), so they are the list
// to check against: readable here without Rust, and true by that test. A call
// to a route that does not exist then fails here, before an e2e flow would.
//
// What counts as naming a route: every `request("METHOD", path)` in src/api,
// and every `/api…` literal there (a URL the browser follows by itself — the
// event stream, tus's endpoint, a file's `src`), which is checked by path
// against any method, since the browser picks it.
import { expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { calls, documented, matches, pathOf } from "./routes";

const HERE = import.meta.dir;
const WEB_MD = readFileSync(join(HERE, "..", "..", "..", "docs", "web.md"), "utf8");
const SOURCES = readdirSync(HERE)
  // routes.ts is the checker, not a caller: its patterns are not routes.
  .filter((name) => /\.tsx?$/.test(name) && !/\.test\.tsx?$/.test(name) && name !== "routes.ts")
  .map((name) => ({ name, text: readFileSync(join(HERE, name), "utf8") }));

const rows = documented(WEB_MD);
const named = SOURCES.flatMap(({ name, text }) =>
  calls(text).map((call) => ({ ...call, file: name })),
);

test("the route tables and the calls are found at all", () => {
  expect(rows.length).toBeGreaterThan(40);
  expect(named.filter((call) => call.method !== null).length).toBeGreaterThan(30);
  expect(named.filter((call) => call.method === null).length).toBeGreaterThan(3);
});

test("every route the front-end names is in docs/web.md", () => {
  const missing = named
    .filter((call) => !rows.some((row) => matches(row, call)))
    .map((call) => `${call.file}: ${call.method ?? "any"} ${call.path}`);
  expect(missing).toEqual([]);
});

test("a row's parameters and a literal's are written alike", () => {
  const md = "| `GET /api/library/{id}/file` | a member | x |\n| `POST /api/x` y |\n";
  expect(documented(md)).toEqual([{ method: "GET", path: "/api/library/{}/file" }]);
  expect(pathOf("`/library/${id}/file`")).toBe("/library/{}/file");
  expect(pathOf("`/credits/history${query({ ...filter })}`")).toBe("/credits/history");
  expect(pathOf("`/payments/${encodeURIComponent(s)}/confirm`")).toBe("/payments/{}/confirm");
  expect(pathOf('"/library/7/file?download=1"')).toBe("/library/7/file");
});

test("a call is found across lines, and a URL apart from it", () => {
  const source = `
    request<{ a: B | null }>(
      "POST",
      \`/projects/\${id}/renders\`,
      settings,
    ),
    url: "/api/events",
  `;
  expect(calls(source)).toEqual([
    { method: "POST", path: "/api/projects/{}/renders" },
    { method: null, path: "/api/events" },
  ]);
});

test("a route is matched by path, and by method unless the browser picks it", () => {
  const row = { method: "PATCH", path: "/api/uploads/{}" };
  expect(matches(row, { method: "PATCH", path: "/api/uploads/{}" })).toBe(true);
  expect(matches(row, { method: "GET", path: "/api/uploads/{}" })).toBe(false);
  expect(matches(row, { method: null, path: "/api/uploads/{}" })).toBe(true);
  expect(matches(row, { method: null, path: "/api/uploads" })).toBe(false);
});
