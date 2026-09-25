# The web app

Scorsese as a hosted service: a URL, a login, a library of a person's own
files, and an assistant that edits for them. It is **the main product** — how
everyone other than the maintainer uses scorsese (`CLAUDE.md`, *The web app is
the main product*). The desktop app and the CLI stay, on local `.scor`
folders.

This page is the doctrine for the web side: the decisions settled with the
maintainer on 2026-09-24/25 in #527, written down so no sub-issue reopens them.
Each sub-issue of #527 adds its own section here as it merges; what is below is
what was decided before any of them started.

## Who it is for

A handful of people: friends and family first, paying customers after — a
law-firm promo, daily same-format selling videos, a paid trailer, the
maintainer's own channel. That scale is load-bearing. Several decisions below
(no message broker, no vector search, a flat library) are right *because* the
user count is small, and each says what would make it worth revisiting.

## The shape

**The API is Rust, in this repo** — `crates/server`, package `scorsese-server`.
It is one more thin client of `core`, `render` and `providers`, a sibling of
`cli` and `mcp`, with **no editing logic of its own**. An endpoint that needs
code the CLI does not share is a sign that code belongs a layer down. No second
server language.

**The front-end is React, in `web/`** — Bun, Tailwind v4, shadcn/ui,
TypeScript. It is its own project the way `app/` is its own cargo workspace,
and it has its own conditional gate. It talks to the server over HTTP and to
nothing else.

**It runs on the maintainer's machine in Docker Compose**, four containers:

| container | role |
| --- | --- |
| `postgres` | the database, on a persistent volume |
| `scorsese-server` | the Rust server, with ffmpeg inside; library files, render cache and scratch mounted from host disk |
| `web` | nginx serving the built React files |
| `cloudflared` | a **Cloudflare Tunnel** — how the site reaches the internet without a static IP; it also terminates HTTPS, so there is no reverse proxy of our own |

**No message broker.** Long work — renders, Veo shots, ElevenLabs lines,
thumbnails, proxies — is a row in a Postgres `jobs` table, claimed by workers
and recovered on restart. A broker is one more container to run and back up,
for nothing at this scale.

The Cloudflare free plan caps a request body at about 100 MB, so **uploads are
chunked and resumable (tus)** and nothing may be configured that assumes a
large single request.

## Everything is per user

**One user can never see or touch another's projects, files, generations or
chat.** There is no sharing and no co-working in v1; a global shared library is
a someday idea. This is a hard rule, and it is enforced so that forgetting it in
a new query **fails loudly** rather than leaking (#533 decides the mechanism).

It reaches into caching too: a generated Veo shot is reused for free across the
**same user's** projects by its brief hash, and **never** across users.

## The edit is a document; everything around it is tables

A project's edit is stored as its **`project.json` document** in a JSON
column, loaded into `scorsese_core::Project`, edited by the **same** functions
the CLI and MCP use, validated by the same `Project::validate`, and saved back.
The timeline is **not** normalised into tables: the format is versioned and
changes often, and normalising it would fork every editing operation into SQL
— two implementations of every edit, drifting.

Everything *around* the edit is normalised: users, library items, which
library items a project uses (derived from the document on every save, never
edited by hand), templates, the credit ledger, the provider audit tables, chat
turns and tool calls, renders, jobs.

**Postgres, no extensions.** No pgvector: a library is dozens to hundreds of
items, and an assistant can read a filtered list of names and descriptions.
Revisit only if libraries get large.

**A schema bump migrates every stored document.** The format rule in
`CLAUDE.md` (*A schema bump ships with a migration*) exists because of this
page: the server runs the migration over stored projects when it starts on a
new build, before it serves anything.

## Files

- Exactly the file kinds scorsese supports today — no more, no fewer.
- Stored **once per (user, SHA-256)**. A byte-identical re-upload is
  **refused** with "you already have this as *X*", whatever its name.
- Lists carry thumbnail, name, kind and size only; the file is fetched when
  opened, and video and audio stream in ranges.
- The library is **flat** in v1: filter by kind, search by name, no folders.

## Money

- **Integer micro-dollars** (`BIGINT`), never floats — one assistant call can
  cost a fraction of a cent. Shown to the user as **≈ reais** at an
  operator-set, dated rate, because a balance in dollars drifts with the
  exchange rate and the page must not imply otherwise.
- Credits are paid up front and **never expire**. Veo, ElevenLabs and the
  assistant are passed through at **cost + 10%**, plus a **$10/month** flat
  fee. Top-ups are recorded by the operator until Pix exists (#548).
- A generation the provider **failed** is free; one that **worked** is
  charged, liked or not. Users can audit every charge themselves — that is what
  makes the second half acceptable.
- A tool that spends money **quotes first and spends only on confirmation**
  (#538), in the tool surface itself, so the built-in assistant, a user's own
  MCP client and the local server all get the same guard.
- Every figure the providers are quoted at is scorsese's own estimate
  (`docs/prices.md`); the assistant's cost is exact, from the token counts its
  responses report.

## The assistant

Claude Opus 5.5, running server-side with scorsese's tool registry (#540). The
same tools are served over web MCP (#539), so a user can connect their own
client instead. One tool set, two ways in — and nothing in the tool surface may
assume the caller is Claude (`CLAUDE.md`, *MCP is a protocol, not a Claude
feature*).

## Out of scope for now

Pix payments (#548), folders in the library, a shared cross-user library,
collaboration, cloud hosting.
