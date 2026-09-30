// Pages render at their URLs. Through `react-dom/server`, so no DOM: a query
// with no seeded data stays pending and fetches nothing, and an effect (the
// uploader) never runs — what is checked is what each page draws first.

import { expect, test } from "bun:test";
import { renderToString } from "react-dom/server";
import { MemoryRouter } from "react-router";
import { App } from "@/App";
import type { Account, Balance, EditorProject } from "@/api";
import { createQueryClient, ME } from "@/app/queryClient";

function render(url: string, seed: (client: ReturnType<typeof createQueryClient>) => void) {
  const client = createQueryClient();
  seed(client);
  return renderToString(
    <App
      queryClient={client}
      router={(routes) => <MemoryRouter initialEntries={[url]}>{routes}</MemoryRouter>}
    />,
  );
}

const ana: Account = { id: 1, email: "ana@example.com", created_at: 0 };

test("the login page asks for an email and a password", () => {
  const html = render("/login", (client) => client.setQueryData(ME, null));
  expect(html).toContain('type="email"');
  expect(html).toContain('type="password"');
  expect(html).toContain("Log in");
});

test("signed in, the library sits in the shell with the balance in ≈ reais", () => {
  const balance: Balance = {
    balance_micros: 2_000_000,
    balance_centavos: 1_086,
    rate: { brl_per_usd_e4: 54_321, set_at: 0 },
  };
  const html = render("/library", (client) => {
    client.setQueryData(ME, ana);
    client.setQueryData(["credits", "balance"], balance);
    client.setQueryData(["library", "list", {}], []);
  }).replaceAll(" ", " ");
  expect(html).toContain("Library");
  expect(html).toContain("R$ 10,86");
  expect(html).toContain("Your library is empty");
  expect(html).toContain("Upload");
});

test("the projects page lists what the user has", () => {
  const html = render("/projects", (client) => {
    client.setQueryData(ME, ana);
    client.setQueryData(
      ["projects"],
      [{ id: 4, name: "teaser", revision: 2, created_at: 0, updated_at: 0 }],
    );
  });
  expect(html).toContain("teaser");
  expect(html).toContain('href="/projects/4/edit"');
});

test("the editor draws the stored document: its tracks, clips and assets", () => {
  const project: EditorProject = {
    id: 4,
    name: "teaser",
    revision: 9,
    created_at: 0,
    updated_at: 0,
    document: {
      schema_version: 38,
      name: "teaser",
      timeline_fps: { num: 30, den: 1 },
      assets: [{ id: "title", kind: "text", text: "HELLO" }],
      tracks: [
        {
          id: "v1",
          kind: "video",
          clips: [{ id: "c1", asset: "title", start: 30, duration: 60 }],
        },
      ],
    },
  };
  const html = render("/projects/4/edit", (client) => {
    client.setQueryData(ME, ana);
    client.setQueryData(["projects", "editor", 4], project);
  });
  expect(html).toContain("revision 9");
  expect(html).toContain("HELLO");
  expect(html).toContain("Video track");
  expect(html).toContain("Ask the assistant");
});
