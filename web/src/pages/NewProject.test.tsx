// Create with no name (#770): it looks unavailable but still answers a click,
// with a red line saying what is missing — and creates nothing. Rendered to a
// string like the other component tests: there is no DOM to click in, so the
// click is `attemptCreate` and the form is drawn in each state it leads to.

import { expect, test } from "bun:test";
import { renderToString } from "react-dom/server";
import { attemptCreate, NewProjectForm } from "./ProjectsPage";

function form(name: string, missing: boolean, pending = false) {
  return renderToString(
    <NewProjectForm
      name={name}
      missing={missing}
      pending={pending}
      error={null}
      onName={() => {}}
      onSubmit={() => {}}
    />,
  );
}

function button(html: string) {
  return html.match(/<button[^>]*>/)?.[0] ?? "";
}

test("a blank name creates nothing and says the name is missing", () => {
  const created: string[] = [];
  expect(attemptCreate("", (name) => created.push(name))).toBe(true);
  expect(attemptCreate("   ", (name) => created.push(name))).toBe(true);
  expect(created).toEqual([]);
});

test("a name is created trimmed, as before", () => {
  const created: string[] = [];
  expect(attemptCreate("  Beach trip ", (name) => created.push(name))).toBe(false);
  expect(created).toEqual(["Beach trip"]);
});

test("with no name, Create looks unavailable but can still be clicked", () => {
  const create = button(form("", false));
  expect(create).toContain('aria-disabled="true"');
  expect(create).toContain("font-normal");
  expect(create).not.toMatch(/ disabled=""/);
});

test("the missing name is said in red under the field, and the field is marked", () => {
  const html = form("", true);
  expect(html).toContain("Write the name of the project");
  expect(html).toContain("text-destructive");
  expect(html).toContain('aria-invalid="true"');
  expect(html).toContain('placeholder="Project&#x27;s name"');
});

test("typing a name clears the message and Create reads as available", () => {
  // `onName` clears `missing`; this is the form it then draws.
  const html = form("Beach trip", false);
  expect(html).not.toContain("Write the name of the project");
  expect(html).not.toContain("aria-invalid=");
  expect(button(html)).not.toContain("aria-disabled=");
});

test("while a create is in flight, Create is truly disabled", () => {
  expect(button(form("Beach trip", false, true))).toMatch(/ disabled=""/);
});
