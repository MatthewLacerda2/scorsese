// Every query key is written in app/queryKeys.ts and nowhere else: a key typed
// as an array literal at a call site is how two pages come to disagree on what
// an area's invalidation refetches. Tests may spell keys out.

import { expect, test } from "bun:test";
import { Glob } from "bun";

const LITERAL = /\bqueryKey:\s*\[|(?:setQueryData|getQueryData|removeQueries)(?:<[^>]*>)?\(\s*\[/g;

test("no query key is an array literal outside queryKeys.ts", async () => {
  const found: string[] = [];
  for await (const path of new Glob("**/*.{ts,tsx}").scan(import.meta.dir + "/..")) {
    if (/\.test\.tsx?$/.test(path) || path === "app/queryKeys.ts") continue;
    const text = await Bun.file(`${import.meta.dir}/../${path}`).text();
    for (const match of text.matchAll(LITERAL)) found.push(`${path}: ${match[0]}`);
  }
  expect(found).toEqual([]);
});

test("the pattern catches what it is for", () => {
  for (const bad of ['queryKey: ["a"]', 'client.setQueryData(["a"], 1)', "getQueryData<X>([1])"]) {
    expect(bad.match(LITERAL)).not.toBeNull();
  }
  for (const fine of ["queryKey: queryKeys.a", "setQueryData(ME, null)", "queryKey: key"]) {
    expect(fine.match(LITERAL)).toBeNull();
  }
});
