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
| `src/lib/money.ts` | micro-dollars and centavos as text, by integer arithmetic | every figure of money on a page |
| `src/pages/` | login, projects, the two file views, the spending history | — |
| `src/editor/` | the editor (`/projects/:id/edit`): `timeline/` (the time↔pixel maths, drag, snap and the tool call a drag becomes, all plain functions), `preview/`, `inspector/`, `assets/`, `templates/` (save the selection, insert at the playhead — #546), `selection.ts` (one clip, or several with Shift), `chat/` (the assistant's panel, its transcript a pure reducer over the event stream); every edit goes through `project.ts`'s `useEdit`, a tool call — docs/web.md, *The editor* | — |

**Uploads** hash a file in the browser first (streamed, so a large file is
not read into memory) and ask `GET /api/library?sha256=`; a duplicate never
crosses the network. The rest goes over tus in 50 MB chunks, under
Cloudflare's 100 MB request cap (docs/web.md, *Library*).

## The gate

`make web` from the repo root runs what CI's `web` job runs, in order:

| step | command | what it checks |
| --- | --- | --- |
| install | `bun install --frozen-lockfile` | `bun.lock` matches `package.json` — the packages checked are the ones committed |
| lint | `bun run lint` (`biome ci`) | lint rules **and** formatting; `bun run format` fixes what it can |
| typecheck | `bun run typecheck` (`tsc --noEmit`) | TypeScript in strict mode |
| test | `bun test` | every `*.test.ts(x)` |
| build | `bun run build` | the bundle the deploy ships actually builds |

`make gates` runs it only when the branch touches `web/`, and prints that it
did not run otherwise — the same rule as the `app` gate. The size gate
(`make size`) holds `.ts`/`.tsx` to the same caps as Rust: 300 lines of code for
source, 150 for a `*.test.ts(x)` file; `tools/lint/src/classify.rs` says why.

## Choices, and why

- **Biome, not ESLint + Prettier.** One tool, one config, one dev dependency,
  and it is both the linter and the formatter — the `cargo fmt --check` +
  `clippy` pair in one command. ESLint's plugin ecosystem is larger, but its
  main draw for React (the hooks rules) is covered by Biome's recommended set.
- **`bun test`, not Vitest.** Bun is already the runtime, and its runner reads
  the same `tsconfig.json` paths, so there is nothing to configure. Logic is
  tested as plain functions (the API client against a fake `fetch`, money,
  the duplicate rule), and pages render through `react-dom/server` with a
  seeded query cache, so no DOM is needed yet; the first test that has to
  click things adds one (happy-dom) then, not before.
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
