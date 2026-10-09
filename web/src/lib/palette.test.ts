// No raw Tailwind palette colour outside the shadcn components (#959): a
// colour on the theme comes from index.css's tokens, picked once per theme and
// named for what it means (`bg-warning`, not `bg-amber-500`). Black and white
// are not caught — over a picture (a scrim, a badge on a thumbnail) a fixed
// colour is right, since the picture does not follow the theme.

import { expect, test } from "bun:test";
import { Glob } from "bun";

const RAW =
  /\b(?:bg|text|border(?:-[trblxy])?|ring|outline|fill|stroke|from|via|to|shadow|divide|accent|caret|decoration)-(?:red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose|slate|gray|zinc|neutral|stone)-\d{2,3}\b/g;

/** Where a raw colour is the point, and why. */
const ALLOWED = [
  // shadcn's own components, kept as the generator writes them.
  "components/ui/",
  // Each asset kind's hue, the desktop app's palette: an identity, like a
  // chart's series colours, the same in both themes.
  "editor/assets/kinds.ts",
];

test("no raw palette colour outside the allowed files", async () => {
  const found: string[] = [];
  for await (const path of new Glob("**/*.{ts,tsx}").scan(import.meta.dir + "/..")) {
    if (/\.test\.tsx?$/.test(path) || ALLOWED.some((allowed) => path.startsWith(allowed))) {
      continue;
    }
    const text = await Bun.file(`${import.meta.dir}/../${path}`).text();
    for (const match of text.matchAll(RAW)) found.push(`${path}: ${match[0]}`);
  }
  expect(found).toEqual([]);
});

test("the pattern catches what it is for", () => {
  for (const raw of ["bg-red-500", "border-sky-500/50", "hover:text-amber-600", "ring-zinc-50"]) {
    expect(raw.match(RAW)).not.toBeNull();
  }
  for (const fine of ["bg-black/70", "text-white", "bg-warning/5", "ring-info", "bg-playhead"]) {
    expect(fine.match(RAW)).toBeNull();
  }
});
