// Pages render at their URLs. Through `react-dom/static`'s `prerender`, so no
// DOM: a query with no seeded data stays pending and fetches nothing, and an
// effect (the uploader) never runs — what is checked is what each page draws
// first. `prerender` waits for every Suspense boundary, so a lazy page (#896)
// is rendered once its module has loaded, never as its fallback; and
// `prerenderHtml` cleans up after it, so a DOM test of a router can follow in
// the same process (#963).

import { expect, test } from "bun:test";
import { MemoryRouter } from "react-router";
import { App } from "@/App";
import type { Account, Balance, EditorProject } from "@/api";
import { createQueryClient, ME } from "@/app/queryClient";
import { ThemeControl } from "@/app/ThemeControl";
import { prerenderHtml } from "@/test/prerender";

async function render(url: string, seed: (client: ReturnType<typeof createQueryClient>) => void) {
  const client = createQueryClient();
  seed(client);
  return prerenderHtml(
    <App
      language="en"
      queryClient={client}
      router={(routes) => <MemoryRouter initialEntries={[url]}>{routes}</MemoryRouter>}
    />,
  );
}

const ana: Account = { id: 1, email: "ana@example.com", created_at: 0 };

test("/login draws the landing page, its film included", async () => {
  // The popup itself is a portal, drawn only in a browser: landing/LandingPage.test.tsx.
  const html = await render("/login", (client) => client.setQueryData(ME, null));
  expect(html).toContain("It gets made.");
  expect(html).toContain('src="/landing/hero.mp4"');
});

test("the theme control offers Light, Dark and System, and System is the default", async () => {
  const html = await prerenderHtml(<ThemeControl />);
  // Each segment's pressed state, paired with its label (the text that ends the button).
  const pressed = [...html.matchAll(/aria-pressed="(\w+)".*?(\w+)<\/button>/g)].map(
    ([, on, label]) => `${label}:${on}`,
  );
  expect(pressed).toEqual(["Light:false", "Dark:false", "System:true"]);
});

test("signed in, the library sits in the shell with the balance in dollars", async () => {
  const balance: Balance = { balance_micros: 2_000_000 };
  const html = (
    await render("/library", (client) => {
      client.setQueryData(ME, ana);
      client.setQueryData(["credits", "balance"], balance);
      client.setQueryData(["library", "list", {}], []);
    })
  ).replaceAll(" ", " ");
  expect(html).toContain("Library");
  expect(html).toContain("$2.00");
  expect(html).not.toContain("R$");
  expect(html).toContain("Your library is empty");
  expect(html).toContain("Upload");
});

test("the projects page lists what the user has", async () => {
  const html = await render("/projects", (client) => {
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

test("the editor draws the stored document: its tracks, clips and assets", async () => {
  const project: EditorProject = {
    id: 4,
    name: "teaser",
    revision: 9,
    created_at: 0,
    updated_at: 0,
    document: {
      schema_version: 47,
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
  const html = await render("/projects/4/edit", (client) => {
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
  expect(html).toContain("c1 — drag to move"); // the timeline, with its clip
  expect(html).toContain("Ask the assistant");
  // The sidebar (#702): the project's assets, the templates, and a button to
  // the library rather than the library itself.
  expect(html).toContain("Assets");
  expect(html).toContain("Templates");
  expect(html).toContain("Library</button>");
  expect(html).not.toContain("Your library");
});

test("the header has room for a page's own controls, empty unless the page fills it", async () => {
  const html = await render("/projects", (client) => {
    client.setQueryData(ME, ana);
  });
  const header = html.slice(html.indexOf("<header"), html.indexOf("</header>"));
  expect(header).toMatch(/<div data-header-slot=""[^>]*><\/div>/);
  expect(header).not.toContain("Render");
});
