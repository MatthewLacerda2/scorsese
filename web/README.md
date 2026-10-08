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

`bun run build` writes `dist/`, and `bun run preview` serves that build locally.

## How it is laid out

| where | what | reuse it for |
| --- | --- | --- |
| `src/api/` | **the only code that knows a URL**: `request` (JSON in and out, `ApiError` with the server's `{"error"}` and the fields beside it), the route functions (`api.library.list(…)`) and the response types, named after the Rust types they mirror | every server call — add a route here, never a `fetch` in a page |
| `src/app/queryClient.ts` | the TanStack Query cache; keys start with their area (`["library", …]`), and any `401` logs the page out | invalidating an area after a change |
| `src/app/queries.ts` | queries more than one page reads: projects, balance, library, one file | the editor's assets panel and header |
| `src/app/routes.tsx` | every page by URL, behind `RequireSession` and inside `Shell` (which gives the editor the whole window) | adding a page |
| `src/app/events.ts` | `useServerEvents`: the one `EventSource` on `/api/events` a page holds, shared by every listener, with a `resync` after a reconnect | anything live — jobs, the assistant, a project another tab changed |
| `src/session/` | `useAccount`, `useLogin`, `useLogout`, the `RequireSession` guard and the `?next=` rule | anything that needs to know who is logged in |
| `src/files/` | the Drive-like browser (grid, details panel, viewer), and uploads: an `UploadsProvider` around the signed-in app, so an upload survives navigation | showing or picking library files anywhere |
| `src/lib/upload/` | the browser-side hash, the Uppy + tus uploader, and the duplicate rule (a `409` with `item` is never retried) | — |
| `src/lib/money.ts` | micro-dollars as dollars, by integer arithmetic | every figure of money on a page |
| `src/i18n/` | the web app's languages (#704): one catalogue per language (`en/`, `pt-BR/`, `es/`, a file per area), the stored choice (`language.ts`), and `useT()` / `useLanguage()` | every user-visible string — see *Strings and languages* below |
| `src/lib/theme.ts` | light or dark: the stored choice, else the system's; `index.html`'s inline script applies the same rule before the bundle loads, so a dark page never flashes white, and `theme.test.ts` runs that script to keep the two agreeing | the control (`src/app/ThemeControl.tsx`, in Settings and on the login page); colours come from index.css's tokens, never a fixed `bg-white` |
| `public/` | the logo (login page) and the square icon (favicon, account button), resized from `app/assets/` | — |
| `src/pages/` | login, projects, the two file views, the spending history | — |
| `src/editor/` | the editor (`/projects/:id/edit`): `timeline/` (the time↔pixel maths, drag, snap and the tool call a drag becomes, all plain functions), `preview/`, `inspector/`, `assets/`, `templates/` (save the selection, insert at the playhead — #546), `selection.ts` (one clip, or several with Shift), `chat/` (the assistant's panel, its transcript a pure reducer over the event stream); every edit goes through `project.ts`'s `useEdit`, a tool call — docs/web.md, *The editor* | — |

**Uploads** hash a file in the browser first (streamed, so a large file is
not read into memory) and ask `GET /api/library?sha256=`; a duplicate never
crosses the network. The rest goes over tus in 50 MB chunks, under
Cloudflare's 100 MB request cap (docs/web.md, *Library*).

## Strings and languages

The web app speaks English, Brazilian Portuguese and Spanish (#704). The
choice is the browser's, not the account's: Settings (the gear in the header,
and a picker on the login page) stores it in `localStorage`, and until then
`navigator.language` decides (`pt*` → pt-BR, `es*` → es, anything else
English). Text the **server** writes — error sentences, job messages, tool
replies — and anything the **assistant** writes stay as they arrive; the
assistant already answers in the language the user writes in.

**Adding a string.** Put it in the English catalogue for its area
(`src/i18n/en/<area>.ts`), then in `pt-BR/` and `es/` — `tsc` refuses a
catalogue that lacks a key English has, and `catalogue.test.ts` checks the same
at run time. Read it with `const t = useT()` and `t.<area>.<key>`. A message
that carries a value is a function (`deleted: (name: string) => …`), and a
plural is a function of the count, so each language words it its own way. A
plain function that returns words (a label, a warning) takes the messages as
an argument; its tests pass `en`. Numbers and dates go through `Intl` in
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
| lint | `bun run lint` (`biome ci`) | lint rules **and** formatting; `bun run format` fixes what it can |
| typecheck | `bun run typecheck` (`tsc --noEmit`) | TypeScript in strict mode, the end-to-end flows included |
| test | `bun test` | every `*.test.ts(x)` under `src/`, in a DOM (`src/test/dom.ts`) |
| build | `bun run build` | the bundle the deploy ships actually builds |

`make web-e2e` runs the end-to-end flows (`tests/e2e/`), which CI runs at the
end of its `fmt + clippy + test` job, on the server binary that job's tests
already built:

| step | command | what it checks |
| --- | --- | --- |
| server | `cargo build -p scorsese-server` | the binary the flows drive |
| browser | `bunx playwright install --only-shell chromium` | Playwright's pinned headless Chromium, fetched once; skipped when `E2E_CHROMIUM` names one |
| flows | `bunx playwright test`, through `tools/with-postgres` | log in, make a project, upload and place a file, resize a panel, ask the assistant — against the real server and a real Postgres |

`playwright.config.ts` starts the server (away from any `.env`, so no provider
key reaches it) and `vite preview` of the production build, and makes the run's
account with `scorsese-server user create`, as the operator does. A failure
keeps its trace and screenshot in `test-results/` — CI uploads them as
`web-e2e-failures` — and `bunx playwright show-trace <trace.zip>` replays one
step by step.

`make gates` runs `web` only when the branch touches `web/`, and `web-e2e` when
it touches `web/` or `crates/server/` — either side can break a flow — and
prints which it did not run otherwise, the same rule as the `app` gate. The
size gate (`make size`) holds `.ts`/`.tsx` to the same caps as Rust: 300 lines
of code for source, 150 for a `*.test.ts(x)` file or anything under `tests/`;
`tools/lint/src/classify.rs` says why.

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
  HTML through `react-dom/server` still does for what it shows.
- **A flow** (`tests/e2e/*.spec.ts`, Playwright): a few things a person does
  from end to end, where the page and the server must agree — not every
  control. The assistant is faked in the browser with `page.route`; nothing
  here may call a provider.

## Choices, and why

- **Biome, not ESLint + Prettier.** One tool, one config, one dev dependency,
  and it is both the linter and the formatter — the `cargo fmt --check` +
  `clippy` pair in one command. ESLint's plugin ecosystem is larger, but its
  main draw for React (the hooks rules) is covered by Biome's recommended set.
- **`bun test`, not Vitest.** Bun is already the runtime, and its runner reads
  the same `tsconfig.json` paths, so there is nothing to configure. Logic is
  tested as plain functions, and pages render through `react-dom/server` with
  a seeded query cache. Since #898 it also has a DOM — happy-dom, preloaded by
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
  `bunx --bun shadcn@latest add <name>`. They are copied in to be edited, so
  they are our code: linted, formatted and size-gated like the rest.
