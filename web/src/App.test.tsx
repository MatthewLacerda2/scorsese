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
      language="en"
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

test("the theme control offers Light, Dark and System, and System is the default", () => {
  const html = render("/login", (client) => client.setQueryData(ME, null));
  // Each segment's pressed state, paired with its label (the text that ends the button).
  const pressed = [...html.matchAll(/aria-pressed="(\w+)".*?(\w+)<\/button>/g)].map(
    ([, on, label]) => `${label}:${on}`,
  );
  expect(pressed).toEqual(["Light:false", "Dark:false", "System:true"]);
});

test("signed in, the library sits in the shell with the balance in dollars", () => {
  const balance: Balance = { balance_micros: 2_000_000 };
  const html = render("/library", (client) => {
    client.setQueryData(ME, ana);
    client.setQueryData(["credits", "balance"], balance);
    client.setQueryData(["library", "list", {}], []);
  }).replaceAll(" ", " ");
  expect(html).toContain("Library");
  expect(html).toContain("$2.00");
  expect(html).not.toContain("R$");
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
  // Open, into the editor, comes before Files.
  const open = html.indexOf('href="/projects/4/edit"');
  expect(open).toBeGreaterThan(-1);
  expect(html.slice(open, html.indexOf("</a>", open))).toContain("Open");
  expect(open).toBeLessThan(html.indexOf('href="/projects/4"'));
});

test("the editor draws the stored document: its tracks, clips and assets", () => {
  const project: EditorProject = {
    id: 4,
    name: "teaser",
    revision: 9,
    created_at: 0,
    updated_at: 0,
    document: {
      schema_version: 44,
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
  // The revision guards saves (docs/web.md) but is never shown (#764), and
  // neither is the preview's quality line; the editor's controls are portalled
  // into the header, which a server render has no element for.
  expect(html).not.toMatch(/revision/i);
  expect(html).not.toContain("Preview at");
  expect(html).not.toContain("Back to projects");
  expect(html).toContain("HELLO");
  expect(html).toContain("Video track");
  expect(html).toContain("Ask the assistant");
  // The sidebar (#702): the project's assets, the templates, and a button to
  // the library rather than the library itself.
  expect(html).toContain("Assets");
  expect(html).toContain("Templates");
  expect(html).toContain("Library</button>");
  expect(html).not.toContain("Your library");
});

test("the header has room for a page's own controls, empty unless the page fills it", () => {
  const html = render("/projects", (client) => {
    client.setQueryData(ME, ana);
  });
  const header = html.slice(html.indexOf("<header"), html.indexOf("</header>"));
  expect(header).toMatch(/<div data-header-slot=""[^>]*><\/div>/);
  expect(header).not.toContain("Render");
});
