import { expect, test } from "bun:test";
import { type Bundle, entryBudget, entryFiles, gzippedSize } from "./budget";

const chunk = (code: string, imports: string[] = [], isEntry = false) =>
  ({ type: "chunk", isEntry, imports, code }) as const;

const bundle: Bundle = {
  "index.js": chunk("entry", ["shared.js"], true),
  "shared.js": chunk("shared", ["deep.js"]),
  "deep.js": chunk("deep"),
  "EditorPage.js": chunk("x".repeat(10_000), ["shared.js"]),
  "index.css": { type: "asset" },
};

test("the entry is its chunk and every chunk it imports statically, never a lazy page", () => {
  expect(entryFiles(bundle)).toEqual(["deep.js", "index.js", "shared.js"]);
});

test("a cycle between chunks is weighed once", () => {
  const cyclic: Bundle = { "a.js": chunk("a", ["b.js"], true), "b.js": chunk("b", ["a.js"]) };
  expect(entryFiles(cyclic)).toEqual(["a.js", "b.js"]);
});

test("the weight is gzipped and counts only the files asked for", () => {
  const entry = gzippedSize(bundle, entryFiles(bundle));
  expect(entry).toBeGreaterThan(0);
  expect(entry).toBeLessThan("entrysharedddeep".length * 3 + 100);
  expect(gzippedSize(bundle, ["EditorPage.js"])).toBeLessThan(10_000);
});

function build(budget: number) {
  const errors: string[] = [];
  const context = {
    error: (message: string): never => {
      errors.push(message);
      throw new Error(message);
    },
  };
  try {
    entryBudget(budget).generateBundle.call(context, undefined, bundle);
  } catch {}
  return errors;
}

test("the build fails over budget, naming the chunks, and passes at it", () => {
  const exact = gzippedSize(bundle, entryFiles(bundle));
  expect(build(exact)).toEqual([]);
  const [error] = build(exact - 1);
  expect(error).toContain("over its");
  expect(error).toContain("index.js, shared.js");
});
