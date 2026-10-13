// The landing page at its URLs (#904): who sees it, and the sign-in popup
// that replaced the login page — opened by its button, already open at
// `/login`, its one "Continue with Google" link (#952) carrying `?next=`
// through, and a failed sign-in's `?error=` said in words.

import { afterEach, expect, test } from "bun:test";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { App } from "@/App";
import type { Account } from "@/api";
import { setFetch } from "@/api/client";
import { createQueryClient, ME } from "@/app/queryClient";

const ana: Account = { id: 1, email: "ana@example.com", created_at: 0 };

/** Open `url`; `account` seeds the session, or leaves it for `GET /api/me` to answer when omitted. */
function visit(url: string, account?: Account | null) {
  const client = createQueryClient();
  if (account !== undefined) client.setQueryData(ME, account);
  client.setQueryData(["projects"], []);
  client.setQueryData(["credits", "balance"], { balance_micros: 0 });
  render(
    <App
      language="en"
      queryClient={client}
      router={(routes) => <MemoryRouter initialEntries={[url]}>{routes}</MemoryRouter>}
    />,
  );
}

const real = setFetch((input, init) => fetch(input, init));
afterEach(() => {
  setFetch(real);
});

test("without a session, / is the landing page", async () => {
  visit("/", null);
  expect(await screen.findByText("It gets made.")).toBeDefined();
  expect(screen.getByText("You see the price before anything is spent.")).toBeDefined();
  expect(screen.queryByRole("dialog")).toBeNull();
});

test("with a session, / is the projects list", async () => {
  visit("/", ana);
  expect(await screen.findByRole("heading", { name: "Projects" })).toBeDefined();
  expect(screen.queryByText("It gets made.")).toBeNull();
});

test("/ draws the landing page before the session answers, and leaves once it says signed in", async () => {
  let answer: (account: Account) => void = () => {};
  setFetch(() => new Promise((resolve) => (answer = (account) => resolve(Response.json(account)))));
  visit("/");
  expect(await screen.findByText("It gets made.")).toBeDefined();
  answer(ana);
  expect(await screen.findByRole("heading", { name: "Projects" })).toBeDefined();
  expect(screen.queryByText("It gets made.")).toBeNull();
});

test("the Sign in button opens the popup", async () => {
  visit("/", null);
  const [header] = await screen.findAllByRole("button", { name: "Sign in" });
  await userEvent.click(header as HTMLElement);
  const dialog = await screen.findByRole("dialog");
  expect(dialog.textContent).toContain("Welcome to scorsese");
  const google = screen.getByRole("link", { name: "Continue with Google" });
  expect(google.getAttribute("href")).toBe("/api/auth/google");
  expect(screen.queryByLabelText("Password")).toBeNull();
});

test("/login is the landing page with the popup already open", async () => {
  visit("/login?next=%2Flibrary", null);
  expect(await screen.findByRole("dialog")).toBeDefined();
  expect(screen.getByText("It gets made.")).toBeDefined();
  const google = screen.getByRole("link", { name: "Continue with Google" });
  expect(google.getAttribute("href")).toBe("/api/auth/google?next=%2Flibrary");
});

test("a next that leaves the site is not carried to the server", async () => {
  visit("/login?next=%2F%2Fevil.example", null);
  const google = await screen.findByRole("link", { name: "Continue with Google" });
  expect(google.getAttribute("href")).toBe("/api/auth/google?next=%2F");
});

test("a sign-in that came back with an error says why in words", async () => {
  visit("/login?error=linked", null);
  const alert = await screen.findByRole("alert");
  expect(alert.textContent).toContain("linked to a different Google account");
});

test("an error code the page does not know says nothing", async () => {
  visit("/login?error=%3Cscript%3E", null);
  expect(await screen.findByRole("dialog")).toBeDefined();
  expect(screen.queryByRole("alert")).toBeNull();
});
