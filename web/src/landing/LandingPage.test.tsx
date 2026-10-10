// The landing page at its URLs (#904): who sees it, and the sign-in popup
// that replaced the login page — opened by its button, already open at
// `/login`, and showing the server's own sentence for a wrong password and
// for the login brake's `429`.

import { afterEach, expect, test } from "bun:test";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { App } from "@/App";
import type { Account } from "@/api";
import { setFetch } from "@/api/client";
import { createQueryClient, ME } from "@/app/queryClient";

const ana: Account = { id: 1, email: "ana@example.com", created_at: 0 };

function visit(url: string, account: Account | null) {
  const client = createQueryClient();
  client.setQueryData(ME, account);
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

/** Answer every request, `POST /api/login` among them, with `status` and the server's `{"error"}`. */
function loginAnswers(status: number, error: string) {
  setFetch(async () => Response.json({ error }, { status }));
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

test("the Sign in button opens the popup", async () => {
  visit("/", null);
  const [header] = await screen.findAllByRole("button", { name: "Sign in" });
  await userEvent.click(header as HTMLElement);
  const dialog = await screen.findByRole("dialog");
  expect(dialog.textContent).toContain("Welcome back");
  expect(screen.getByLabelText("Email")).toBeDefined();
  expect(screen.getByLabelText("Password")).toBeDefined();
});

test("/login is the landing page with the popup already open", async () => {
  visit("/login?next=%2Flibrary", null);
  expect(await screen.findByRole("dialog")).toBeDefined();
  expect(screen.getByText("It gets made.")).toBeDefined();
});

async function logIn() {
  visit("/login", null);
  await userEvent.type(await screen.findByLabelText("Email"), "ana@example.com");
  await userEvent.type(screen.getByLabelText("Password"), "wrong");
  await userEvent.click(screen.getByRole("button", { name: "Log in" }));
  return (await screen.findByRole("alert")).textContent;
}

test("a wrong password shows the server's sentence", async () => {
  loginAnswers(401, "wrong email or password");
  expect(await logIn()).toBe("wrong email or password");
});

test("the login brake's 429 shows as it stands", async () => {
  loginAnswers(429, "too many attempts; try again in 5 minutes");
  expect(await logIn()).toBe("too many attempts; try again in 5 minutes");
});
