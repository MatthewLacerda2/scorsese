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
  the same `tsconfig.json` paths, so there is nothing to configure. The smoke
  test renders through `react-dom/server` and needs no DOM; the first page that
  needs to click things adds one (happy-dom) then, not before.
- **shadcn/ui components live in `src/components/ui/`**, added with
  `bunx --bun shadcn@latest add <name>`. They are copied in to be edited, so
  they are our code: linted, formatted and size-gated like the rest.
