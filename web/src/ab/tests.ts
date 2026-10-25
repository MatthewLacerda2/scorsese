// The landing page's A/B tests (#1553): which are defined, and the one that is
// running. The server knows none of them (crates/server/src/ab); it keeps what
// each visitor was shown and did, and `scorsese-server ab report` decides a
// test by the rule docs/web.md's *A/B tests* states.
//
// One change at a time: the list may hold tests waiting their turn, but at
// most one is `running`, and `tests.test.ts` refuses a list with two. Deciding
// a test is a pull request that removes the losing version and the test's
// entry, and sets the next one running in the same change.

/** One test: its id, its versions (the first is the current page), and whether it runs. */
export interface AbTest {
  /** Lowercase letters, digits and `-`, at most 32: what the server keeps. */
  id: string;
  /** The versions' ids, shaped as `id` is; each drawn with equal weight. */
  versions: readonly string[];
  /** Whether visitors are being drawn into it now. */
  running: boolean;
}

/** Every test, in the order they run. */
export const TESTS: readonly AbTest[] = [
  // The first screen's heading (#1554): `a` the metaphor, `b` the concrete promise.
  { id: "hero-title", versions: ["a", "b"], running: true },
];

/**
 * A stand-in test for the end-to-end flows only (`make web-e2e` builds with
 * `VITE_AB_FIXTURE=1`), so the draw and the recording are proven before a real
 * test is defined. Its versions change nothing on the page.
 */
const FIXTURE: AbTest = { id: "e2e-fixture", versions: ["a", "b"], running: true };

/** The tests this build runs: `TESTS`, or the fixture in an end-to-end build. */
export function runningTests(): AbTest[] {
  const tests = import.meta.env.VITE_AB_FIXTURE === "1" ? [FIXTURE] : TESTS;
  return tests.filter((test) => test.running);
}

const ID = /^[a-z0-9-]{1,32}$/;

/** What is wrong with `tests`, one sentence each; empty when nothing is. */
export function problems(tests: readonly AbTest[]): string[] {
  const found: string[] = [];
  const running = tests.filter((test) => test.running).map((test) => test.id);
  if (running.length > 1)
    found.push(`at most one test runs at a time, and ${running.join(", ")} all do`);
  const seen = new Set<string>();
  for (const test of tests) {
    if (seen.has(test.id)) found.push(`${test.id} is defined twice`);
    seen.add(test.id);
    if (!ID.test(test.id)) found.push(`${test.id}: an id is [a-z0-9-], at most 32`);
    if (test.versions.length < 2) found.push(`${test.id} needs two versions or more`);
    if (new Set(test.versions).size !== test.versions.length)
      found.push(`${test.id} names a version twice`);
    for (const version of test.versions)
      if (!ID.test(version)) found.push(`${test.id}: version ${version} is not [a-z0-9-]`);
  }
  return found;
}
