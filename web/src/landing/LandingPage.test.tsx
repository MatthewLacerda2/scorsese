// The landing page at its URLs (#904): who sees it, and the sign-in popup
// that replaced the login page — opened by its button, already open at
// `/login`, its one "Continue with Google" link (#952) carrying `?next=`
// through, and a failed sign-in's `?error=` said in words. And what `/` draws
// while the session is asked, with and without the hint (#1246).

import { afterEach, expect, test } from "bun:test";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { App } from "@/App";
import type { Account } from "@/api";
import { setFetch } from "@/api/client";
import { createQueryClient, ME } from "@/app/queryClient";
import { forgetSignedInHint, HINT_COOKIE, signedInHint } from "@/session/hint";

const LANDING = "The video is one conversation away";
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
  forgetSignedInHint();
});

/** `GET /api/me`, held until the test answers it with an account or a `401`. */
function heldSession(): (account: Account | null) => void {
  let answer: (account: Account | null) => void = () => {};
  setFetch(
    () =>
      new Promise((resolve) => {
        answer = (account) =>
          resolve(
            account ? Response.json(account) : Response.json({ error: "no" }, { status: 401 }),
          );
      }),
  );
  return (account) => answer(account);
}

test("without a session, / is the landing page", async () => {
  visit("/", null);
  expect(await screen.findByRole("heading", { name: LANDING })).toBeDefined();
  // One prompt box on the first screen, one in the closing section.
  expect(screen.getAllByRole("textbox", { name: "Describe your video" }).length).toBe(2);
  expect(screen.getByText("The video is one message away")).toBeDefined();
  // Everything the page said before is still on it, below the prompt box.
  expect(screen.getByText("Three steps. You just say what you want.")).toBeDefined();
  expect(screen.getByText("Ready when you are.")).toBeDefined();
  // How it works comes before the film.
  const how = screen.getByText("Three steps. You just say what you want.");
  const film = screen.getByLabelText("Videos made with scorsese", { selector: "video" });
  expect(how.compareDocumentPosition(film) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  expect(screen.queryByRole("dialog")).toBeNull();
});

test("with a session, / is the home with its sidebar", async () => {
  visit("/", ana);
  expect(await screen.findByRole("navigation", { name: "Main menu" })).toBeDefined();
  expect(screen.queryByRole("heading", { name: LANDING })).toBeNull();
});

test("/ draws the landing page before the session answers, and leaves once it says signed in", async () => {
  const answer = heldSession();
  visit("/");
  expect(await screen.findByRole("heading", { name: LANDING })).toBeDefined();
  answer(ana);
  expect(await screen.findByRole("navigation", { name: "Main menu" })).toBeDefined();
  expect(screen.queryByRole("heading", { name: LANDING })).toBeNull();
});

test("signed in here before, / waits on the session and never draws the landing page (#1246)", async () => {
  document.cookie = `${HINT_COOKIE}=1; Path=/`;
  const answer = heldSession();
  visit("/");
  await new Promise((resolve) => setTimeout(resolve, 50));
  // Text and a count, not elements: a failure prints them, not a whole DOM node.
  expect(document.body.textContent).not.toContain(LANDING);
  expect(document.querySelectorAll("video").length).toBe(0);
  answer(ana);
  expect(await screen.findByRole("navigation", { name: "Main menu" })).toBeDefined();
});

test("a hint the session no longer stands behind costs the wait, then is dropped", async () => {
  document.cookie = `${HINT_COOKIE}=1; Path=/`;
  const answer = heldSession();
  visit("/");
  answer(null);
  expect(await screen.findByRole("heading", { name: LANDING })).toBeDefined();
  expect(signedInHint()).toBe(false);
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
  expect(screen.getByRole("heading", { name: LANDING, hidden: true })).toBeDefined();
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

test("Enter in a landing prompt box opens the sign-in popup and keeps what was typed", async () => {
  visit("/", null);
  const [box] = await screen.findAllByRole("textbox", { name: "Describe your video" });
  await userEvent.click(box as HTMLElement);
  await userEvent.keyboard("{Enter}");
  expect(screen.queryByRole("dialog")).toBeNull();
  await userEvent.type(box as HTMLElement, "a promo for my bakery{Enter}");
  expect(await screen.findByRole("dialog")).toBeDefined();
  expect((box as HTMLTextAreaElement).value).toBe("a promo for my bakery");
});
