// The entry chunk's budget (#896): a hard gate inside `vite build`, so inside
// `make web` and CI's `web` job, the same kind of rule as the size gate — one
// number, in one place, refusing the build when crossed.
//
// What it weighs is what a first visit downloads before any page can draw:
// the entry chunk and every chunk it imports statically, gzipped (roughly
// what crosses the wire; Cloudflare sends brotli, a little smaller still). A
// page's own chunk is not in it: pages are lazy (`src/app/pages.ts`) and the
// editor growing is expected. A heavy dependency pulled into the entry — by a
// page imported statically, or by the shell — is what this catches.
//
// The budget is the entry measured after the split, 193 KB gzipped on
// 2026-10-08 (258 KB when everything was one chunk), plus about an eighth for
// the shell to grow into. Most of it is react-dom, React Router, TanStack Query, Uppy with
// tus-js-client and the hasher, and the Radix menus the header uses. Raising it
// is a deliberate edit to this line, with the reason in the pull request;
// moving the new weight into a lazy chunk is usually the better answer.

import { gzipSync } from "node:zlib";

/** The most the entry and its static imports may weigh, gzipped, in bytes. */
export const ENTRY_BUDGET = 220 * 1024;

/** The parts of a Rolldown output chunk the budget reads. */
interface Chunk {
  type: "chunk";
  isEntry: boolean;
  imports: string[];
  code: string;
}

/** A bundle as `generateBundle` sees it: chunks and assets by file name. */
export type Bundle = Record<string, Chunk | { type: "asset" }>;

/** The entry chunk and every chunk reachable from it by static imports. */
export function entryFiles(bundle: Bundle): string[] {
  const entries = Object.entries(bundle).filter(([, out]) => out.type === "chunk" && out.isEntry);
  const seen = new Set<string>();
  const walk = (file: string) => {
    const out = bundle[file];
    if (seen.has(file) || out?.type !== "chunk") return;
    seen.add(file);
    for (const next of out.imports) walk(next);
  };
  for (const [file] of entries) walk(file);
  return [...seen].sort();
}

/** The gzipped weight of `files`, in bytes. */
export function gzippedSize(bundle: Bundle, files: string[]): number {
  let total = 0;
  for (const file of files) {
    const out = bundle[file];
    if (out?.type === "chunk") total += gzipSync(out.code, { level: 9 }).length;
  }
  return total;
}

const kb = (bytes: number) => `${(bytes / 1024).toFixed(1)} KB`;

/** The Vite plugin: weighs the entry after bundling and fails the build over budget. */
export function entryBudget(budget = ENTRY_BUDGET) {
  return {
    name: "scorsese-entry-budget",
    apply: "build" as const,
    generateBundle(this: { error: (message: string) => never }, _: unknown, bundle: Bundle) {
      const files = entryFiles(bundle);
      const size = gzippedSize(bundle, files);
      if (size > budget) {
        this.error(
          `the entry chunk is ${kb(size)} gzipped, over its ${kb(budget)} budget ` +
            `(src/build/budget.ts says why it is that number). It is ${files.join(", ")}. ` +
            "Load the new weight lazily, or raise the budget on purpose.",
        );
      }
      console.log(`entry budget: ${kb(size)} of ${kb(budget)} gzipped (${files.length} chunks)`);
    },
  };
}
