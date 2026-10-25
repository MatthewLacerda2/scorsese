# web

The React front-end of scorsese's hosted web app (#527). It is its own project,
the way `app/` is its own cargo workspace: it has its own toolchain and its own
gate, and nothing in the Rust workspace depends on it.

**Bun**, **Vite**, **React**, **TypeScript**, **Tailwind v4** and **shadcn/ui**.
The build is plain static files in `dist/`, which the deploy serves from nginx;
there is no JavaScript server in production.

## Running it

Bun, at the version `packageManager` in `package.json` names
(<https://bun.com/docs/installation>). From this directory:

    bun install              # once, and after bun.lock changes
    bun run dev              # http://localhost:5173, reloading on save

The dev server proxies `/api` to the Rust server, so a page calls
`fetch("/api/...")` and it works the same in development and behind the deploy.
It expects the server on `http://localhost:8080`; point it elsewhere with

    SCORSESE_API=http://localhost:9000 bun run dev

To sign in without a Google client (the usual dev stack, `docs/web.md`'s
*Accounts*), make an account and a session with the server's own commands and
set the cookie in the browser's console on `http://localhost:5173`:

    scorsese-server user create me@example.com
    scorsese-server user session me@example.com   # prints scorsese_session=…
    document.cookie = "scorsese_session=…; path=/api; secure"

With a client of your own, `SCORSESE_PUBLIC_ORIGIN=http://localhost:5173` and
that origin's `/api/auth/google/callback` as a redirect URI make the real
button work here too.

`bun run build` writes `dist/`, and `bun run preview` serves that build locally.

## How it is laid out

| where | what | reuse it for |
| --- | --- | --- |
| `src/api/` | **the only code that knows a URL**: `request` (JSON in and out, `ApiError` with the server's `{"error"}` and the fields beside it), the route functions (`api.library.list(…)`), the URLs the browser follows by itself (`api.events.url`, `api.uploads.endpoint`, `api.library.fileUrl`, `api.account.googleSignIn`) and the response types, named after the Rust types they mirror. `routes.test.ts` holds every route named here to `docs/web.md`'s route tables, which the server's router is held to (#994), so a call to a route that does not exist fails `bun test` (#1227) | every server call — add a route here, never a `fetch` or a `/api` path in a page (both fail the lint) |
| `src/app/queryClient.ts` | the TanStack Query cache; keys start with their area (`["library", …]`), and any `401` logs the page out | invalidating an area after a change |
| `src/app/queries.ts` | queries more than one page reads: projects, balance, library, one file | the editor's assets panel and header |
| `src/app/routes.tsx` | every page by URL, behind `RequireSession`: the library and spending inside `SidebarLayout`, the project's files page and the editor inside `Shell` (the header, which gives the editor the whole window) | adding a page |
| `src/app/pages.ts` | every page as a **lazy** import (#896), each its own chunk: a page is added here and named in `routes.tsx`, **never imported statically**, so the landing page never downloads the editor. `preload()` fetches one ahead (the home fetches the editor when idle) | adding a page |
| `src/build/budget.ts` | the entry chunk's budget, a Vite plugin: the build fails when what a first visit downloads outgrows it, and says why the number is what it is | — |
| `src/build/kit.ts` | the motion kit for the landing page, a Vite plugin (and, for `bun test`, `src/test/kit.ts`): joins the kit's pieces into the one script a video's page loads, in the same order | — |
| `src/app/events.ts` | `useServerEvents`: the one `EventSource` on `/api/events` a page holds, shared by every listener, with a `resync` after a reconnect | anything live — jobs, the assistant, a project another tab changed |
| `src/app/ErrorBoundary.tsx`, `src/app/crashes.ts` | a crash contained where it happened: one boundary around the app (#1426), one around each editor panel with `place="panel"` (#1428); what a boundary catches, and what the window sees thrown or rejected, is posted once to `POST /api/client-errors` for the server's log | a new independent piece of a page — give it its own boundary |
| `src/session/` | `useAccount`, `useLogout`, the `RequireSession` guard and the `?next=` rule | anything that needs to know who is logged in |
| `src/files/` | the Drive-like browser (grid, details panel, viewer), and uploads: an `UploadsProvider` around the signed-in app, so an upload survives navigation | showing or picking library files anywhere |
| `src/lib/upload/` | the browser-side hash, the Uppy + tus uploader, and the duplicate rule (a `409` with `item` is never retried) | — |
| `src/lib/money.ts` | micro-dollars as dollars, by integer arithmetic | every figure of money on a page |
| `src/i18n/` | the web app's languages (#704): one catalogue per language (`en/`, `pt-BR/`, `es/`, a file per area), the stored choice (`language.ts`), and `useT()` / `useLanguage()` | every user-visible string — see *Strings and languages* below |
| `src/lib/theme.ts` | light or dark: the stored choice, else light (never the system's); `index.html`'s inline script applies the same rule before the bundle loads, so a dark page never flashes white, and `theme.test.ts` runs that script to keep the two agreeing | the control (`src/app/ThemeControl.tsx`, in Settings and on the landing page's header); the mark in the page's theme is `src/components/Mark.tsx`; colours come from index.css's tokens, never a fixed `bg-white` |
| `public/` | files served at a fixed URL: the square icon (favicon, account button), resized from `app/assets/`; `robots.txt`, `sitemap.xml` and `.well-known/`; and `landing/og.png`, the link-preview still `index.html` names absolutely. Anything a page shows goes in `src/` and is imported instead, so the build fingerprints it and it is cached for good (#1238) | — |
| `landing/` | `hero.scor`, the film scorsese made of itself: pages, native text and a synthesised score, so re-rendering it costs nothing. Only its link-preview still is used now (`public/landing/og.png`) | changing the preview — edit the project, run `make landing-og`, commit what it writes |
| `src/landing/` | the landing page at `/` (#904): its sections, the sign-in popup, and `motion.ts`, which drives the motion kit's helpers (`crates/render/src/page/shipped/kit/`, joined as a page gets it by `src/build/kit.ts`, never copied) with the scroll as their clock, and `ScrollIn.tsx`, which plays a section's entrance as it is scrolled to. It follows the theme (light by default) like the app. Its type is the app's Geist; its films are in `media/showcase/`, cuts of videos made with scorsese (the strip under the headline, and the whole LinkedIn post in the visitor's language), imported so they are fingerprinted under `/assets/` (#1238) | — |
| `src/pages/` | the signed-in home (`HomePage`: the prompt box that starts a project), the two file views, the spending history | — |
| `src/home/` | the sidebar the home, the library and the spending history share (`SidebarLayout`, `Sidebar`, `ProjectItem` with inline rename and a confirmed delete), the rotating title, and `projectName` (a project's name from its prompt) and `useStartFromPrompt` (make the project, send the prompt, open it) | the sidebar's pages |
| `src/payments/` | buying credits (#548): the *Add credits* dialog (its own chunk, opened from the button beside the balance), the `/credits/return` page Stripe sends a buyer back to, and reais typed and shown as integer centavos (`reais.ts`) | anything in reais |
| `src/start/` | what a project is started for (#1016): the platform and style pickers, the menu query (`GET /api/styles`) and its plain decisions (`menu.ts`) — shared by the new-project modal and the editor's Platform and style control | choosing a platform or a style anywhere |
| `src/editor/` | the editor (`/projects/:id/edit`): `timeline/` (the time↔pixel maths, drag, snap and the tool call a drag becomes, all plain functions), `preview/`, `inspector/`, `assets/`, `selection.ts` (one clip, or several with Shift), `chat/` (the assistant's panel, its transcript a pure reducer over the event stream); every edit goes through `project.ts`'s `useEdit`, a tool call — docs/web.md, *The editor* | — |

**Uploads** hash a file in the browser first (streamed, so a large file is
not read into memory) and ask `GET /api/library?sha256=`; a duplicate never
crosses the network. The rest goes over tus in 50 MB chunks, under
Cloudflare's 100 MB request cap (docs/web.md, *Library*).

## Strings and languages

The web app speaks English, Brazilian Portuguese and Spanish (#704). The
choice is the browser's, not the account's: Settings (the gear in the header,
and a picker on the login page) stores it in the browser (`src/lib/storage.ts`),
and until then the browser's preferred languages decide, the first we speak
winning (`pt*` → pt-BR, `es*` → es, `en*` → en), with Brazilian Portuguese
when none is (#770). Text the **server** writes — error sentences, job messages, tool
replies — and anything the **assistant** writes stay as they arrive; the
assistant already answers in the language the user writes in.

**Adding a string.** Put it in the English catalogue for its area
(`src/i18n/en/<area>.ts`), then in `pt-BR/` and `es/` — `tsc` refuses a
catalogue that lacks a key English has, and `catalogue.test.ts` checks the
same at run time. Read it with `const t = useT()` and `t.<area>.<key>`. A word
written straight into JSX is refused by the `literalUiText` house rule (*House
rules*); text that is genuinely not the UI's language, like the legal pages'
Portuguese, says so in a `biome-ignore` with its reason. A message that
carries a value is a function (`deleted: (name: string) => …`), and a plural
is a function of the count, so each language words it its own way. A plain
function that returns words (a label, a warning) takes the messages as an
argument; its tests pass `en`. Numbers and dates go through `Intl` in
`useLanguage().language` (`lib/format.ts` takes it); **money stays dollars**,
formatted by `lib/money.ts` whatever the language — only the words around it
translate. Write the translation the way a native speaker would say it in an
app, not word for word.

**Adding a language.** Add it to `Language` and `LANGUAGES` in
`src/i18n/language.ts` (named in itself), teach `browserLanguage` its tag, copy
`src/i18n/en/` to a new folder typed as `Messages` (as `pt-BR/` is), translate,
and list it in `CATALOGUES` (`src/i18n/catalogue.ts`).

**Why not react-i18next.** A catalogue here is a typed TypeScript object, so a
missing key, a misspelt one and a wrong argument are compile errors, `t.` is
autocompleted, and there is no runtime dependency, loader or string-keyed
lookup. i18next's draws — loading catalogues over the network, ICU message
syntax, dozens of languages — are not needs at three bundled languages; if
they become needs, the catalogues port across as they are.

## The gate

`make web` from the repo root runs what CI's `web` job runs, in order:

| step | command | what it checks |
| --- | --- | --- |
| install | `bun install --frozen-lockfile` | `bun.lock` matches `package.json` — the packages checked are the ones committed |
| lint | `bun run lint` (`biome ci`) | lint rules, the house rules below, **and** formatting; `bun run format` fixes what it can. `noShadow` is an error: an inner name never hides an outer one (a local `t` over `useT()`'s, #1209) |
| typecheck | `bun run typecheck` (`tsc --noEmit`) | TypeScript in strict mode, the end-to-end flows included |
| dead code | `bun run deadcode` ([knip](https://knip.dev), `knip.jsonc`) | what is dead *between* files, which `noUnusedLocals` cannot see: a file nothing imports, an export nothing outside its file uses, a `package.json` dependency nothing needs — see *Dead code* below |
| test | `bun test` | every `*.test.ts(x)` under `src/`, in a DOM (`src/test/dom.ts`) |
| build | `bun run build` | the bundle the deploy ships actually builds, and its entry chunk is within budget (`src/build/budget.ts`) |

`make web-e2e` runs the end-to-end flows (`tests/e2e/`), which CI runs at the
end of its `fmt + clippy + test` job, on the server binary that job's tests
already built:

| step | command | what it checks |
| --- | --- | --- |
| server | `cargo build -p scorsese-server` | the binary the flows drive |
| browser | `bunx playwright install --only-shell chromium` | Playwright's pinned headless Chromium, fetched once; skipped when `E2E_CHROMIUM` names one |
| flows | `bunx playwright test`, through `tools/with-postgres` | sign in, make a project, upload and place a file, resize a panel, ask the assistant — against the real server and a real Postgres. The flows that run an assistant turn (a message, a new platform, a confirmed quote) run in every language, since a translated piece can crash in one language only (#1429); the rest stay English |

`playwright.config.ts` starts the server (away from any `.env`, so no provider
key reaches it) and `vite preview` of the production build, and makes the run's
account with `scorsese-server user create` and signs it in with the cookie
`user session` prints, as the operator would — no flow reaches Google (#952);
the server runs without a Google client, so the sign-in button's flow is
checked as far as its "not set up" answer. A failure
keeps its trace and screenshot in `test-results/` — CI uploads them as
`web-e2e-failures` — and `bunx playwright show-trace <trace.zip>` replays one
step by step.

`make gates` runs `web` only when the branch touches `web/`, and `web-e2e` when
it touches `web/` or any crate but `crates/golden` — the server is a thin
client of the others, so any of them can break a flow — and
prints which it did not run otherwise, the same rule as the `app` gate. The
size gate (`make size`) holds `.ts`/`.tsx` to the same caps as Rust: 300 lines
of code for source, 150 for a `*.test.ts(x)` file or anything under `tests/`;
`tools/lint/src/classify.rs` says why.

## Dead code

`bun run deadcode` fails on a file nothing reaches from an entry point, an
export no other file imports, and a dependency no file needs (#1208). The
entry points are the app (`index.html` → `src/main.tsx`), the two configs,
every `bun test` file and its preloads, and the end-to-end flows; knip's Vite,
Playwright and Bun plugins find most of them and `knip.jsonc` names the rest.

- **An export only its own file uses** loses its `export`; one that is used
  nowhere is deleted. It comes back the moment a second file needs it.
- **`landing/` and `lint/fixtures/` are not checked**: the first is the hero
  film's scorsese project, whose pages scorsese-render loads and the build
  never imports; the second is input Biome reads in the house-rule tests.
- **Exemptions live in `knip.jsonc`, each with its reason beside it.**
  `src/components/ui/` keeps its unused *exports*, since shadcn ships whole
  families (`DialogClose`, `TableFooter`…) and is re-added with `shadcn add`;
  an unused *file* there is still reported and deleted. A redundant entry or
  exemption is a config hint, and the gate treats a hint as an error, so the
  config cannot keep a line that no longer does anything.

## House rules

Conventions of this codebase that Biome's built-in set does not have, each
enforced rather than written down (CLAUDE.md, *A convention a machine can
check is a gate*). They run inside `biome ci`, so `bun run lint`, `make web`
and CI's `web` job report them like any other lint.

| rule | what it holds |
| --- | --- |
| `oneComponentPerFile` | a `.tsx` file exports **at most one React component** (#1207) — a PascalCase function, class or const, a default export, or a name in an `export { … }` list. Helpers only the file uses stay in it, unexported; anything else gets its own file, and a family of them a subfolder. An agent looking for a component then finds it by its file's name. `src/components/ui/` is exempt: shadcn ships compound modules (`Dialog`, `DialogHeader`…) on purpose |
| `noRawStorage` | browser storage goes through **`src/lib/storage.ts`** (#1212), never `localStorage` or `sessionStorage` directly — bare, off `window` or `globalThis`, or destructured. The module's `KEYS` names every key the app keeps, so what a user's browser holds is one list; a caller pairs its key with a codec (its fallback, how it is read and written) and calls `read` / `write`, which catch the storage a private window throws. A key's name, once shipped, never changes: renaming it forgets every user's saved choice. `src/test/` is exempt: the test setup clears storage between tests |
| `literalUiText` | words a user reads come from the catalogue (#1211, *Strings and languages*): no text between JSX tags, no string or template literal in a JSX child — `?:`, `??`, `\|\|` and `&&` branches included — and none in `alt`, `title`, `placeholder`, `label` or the text `aria-*` attributes. Symbols, numbers, single letters, acronyms and `scorsese` alone pass; calls (`t.x(…)`, `cn("…")`) are not looked into. **Its hole:** a literal outside JSX (`name \|\| "Untitled"` in a hook) is not seen, so review still reads for one. Tests and `src/test/` are exempt, since their JSX is a harness no user sees; `src/components/ui/` is not, since a shadcn component's screen-reader text is read by users too |
| `colorsFromTokens` | a colour comes from a **theme token**, never a literal (#1210): no hex, `rgb(` / `hsl(` / `oklch(`… or Tailwind palette class (`bg-sky-500`, `text-zinc-400`, `bg-black`) in a `.ts` / `.tsx` string, template or JSX attribute — `className`, `cn(…)`, `style` alike. Tokens live in `src/index.css` (`--background`, `--kind-video`, `--screen`, `--color-paper`…) and are used as classes (`bg-kind-video`, `text-on-screen`, `border-paper/10`); a new colour is a token there first, with its dark value where it differs. Stripping Tailwind's default palette from the theme instead was weighed and not done: it would break every shadcn component re-added by `shadcn add` and say nothing about a hex. The rule reads quoted text only, so a colour built at run time, a CSS named colour inside a value and four-digit hex (`#1210` is a ticket) are not seen. `src/components/ui/`, `src/index.css` and tests are out of scope; a literal that must stay (a flag's colours in SVG) takes `biome-ignore-all lint/plugin/colorsFromTokens: <why>` |
| `serverUrlOutsideApi` | only `src/api/` names a server path (#1227): a string, template or JSX literal starting `/api`, or an absolute URL whose path does, is refused anywhere else in `src/`. Tests are exempt, since they name the URL they expect or fake |
| `shadcnControls` | a control shadcn/ui has a component for is built from it (#1213): `<button>` is `Button`, a text-like `<input>` is `Input`, `<textarea>` is `Textarea`, `<select>` is `NativeSelect` (a styled native `<select>`, so roles and keyboard behaviour stay the browser's). Inputs of type `checkbox`, `range`, `file`, `color` and `hidden` stay native: nothing in `components/ui` replaces them. A real custom surface keeps the native element with a `biome-ignore lint/plugin/shadcnControls: <why>` (today: the timeline clip, the draggable asset and file tiles, click-to-edit text); a primitive not yet in `components/ui` is added with `bunx --bun shadcn@latest add <name>`, never hand-written. `src/components/ui/` and tests are exempt. Other rules' fixtures write a control as a component (or a plain element), so only the rule under test fires |

Two of the same conventions are Biome built-ins instead, configured in
`biome.jsonc`: `noRestrictedGlobals` keeps `fetch` and `EventSource` to
`src/api/` and `src/app/events.ts` (the one stream a page holds), and
`noRestrictedImports` keeps a page (`@/pages/…`) out of every file but
`src/app/pages.ts`, which imports each lazily (#896). Their exemptions are
`overrides` there, each with its reason.

**How one is made.** A rule is a [GritQL plugin](https://biomejs.dev/linter/plugins/)
in `lint/<rule>.grit` whose message starts with `<rule>:`, listed under
`plugins` in `biome.jsonc` (with any folder it does not apply to, and why,
beside it) and in `lint/fixtures/rules.jsonc`. Its fixtures are
`lint/fixtures/<rule>/`: a `flags*.tsx` whose every line ending in
`// flagged` must be reported, and no other line, and a `passes*.tsx` that
must not be. `src/lint/houseRules.test.ts` runs Biome over them and fails on
a rule missing from either config or without both kinds of fixture, so a new
rule is a `.grit` file, two config lines and its fixtures. The real lint
leaves `lint/fixtures/` out, since its `flags` files fail on purpose.

**Every rule runs over every fixture**, since `rules.jsonc` loads them all,
and the test fails on a diagnostic from any rule but the one under test. So a
fixture is written in the house style for everything except the rule it is
about: no literal words in JSX (`{label}` from a `declare const`), no colour,
no server path, components for controls. A new rule's first job is to run
over the fixtures already there, and fix them, not itself, when it is right
(2026-10-16: `literalUiText` flagged `serverUrlOutsideApi`'s link text, and
`shadcnControls` two other rules' `<button>`s).

**Two branches adding a rule collide** on `biome.jsonc`'s `plugins`,
`rules.jsonc` and the table above: keep both sides, and check the
`includes` arrays still close, since a hand-merge of two entries can drop a
`]` (it did, once, and Biome refused the whole config).

**An exception is inline, with its reason.** Biome refuses a suppression
without one. A file-wide rule like `oneComponentPerFile` takes it once, at the
top of the file:

    // biome-ignore-all lint/plugin/oneComponentPerFile: <why this file is the exception>

A line-level rule takes `// biome-ignore lint/plugin/<rule>: <why>` above the
line. A folder that a rule does not fit at all goes in that rule's `includes`
in `biome.jsonc`, with the reason in a comment beside it — never a list of
excused files.

## Which test, where

Three kinds, cheapest first; a new test goes in the first one that can see
what it is about.

- **A plain function** (`*.test.ts` beside it): the logic — sizes, snapping,
  the tool call a drag becomes, money. Most tests are this, and anything a
  component decides belongs in a function this can reach.
- **A component under a pointer** (`*.test.tsx`, `@testing-library/react` and
  `user-event`, in happy-dom): where a mouse or a key is the subject — a handle
  dragged, a double-click, a clip let go as one `clip_move`. happy-dom lays
  nothing out, so a test gives an element the box it needs
  (`timeline/pointer.test.tsx`) rather than trusting zeros. A page rendered to
  HTML (`App.test.tsx`, `prerender`) still does for what it shows.
- **A flow** (`tests/e2e/*.spec.ts`, Playwright): a few things a person does
  from end to end, where the page and the server must agree — not every
  control. The assistant is faked in the browser with `page.route`; nothing
  here may call a provider.

## Choices, and why

- **knip for dead code.** It reads the Vite, Playwright and Bun configs for
  entry points, so it knows what runs without a hand-kept list, and it checks
  files, exports and `package.json` in one pass. `tsc`'s `noUnusedLocals`
  stays: it sees inside a file, which knip does not.
- **Biome, not ESLint + Prettier.** One tool, one config, one dev dependency,
  and it is both the linter and the formatter — the `cargo fmt --check` +
  `clippy` pair in one command. ESLint's plugin ecosystem is larger, but its
  main draw for React (the hooks rules) is covered by Biome's recommended set.
- **House rules as Biome plugins, not a checker of our own.** A GritQL plugin
  runs inside `biome ci`, which the gate already runs, so a rule costs no new
  tool, no new step and no second parser. It reports a diagnostic, takes
  Biome's own inline suppressions with their mandatory reason, and is scoped
  to folders by its `includes` (checked against Biome 2.5.15 when the first
  rule was made, #1207). A checker of our own would have needed a TypeScript
  parser: the installed TypeScript 7 exports only `typescript/unstable/*`, not
  a compiler API to build on. The config is `biome.jsonc` so that an
  exemption's reason can sit beside it.
- **`bun test`, not Vitest.** Bun is already the runtime, and its runner reads
  the same `tsconfig.json` paths, so there is nothing to configure. Logic is
  tested as plain functions, and pages render through `react-dom/static`'s
  `prerender` (which waits for a lazy page's module) with a seeded query cache. Since #898 it also has a DOM — happy-dom, preloaded by
  `bunfig.toml` — for `@testing-library/react` and `user-event`, the standard
  pair for a component that has to be clicked, typed into or dragged.
- **Playwright for the flows, not Cypress.** Faster headless, several tabs in
  one test, and a trace that is readable from a CI artifact. Chromium only,
  through Playwright's own runner on Node (installed by Bun), against the real
  server rather than a faked API, so the two disagreeing is caught.
- **TanStack Query for server state, React Router for pages.** Every page is
  a view of rows the server owns, and the questions — cached, refetched on
  focus, invalidated after a change, a `401` anywhere meaning logged out — are
  exactly what the query cache answers once, rather than each page again.
- **Uppy + tus for uploads**, because the tunnel refuses a body over 100 MB
  and the server speaks tus (#535). Only `@uppy/core` and `@uppy/tus`: the
  progress list is ours, so it is styled like the rest.
- **shadcn/ui components live in `src/components/ui/`**, added with
  `bunx --bun shadcn@latest add <name>`, and are used wherever one exists: the
  `shadcnControls` house rule refuses a native button, text input, textarea or
  select elsewhere. They are copied in to be edited, so
  they are our code: linted, formatted and size-gated like the rest.
