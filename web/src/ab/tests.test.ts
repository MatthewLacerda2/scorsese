// The one-running-test rule (#1553), held over the real list and shown on a
// list that breaks it.

import { expect, test } from "bun:test";
import { type AbTest, problems, TESTS } from "./tests";

const heading = (running: boolean): AbTest => ({ id: "heading", versions: ["a", "b"], running });

test("the landing's own list keeps every rule", () => {
  expect(problems(TESTS)).toEqual([]);
});

test("one running test with others waiting passes", () => {
  const waiting: AbTest = { id: "film-strip", versions: ["a", "b"], running: false };
  expect(problems([heading(true), waiting])).toEqual([]);
});

test("two running at once is refused", () => {
  const strip: AbTest = { id: "film-strip", versions: ["a", "b"], running: true };
  expect(problems([heading(true), strip])).toEqual([
    "at most one test runs at a time, and heading, film-strip all do",
  ]);
});

test("a test the server would not keep is refused", () => {
  expect(
    problems([
      { id: "Heading", versions: ["a", "b"], running: false },
      { id: "solo", versions: ["a"], running: false },
      { id: "twice", versions: ["a", "a"], running: false },
      { id: "twice", versions: ["a", "B"], running: false },
    ]),
  ).toEqual([
    "Heading: an id is [a-z0-9-], at most 32",
    "solo needs two versions or more",
    "twice names a version twice",
    "twice is defined twice",
    "twice: version B is not [a-z0-9-]",
  ]);
});
