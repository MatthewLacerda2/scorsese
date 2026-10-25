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
import { type Account, type Balance, type EditorProject, SCHEMA_VERSION } from "@/api";
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

test("/login draws the landing page, its films included", async () => {
  // The popup itself is a portal, drawn only in a browser: landing/LandingPage.test.tsx.
  const html = await render("/login", (client) => client.setQueryData(ME, null));
  expect(html).toContain("The video is one conversation away");
  // Imported, so the build fingerprints it under /assets (#1238).
  expect(html).toMatch(/<video[^>]*src="[^"]*showcase\/[^"]*\.mp4"/);
});

test("the theme control offers Light and Dark, and Light is the default", async () => {
  const html = await prerenderHtml(<ThemeControl />);
  // Each segment's pressed state, paired with its label (the text that ends the button).
  const pressed = [...html.matchAll(/aria-pressed="(\w+)".*?(\w+)<\/button>/g)].map(
    ([, on, label]) => `${label}:${on}`,
  );
  expect(pressed).toEqual(["Light:true", "Dark:false"]);
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

test("signed in, / is the home, listing the user's projects in the sidebar", async () => {
  const html = await render("/", (client) => {
    client.setQueryData(ME, ana);
    client.setQueryData(
      ["projects"],
      [
        {
          id: 4,
          name: "teaser",
          revision: 2,
          created_at: 0,
          updated_at: 0,
          platform: null,
          style: null,
        },
      ],
    );
  });
  expect(html).toContain("teaser");
  expect(html).toContain('href="/projects/4/edit"');
  expect(html).toContain('aria-label="Describe your video"');
});

test("the editor draws the stored document: its tracks, clips and assets", async () => {
  const project: EditorProject = {
    id: 4,
    name: "teaser",
    revision: 9,
    script_changed_at: null,
    created_at: 0,
    updated_at: 0,
    platform: null,
    style: null,
    document: {
      schema_version: SCHEMA_VERSION,
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
  // The sidebar (#702): the project's assets, and a button to
  // the library rather than the library itself.
  expect(html).toContain("Drag onto a track to place it"); // the assets, headed by nothing (#1446)
  expect(html).toContain("Library</button>");
  expect(html).not.toContain("Your library");
});

test("the header has room for a page's own controls, empty unless the page fills it", async () => {
  const html = await render("/projects/9", (client) => {
    client.setQueryData(ME, ana);
  });
  const header = html.slice(html.indexOf("<header"), html.indexOf("</header>"));
  expect(header).toMatch(/<div data-header-slot=""[^>]*><\/div>/);
  expect(header).not.toContain("Render");
});
