# The web app

Scorsese as a hosted service: a URL, a login, a library of a person's own
files, and an assistant that edits for them. It is a **convenience layer** over
the open-source app, for people who would rather not set anything up, and it
must be able to do everything the app does (`CLAUDE.md`, *The app is the
product; the web app is a convenience layer*).

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

**It runs on the maintainer's machine in Docker Compose**, five service
containers and one that keeps them safe, plus a short-lived container per
page capture:

| container | role |
| --- | --- |
| `postgres` | the database, on a persistent volume |
| `server` | `scorsese-server`, the Rust server, with ffmpeg inside; library files, render cache and scratch mounted from host disk |
| `web` | nginx serving the built React files |
| `named-tunnel` | `cloudflared` holding a **Cloudflare Tunnel** — how the site reaches the internet without a static IP; it also terminates HTTPS, so there is no reverse proxy of our own. Before there is a domain, `quick-tunnel` instead, or no tunnel (#568, *Testing before there is a domain*) |
| `backup` | scheduled `pg_dump` plus a sync of the library **off the machine** (#532). Not part of serving a request — the fifth container exists because a home machine has no redundancy, and a backup that depends on someone remembering is not one |
| `capture-launcher` | starts each capture of a user's web page (`html` clips) in a container of its own, **offline and sandboxed, mounting only that job** (#778, #852, *Page captures*). The one container holding the Docker socket, so it holds nothing else: no network, no database, no secret |

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
a new query **fails loudly** rather than leaking — Postgres row-level security,
argued under *Accounts* below.

It reaches into caching too: a generated Veo shot is reused for free across the
**same user's** projects by its brief hash, and **never** across users.

And to what lives in a vendor's account rather than in our database: a voice
a user designed is in the operator's ElevenLabs account, which every user
shares, and is still only theirs to use or look up (*Designing a voice*).

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
`crates/core/CLAUDE.md` (*A schema bump ships with a migration*) exists because of this
page: the server runs the migration over stored projects when it starts on a
new build, before it serves anything.

## Files

- Exactly the file kinds scorsese supports today — no more, no fewer. MIDI
  was a library kind (#678) until #964: once #785 took `synth_import` and
  `synth_export` off the list nothing on the web could use one, so a `.mid`
  is refused like any other file scorsese cannot use, and migration `0025`
  deleted the rows of the kind (there were none).
- Stored **once per (user, SHA-256)**. A byte-identical re-upload is
  **refused** with "you already have this as *X*", whatever its name.
- Lists carry thumbnail, name, kind and size only; the file is fetched when
  opened, and video and audio stream in ranges.
- The library is **flat** in v1: filter by kind, search by name, no folders.

## Money

- **Integer micro-dollars** (`BIGINT`), never floats — one assistant call can
  cost a fraction of a cent. Shown to the user **in US dollars, and only
  dollars** (#703): a converted figure is wrong the day after it is shown —
  $5 spent today and $5 spent last month were different amounts of any other
  currency — and asking users to pick a rate would put a technical choice in
  front of them. The one record of another currency is a payment's own row:
  what a user was charged in reais, and the rate it bought their dollars at
  (*Buying credits*).
- Credits are paid up front and **never expire**. Every paid generation —
  Veo shots, Gemini stills (one at a time or batched), ElevenLabs lines, music
  tracks and voice designs — and the assistant are charged at **cost times a multiplier
  per kind** (`credits::Product`: ×1.5 on a Veo shot, ×1.8 on the rest;
  *Credits*, *Pricing* has why), and **nothing else: there is no monthly fee**
  (#1243). Users top up themselves through Stripe (*Buying credits*, #548); the
  operator can still record one by hand.
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

A model of the project's choosing — Claude Opus 5.5 by default (#1444), or
Claude Sonnet 5.5 or Fable 5.1 (#705; Gemini 3.8 Flash and 3.5 Flash Lite were offered until #1220
cut them as not good enough at the job) — running server-side
with scorsese's tool registry (#540). The same tools are served over web MCP
(#539), so a user can connect their own client instead. One tool set, two ways
in — and nothing in the tool surface may assume who the caller is (`CLAUDE.md`,
*MCP is a protocol, not a Claude feature*). How a turn runs, what it costs and the API the web editor calls
are *Assistant turns*, below.

## `crates/server`

The API (#530): axum over HTTP, sqlx over Postgres. The crate's own `lib.rs`
doc argues both choices and the tool-registry decision; this is what a
developer needs to run it.

**Every route lives under `/api`**, the health check included
(`GET /api/health`: `200` when the database answers, `503` when it does not).
One prefix is one routing rule — for `web/`'s dev proxy, which forwards `/api`
unchanged to `SCORSESE_API`, and for whatever fronts the server in production.

| route | who | what |
| --- | --- | --- |
| `GET /api/health` | anyone | `200 ok` when the database answers, `503` when it does not |

**Every route is declared once**, in `crates/server/src/http/routes.rs`, and
the server is built from that list. Every route row on this page is held to
it by a test (#994): a route with no row, or a row with no route, fails the
build. Adding a route is one line there and one row here.

**Configuration** comes from the environment, through the same lookup the
provider keys use, and is documented in `.env.example`:

| Variable | What it is |
|---|---|
| `DATABASE_URL` | The Postgres to connect to. Required; the server will not start without reaching it, and it is never printed. |
| `SCORSESE_STORAGE` | The absolute directory users' files are kept under. Required. |
| `SCORSESE_CACHE` | The absolute directory what can be rebuilt is kept under — thumbnails, uploads still arriving, finished renders. Required, and refused inside `SCORSESE_STORAGE`, which is backed up. |
| `SCORSESE_RENDER_QUOTA` | How much disk finished renders and page captures aim to stay under: `500MB`, `20GB`, `1TB`. Defaults to `20GB`. See *Renders*. |
| `SCORSESE_BIND` | Where to listen. Defaults to `127.0.0.1:8080`, this machine only. |
| `GEMINI_API_KEY`, `ELEVENLABS_API_KEY` | The provider keys paid generations are made with (*Web MCP*), read by the one resolver `docs/credentials.md` describes; Optional: without one, a generation needing it fails, free. |
| `ANTHROPIC_API_KEY` | The key the assistant's Claude models answer with (*Assistant turns*), by the same resolver. Optional: without it a turn on a Claude model is refused with "`ANTHROPIC_API_KEY` is not set", and nothing else changes. |
| `PIXABAY_API_KEY` | The key `stock_search` and `stock_import` look Pixabay up with, by the same resolver, shared by every user (*Web MCP*). Optional: without it a Pixabay search fails with the variable's name; LottieFiles needs no key. |
| `SCORSESE_ASSISTANT_TURN_CAP` | The most one assistant turn on Sonnet 5.5 may cost a user, in dollars: `2.50`. Defaults to `3.50`. Every other model's cap scales from it with its rates (*Assistant turns*, *Money*): at `3.50`, about `7.00` on Opus 5.5 and `17.50` on Fable 5.1. |
| `SCORSESE_ELEVENLABS_CONCURRENCY` | How many ElevenLabs calls may be in flight at once, every kind of job together (*The job queue*): the plan's concurrent requests. Defaults to `3`, Starter's; Free allows `2`. A whole number above zero, or the server will not start. |
| `SCORSESE_ELEVENLABS_VOICE_SLOTS` | How many designed voices the ElevenLabs account holds, every user's together (*Designing a voice*). Defaults to `10`, Starter's. A whole number above zero, or the server will not start. |
| `GOOGLE_OAUTH_CLIENT_ID`, `GOOGLE_OAUTH_CLIENT_SECRET` | The Google OAuth client people sign in with (*Accounts*). Optional, with the next one: without all three, Google sign-in is off, the server says so in its log, and everything else is served. The secret is never printed. |
| `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET` | Stripe's secret key and the webhook endpoint's signing secret, for users buying credits (*Buying credits*). Optional, with the next one: without all three, buying is off, the server says so in its log, and a purchase is answered `503` "not configured". Both are secrets and never printed. |
| `SCORSESE_PUBLIC_ORIGIN` | Where browsers reach the site, `https://scorseseestudio.com`: what Google sends a sign-in back to, at `/api/auth/google/callback`, and where Stripe sends a buyer back to. `https://` and a host, or `http://localhost` / `http://127.0.0.1`; anything else stops the server from starting. |
| `SCORSESE_ASSISTANT_MODEL` | **Retired** (#705): the model is each project's choice now. Set to anything, it stops the server from starting, so a setting that no longer means anything cannot pass for one that does. |

**Schema migrations** are embedded in the binary and run at startup, before the
first request. Files in `crates/server/migrations/` are numbered
`NNNN_name.sql` without gaps, only go forward, and are never edited once merged
— the folder's README has the rules. One test refuses a file sqlx would
silently skip, and another holds every migration to the checksum in the
folder's append-only `merged.txt`, since sqlx refuses to start against a
database that applied a different version of a file (#1223).

**Database tests run, or they fail — they never skip.** `make test` and
`make coverage` go through `tools/with-postgres`, which starts a throwaway
`postgres:17-alpine` container on a random local port and removes it however
the run ends. So **Docker is needed for `make gates`**, unless
`SCORSESE_TEST_DATABASE_URL` names a Postgres the tests may create and drop
databases on. An ambient `DATABASE_URL` is deliberately ignored, so a test run
never touches the development database. CI's `check` job and the weekly coverage
workflow start a native Postgres 17 with `tools/ci-postgres` instead, from the
PostgreSQL project's apt repository rather than an image, since every
registry's anonymous limit was hit from GitHub's shared runners (#1260).

## Running the service

The deploy (#532) is `deploy/`: one Compose file, the images it builds, and
`deploy/.env.example`, which documents every setting. `make deploy` is a gate
that holds the two together — the Compose file parses, and every variable it
reads is in the example.

| file | what it is |
| --- | --- |
| `compose.yaml` | the containers, their health checks and restart policies |
| `server.Dockerfile` | `scorsese-server`, release build, with ffmpeg on `PATH` |
| `web.Dockerfile` | `web/`'s Vite build, served by nginx |
| `nginx.conf` | the `/api` split, the SPA fallback, upload and streaming settings |
| `backup/` | the backup image and its three scripts |
| `capture/seccomp.json` | the one seccomp profile every page capture runs under: Docker's default plus the one grant Chromium's sandbox needs, mounted into `capture-launcher`, which refuses to start without it (*Page captures*) |

**The traffic path.** Cloudflare's edge terminates HTTPS and sends the request
down the tunnel `cloudflared` holds open from this machine. `cloudflared` sends
everything to `web` on its port 80; nginx serves the React build and passes
`/api/` to `server`, unchanged but for naming the client's address
(*The access record* has why). Nothing is published to the network: `web`
also answers on `$SCORSESE_WEB_BIND:$SCORSESE_WEB_PORT` — its port 8080 inside,
a second door so it can tell the two apart — loopback unless `.env` says
otherwise, for checking the site from this machine and for `tailscale serve`.

**Which tunnel is a setting, not an edit.** `COMPOSE_PROFILES` in `deploy/.env`
names one: `named-tunnel` (production — a domain and a token), `quick-tunnel` (a
throwaway `*.trycloudflare.com` address, for testing), or blank for none. Both
are the same `cloudflared` image told two different things, and Compose starts
only the one named. `make deploy` checks the file in all three modes, and that
each starts its own tunnel and no other.

**The `/api` split lives in nginx, not in the tunnel's ingress rules.** The
tunnel's rules are Cloudflare's, and reach it through Terraform (*Infrastructure, as
code*); a split there would still be one no local run exercises. In
`nginx.conf` it is identical on loopback and on the public hostname, and nginx
is needed for the SPA fallback anyway, so the tunnel's ingress names hostnames
and nothing else.
The MCP endpoint (#539) is `/api/mcp`, under `/api` like everything else, so it
needs nothing of its own there.

**The `backup` container** is the one in *The shape* that serves no request. It runs on the
database's own image, so `pg_dump` is always the server's version, and it
starts and stops with the rest — a host cron job would be one more thing set up
by hand on the machine and forgotten on the next one.

**After a power cut** the Docker daemon starts at boot and brings back every
container, which all carry `restart: unless-stopped`. The server waits for a
healthy database, runs any migrations, and answers `GET /api/health`; the job
queue (#536) resumes interrupted work. Only `docker compose stop` keeps a
container down. There is no GPU passthrough — nothing needs it until NVENC
(#311).

**What `SCORSESE_DATA` holds.** `library/` is users' files (the server's
`SCORSESE_STORAGE`) and is backed up. `backups/` holds the newest few database
dumps and the time of the last good backup. `cache/` (`SCORSESE_CACHE`, made by
the server on start) holds what can be rebuilt — thumbnails, uploads still
arriving, finished renders (*Renders*) and the scratch they are made in, page
captures and their spool (*Page captures*) — and
anything rebuildable a later issue adds belongs beside them too, never inside
`library/`, so it is not shipped off the machine every night.

### First-time setup

Steps 3 and 4 need the maintainer's own accounts; nothing in the repo can do
them. The rest is commands, from the checkout on the machine that serves.

1. **Docker starts at boot**: `sudo systemctl enable --now docker`. Without
   it, a power cut is an outage until someone logs in.
2. **The data directory**, owned by the user the containers run as — Compose
   refuses to create it, so a typo or an unmounted disk fails loudly:

       mkdir -p /path/to/scorsese-data/library /path/to/scorsese-data/backups

3. **The Cloudflare Tunnel** — production, once there is a domain; before
   that, skip this step and see *Testing before there is a domain* below.
   The domain must be on Cloudflare (its nameservers pointed there). In the dashboard: *Zero Trust → Networks →
   Tunnels → Create a tunnel → Cloudflared*, name it, and copy the token from
   the install command shown (the string after `--token`); skip installing
   the connector, the `cloudflared` container is the connector. The token
   goes in `CLOUDFLARE_TUNNEL_TOKEN`, and `COMPOSE_PROFILES=named-tunnel`
   turns it on. **Add no public hostname in the dashboard**: the hostnames,
   their DNS records and the rest come from `make infra-apply` (*Infrastructure,
   as code*), each to `http://web:80` — nginx's tunnel-only door, never 8080
   (*The access record* says why).
4. **The backup destination.** Somewhere off this machine the maintainer
   chooses — an object store (Backblaze B2, S3, R2), a cloud drive, another
   machine over SFTP. `rclone config` creates a remote for it; then
   `SCORSESE_BACKUP_REMOTE` is `<remote>:<folder>` and `SCORSESE_RCLONE_CONFIG`
   the directory holding that `rclone.conf`. **Keep that remote's credentials
   somewhere other than this machine too** (a password manager): the day this
   machine is lost is the day they are needed, and a copy that lived only here
   went with it.
5. **The settings**: `cp deploy/.env.example deploy/.env` and fill in every
   line; the example says what each is and how to generate the password.
6. **Up**, from `deploy/`:

       CARGO_BUILD_JOBS=4 docker compose up --detach --build

   The first build compiles the server in release mode, several minutes; later
   builds reuse BuildKit's cache and recompile only what changed.
   `CARGO_BUILD_JOBS` leaves room for whatever else the machine is doing.
7. **Check**: `docker compose ps` shows every container healthy (`backup`
   after its first run, which starts immediately), `curl
   http://127.0.0.1:8088/api/health` answers `ok`, and so does the tunnel's
   address, if there is one. `docker compose logs backup` shows the first backup reaching the
   remote.
8. **Restore once, on purpose** (below), into this fresh install. A backup
   that has never been restored is a hope.

### Testing before there is a domain

Production is the named tunnel of step 3. Before there is a domain there are
three ways in, and they combine — a quick tunnel and a tailnet can run at the
same time:

| path | who reaches it | address | in `deploy/.env` | besides |
| --- | --- | --- | --- | --- |
| quick tunnel | anyone with the link | `https://<words>.trycloudflare.com`, new on every restart | `COMPOSE_PROFILES=quick-tunnel` | nothing |
| tailnet | the maintainer's devices, and friends invited to the tailnet | `https://<machine>.<tailnet>.ts.net` | nothing — the loopback default | Tailscale on this machine and theirs; `tailscale serve` once |
| home network, plain HTTP | devices on the LAN | `http://192.168.x.x:8088` | `SCORSESE_WEB_BIND=0.0.0.0` | **a browser cannot stay logged in** — below |

**Quick tunnel.** Set `COMPOSE_PROFILES=quick-tunnel`, `docker compose up
--detach` from `deploy/`, and read the address out of the log:

    docker compose logs quick-tunnel | grep -o 'https://[-a-z0-9]*\.trycloudflare\.com'

It is HTTPS, so a session works as in production. What to know about it:

- **The address changes every time the container starts** — after a power cut,
  after an `up` that recreates it. Read the log again and send the new link.
- **It is public.** Anyone with the link reaches the landing page and can
  sign up with Google (*Accounts*), at zero credits. Signing in through it
  needs its callback added to the OAuth client (*Sign in with Google: the
  client*), and the address changes on every restart — so for testing the
  sign-in itself, the tailnet's fixed address is the easier one.
- **Cloudflare's limits for it**: 200 requests in flight, no uptime guarantee,
  and **no server-sent events**, so `GET /api/events` (live job progress) does
  not stream through it. Uploads are unaffected: they are chunked under the
  100 MB a Cloudflare request may carry, whichever tunnel it is.

**Tailnet**, which is also the answer for the maintainer's own devices at home.
`web` stays on loopback; on this machine, once:

1. Tailscale installed and logged in: `sudo systemctl enable --now tailscaled`,
   `sudo tailscale up`.
2. In the Tailscale admin console, *DNS*: **MagicDNS** on and **HTTPS
   Certificates** enabled.
3. `sudo tailscale serve --bg 8088` — the tailnet address, over HTTPS with a
   real certificate, proxied to `http://127.0.0.1:8088`, and remembered across
   reboots. `tailscale serve status` shows it, `sudo tailscale serve reset`
   removes it.

Friends install Tailscale and either join the tailnet by invitation or accept
a share of this one machine (admin console, *Machines → Share*); both open the
same `https://` address, and the tailnet's access rules decide who gets there.
The port is never opened to the LAN for any of this.

**Home network, and why plain HTTP from another device cannot log in.** The
session cookie is `Secure` (*Accounts*), and a browser keeps a `Secure` cookie
only from `https://` or `localhost`. From `http://192.168.x.x:8088` a sign-in
sets the cookie, the browser drops it, and the next request is logged
out. The same goes for `http://100.x.y.z:8088` on the tailnet, encrypted
underneath or not — the browser sees plain HTTP, which is what `tailscale
serve` is for. So a device at home logs in through the tailnet or the quick
tunnel.

A setting that drops `Secure` for a "trusted network" was weighed and **not
built** (#568):

- Every path planned before a domain already has HTTPS for nothing — the quick
  tunnel and `tailscale serve` — and a phone joins a tailnet in about the time
  it takes to find the machine's LAN address.
- Over plain HTTP the session crosses the network in the clear.
  A home Wi-Fi with guests and appliances on it is not a trusted network, and a
  flag named for one gets left on.
- The server that reads the flag is the one behind the tunnel. Left on in
  production, a browser would send the session over `http://` to the public
  hostname — exactly what `Secure` exists to prevent.
- Browsers gate more than cookies on HTTPS (the clipboard, Web Crypto, service
  workers). Plain HTTP would be a place the web app keeps breaking in new ways
  as it grows, not a place it works minus one thing.

`SCORSESE_WEB_BIND` beyond loopback is therefore for what is not a browser
session: a client with an API token (web MCP, `curl`) or a health check from
another machine. Prefer `0.0.0.0` to one interface's address: an address that
is not up yet when Docker starts the container at boot — a tailnet address, a
DHCP lease — is a `web` container that does not start.

### Infrastructure, as code

Everything a vendor lets Terraform manage is managed in `deploy/terraform/`
(#957, #952, #1256, #1352), so it is versioned, reviewed in a pull request, and something an
agent can change. A change made in a dashboard is drift: the next plan shows
it, and the next apply undoes it. There is one configuration per vendor, each
with its own state in one versioned Google Cloud Storage bucket,
`gs://scorseseestudio-tfstate`, so applying from a worktree that is then
removed, or from a machine that then dies, loses nothing (*Where the state
lives*, below).

| File | What it holds |
| --- | --- |
| `cloudflare/zone.tf` | the zone, looked up by `domain`; DNSSEC on |
| `cloudflare/tunnel.tf` | the tunnel's ingress (the apex and `www` to `http://web:80`, anything else 404), their proxied CNAMEs, and the `www` → apex 301 |
| `cloudflare/email.tf` | Email Routing's records, the `security@` and `contato@` forwards (one verified destination, `SCORSESE_SECURITY_FORWARD_TO`), and DMARC `p=reject` (the domain never sends mail) |
| `google/project.tf` | the `scorseseestudio` Google Cloud project (deletion-protected), the billing account it is charged to, and the APIs enabled on it |
| `google/billing.tf` | a monthly budget on the project, R$5 (about US$1; the billing account is in reais, and a budget must be in its account's currency), emailing the billing admins at 50% and 100%. It warns; it stops nothing |
| `google/state.tf` | the bucket every configuration's state lives in: `us-central1` (Always Free), private, versioned, impossible to destroy by accident (`prevent_destroy`, no `force_destroy`) |
| `stripe/webhook.tf` | the webhook endpoint for credit purchases (*Buying credits*): `<SCORSESE_PUBLIC_ORIGIN>/api/payments/stripe/webhook` and the three events it acts on, and its signing secret as a sensitive output |
| `github/repository.tf` | the `MatthewLacerda2/scorsese` repository's settings: squash merges only, a merged branch deleted, no wiki or projects, Dependabot alerts and security updates on, secret scanning and push protection on. Imported, never created, and impossible to destroy by accident (`prevent_destroy`, and `archive_on_destroy` behind it) |
| `github/ruleset.tf` | `main`'s ruleset: a change arrives as a squashed pull request with `CI ok` green on `main`'s tip, no review required, no deletion or force-push, nobody bypasses it |
| `tf` | runs one configuration with its credentials and its state: `tf cloudflare plan`, `tf google apply`, `tf stripe secret`, `tf github plan`; and, once, `tf google bootstrap` and `tf <name> migrate` (*Moving the state into the bucket*) |

    make infra-plan     # what would change, for all four; changes nothing
    make infra-apply    # shows each plan again and asks before applying it

**Credentials.** `tf cloudflare` reads three values from `deploy/.env`:
`CLOUDFLARE_API_TOKEN` (scoped to this account and zone;
`deploy/.env.example` lists its permissions), `CLOUDFLARE_TUNNEL_TOKEN`, which
already names the account and the tunnel so neither ID is written anywhere
else, and `SCORSESE_SECURITY_FORWARD_TO`, kept out of the repository because it
is a personal mailbox. `tf google` uses the maintainer's own `gcloud` login,
through a short-lived access token, so no service-account key exists anywhere.
`tf stripe` reads `STRIPE_SECRET_KEY` and `SCORSESE_PUBLIC_ORIGIN`.
`tf github` uses the maintainer's own `gh` login (`gh auth token`), so no
GitHub token is kept anywhere either; it needs an admin of the repository.
**Every** configuration also needs the `gcloud` login, because every state is
in the bucket: `tf` hands Terraform's `gcs` backend the same short-lived token
(`GOOGLE_OAUTH_ACCESS_TOKEN`), and stops with one line saying so when there is
no login. `make infra`, the gate, checks formatting and each configuration
against its provider's schema without credentials or state
(`init -backend=false`), in `make gates` and in CI's `deploy config` job.

**Where the state lives.** A state records what Terraform made, and two of
them hold secrets: Cloudflare's tunnel token and Stripe's signing secret. So
they live in `gs://scorseseestudio-tfstate` (`google/state.tf`), not on one
disk that no backup covers: one prefix per state (`cloudflare`, `github`,
`google`, `stripe-test`, `stripe-live`), each a `default.tfstate` object that
Terraform locks while it writes. Public access is prevented and access is
uniform, so only the project's owners read it. Every apply leaves the
previous version behind; an old version is deleted once it is both more than
30 days old and 20 versions behind, so there is always a month and always
twenty applies to go back to. Going back is copying a noncurrent version over
the live one: `gcloud storage ls --all-versions gs://scorseseestudio-tfstate/<prefix>/`
lists them, `gcloud storage cp <that version's URL> gs://scorseseestudio-tfstate/<prefix>/default.tfstate`
restores one. It costs nothing (kilobytes inside the free tier) and Cloud
Storage is the one part that made the project need a billing account, which
the budget watches. The bucket was chosen over Cloudflare R2 (no object
versioning), HCP Terraform (one more account) and a gist or the repository
(both public).

**Moving the state into the bucket** is done once, on the machine that holds
the old states (the maintainer linked the billing account it needs by hand on
2026-10-15). Before the bucket, each state was a file in
`$SCORSESE_DATA/terraform/<state>.tfstate`. The google configuration makes
the bucket, so it goes first, against its old file, and then every state
moves in, one command each:

    deploy/terraform/tf google bootstrap plan    # bucket, budget, two APIs; the project unchanged
    deploy/terraform/tf google bootstrap apply
    deploy/terraform/tf google migrate           # answer yes when Terraform asks to copy
    deploy/terraform/tf cloudflare migrate
    deploy/terraform/tf github migrate
    deploy/terraform/tf stripe migrate           # stripe-test and stripe-live; needs no key
    make infra-plan                              # every configuration: no changes

`migrate` copies the file to `<state>.tfstate.migrated` first, runs
`terraform init -migrate-state`, checks the bucket holds the same state
(Terraform's lineage), and only then removes the original; delete the
`.migrated` copies (and `cloudflare.tfstate.backup`) once `make infra-plan`
shows no changes, since they hold the same secrets on the same unbacked disk.
Until a state has moved, every other `tf` command on it refuses, so nothing is
ever planned or applied against an empty state, which would try to make
everything a second time. `bootstrap` refuses once google's state is in the
bucket.

**What stays by hand**, because no API reaches it or it should not be
automated:

- Registering and renewing the domain.
- **Creating** the tunnel. Its token is the connector's identity, made once
  and imported, never recreated.
- The Cloudflare API token itself, and verifying the `security@` forward's
  destination address, which is confirmed by a link in that mailbox.
- **Google's OAuth consent screen and the sign-in client** (#952). Google
  offers no public API for either; the `google_iap_*` resources are for IAP,
  a different product. In the console, with the `scorseseestudio` project
  selected: *Google Auth Platform → Branding* (app name `scorsese`, the
  support and developer email, home page `https://scorseseestudio.com`),
  *Audience → External*, then *Clients → Create client → Web application*
  with the authorised redirect URI
  `https://scorseseestudio.com/api/auth/google/callback`. The client's ID and
  secret go in `deploy/.env`.

**Stripe's signing secret.** Stripe returns an endpoint's secret only when it
creates the endpoint, and the provider keeps it in the state as the sensitive
`webhook_secret` output. After the apply that creates it:

    deploy/terraform/tf stripe secret   # writes STRIPE_WEBHOOK_SECRET into deploy/.env

It prints nothing, replaces the line in place, and keeps the file's
permissions; `docker compose up --detach` then restarts the server with it. An endpoint
**imported** rather than created has no secret in the state, so `secret`
refuses: roll the secret in Stripe's Dashboard and paste it by hand, or delete
that endpoint there and let the apply make a new one.

**Test and live mode are different endpoints at Stripe**, each with its own
secret. `tf` keeps a state for each, under the bucket's `stripe-test` or
`stripe-live` prefix, chosen by whether `STRIPE_SECRET_KEY` is a test or a
live key, so the test endpoint is neither forgotten nor touched when the key
changes. Switching to live (after #547) is: put the `sk_live_…` key in
`deploy/.env`, `make infra-apply`, `deploy/terraform/tf stripe secret`,
restart the server.

Something that exists already is **imported** before the first apply, or the
apply tries to create it a second time:

    deploy/terraform/tf cloudflare import cloudflare_zero_trust_tunnel_cloudflared_config.site <account_id>/<tunnel_id>
    deploy/terraform/tf cloudflare import cloudflare_zone_dnssec.site <zone_id>

The GitHub repository and its ruleset are imported by `import {}` blocks in
`github/` itself, so the first apply does it and nobody runs an import by hand.

**The ruleset requires `CI ok`**, the one job in CI that stands for every
other (#1351). A required check that never reports blocks every pull request,
so that job must be on `main` before the ruleset asking for it is applied.
The check is also *strict*: the branch must hold `main`'s tip. That costs
nothing, because every merge goes through `make queue`, which rebases first.

### Updating

From the checkout, on `main`:

    git pull
    cd deploy
    docker compose exec backup backup          # a dump from just before
    CARGO_BUILD_JOBS=4 docker compose up --detach --build
    ./purge-landing                            # Cloudflare's copy of `/`

The last step matters: Cloudflare keeps the landing page's HTML at its edge
(`deploy/terraform/cloudflare/cache.tf`), and that copy names the previous
build's fingerprinted files, which the new `web` image no longer has. Until it
is dropped, or expires within the hour, a first visit draws a blank page.

Compose recreates only the containers whose image or settings changed. The new
server runs its migrations before serving, and they only go forward, so **the
way back from a bad update is the dump taken first**: check out the previous
commit, rebuild, and restore that dump. BuildKit's cache for the Rust build
grows over time; `docker buildx du` shows it.

**If `SCORSESE_DATA` has moved** since the last `up`, Compose asks whether to
recreate the volume `capture-spool`, warning that data will be lost. Answer
yes: that volume only points `capture-launcher` at `cache/captures`, and
recreating it deletes nothing (`deploy/compose.yaml` has why the spool is a
volume, #861). Answered no, or run without a terminal, it keeps the old path,
and every page capture waits out its 30 s and draws a slug card.

### Backups

`backup` runs one when the last good one is more than
`SCORSESE_BACKUP_EVERY_HOURS` old, checking hourly — so a machine that was off
at the usual time catches up within an hour of coming back, and a failure is
retried an hour later. Each run:

- dumps the database with `pg_dump --format custom` into `backups/db/`
  (keeping the newest three there, for a fast restore), and copies the dumps to
  `<remote>/db`, where they are kept `SCORSESE_BACKUP_KEEP_DAYS`;
- syncs `library/` to `<remote>/library`, and moves every remote file the sync
  would delete or overwrite into `<remote>/library-replaced/<when>` for the same
  number of days, counted from `<when>` (when it was replaced, not when it was
  uploaded) — so a bug that deletes users' files is not faithfully
  mirrored into the backup that night.

The container's health is the backups': **unhealthy** once the last good one is
older than two intervals. `docker compose ps` is where to look, and
`docker compose logs backup` says why. `docker compose exec backup backup` runs
one now.

### Restoring

From `deploy/`, with the stack up. Everything runs inside the backup image, so
the host needs no Postgres or rclone tools. Stop what writes first:

    docker compose stop server backup

**The database**, from a local dump (`ls "$SCORSESE_DATA"/backups/db`) or one
fetched from the remote first:

    docker compose run --rm backup sh -c \
      'rclone copy "$SCORSESE_BACKUP_REMOTE/db/scorsese-<when>.dump" /data/backups/db'
    docker compose run --rm backup pg_restore --clean --if-exists \
      --single-transaction --dbname scorsese /data/backups/db/scorsese-<when>.dump

`--clean` drops what is there before recreating it, so this works on top of a
running install as well as on a fresh one. `--single-transaction` means a
failed restore leaves the database as it was.

**The library**, to put back what is missing (`sync` instead of `copy` would
also delete what the backup does not have):

    docker compose run --rm backup sh -c \
      'rclone copy "$SCORSESE_BACKUP_REMOTE/library" /data/library'

A file deleted or overwritten since is under `library-replaced/<when>/` on the
remote, by the same path. Then:

    docker compose start server backup

**On a new machine** after losing this one: first-time setup steps 1–6 with the
same remote and credentials (the tunnel's token is in the dashboard still),
then the restore above.

## Accounts

Who may use the server, and how each request proves it (#533). The code is
`crates/server/src/accounts/` and `crates/server/src/http/google/`, and each
module's doc carries its argument; this is the whole of it in one place.

**Sign in with Google is sign-up and login at once** (#952). There are no
passwords. "Continue with Google" on `/login` gives a new Google account a
scorsese account on its first sign-in, **at zero credits**, so nothing paid
runs until credits are added (*Credits*); a returning one is simply logged in.
There is no invite mode and no switch.

**An account is a Google identity.** On a first sign-in, an account that
already exists under Google's **verified** email — made before #952, or by the
operator's `user create` — is linked: Google's id for the person (`sub`,
"unique among all Google Accounts and never reused") is stored on it, and from
then on the `sub` is the identity and the email is never looked at again. So
somebody who changes their Google address keeps their account, and somebody
later handed that old address does not get it. An unverified email is
refused, and so is a sign-in whose email's account is already linked to a
different Google account. Where the addresses differ, the operator links by
hand; at this size that is acceptable, so there is no linking UI.

The operator's commands, run where the server runs:

```text
docker compose exec server scorsese-server user create ana@example.com  # linked at their first sign-in
scorsese-server user list                     # each account, its name and the Google id it signs in as
scorsese-server user link ana@example.com 110169484474386276334
scorsese-server user logout ana@example.com   # ends every browser session; tokens stay
scorsese-server user delete ana@example.com --yes   # rows and files, for good
scorsese-server user session ana@example.com  # a session cookie, for a stack without Google
scorsese-server token create ana@example.com "ana's laptop"
```

Linking by hand usually follows a first sign-in under another address, which
made an empty account holding the `sub`: `user link` refuses while another
account holds it and says so, the operator `user delete`s the empty one, and
links again. `user session` prints a cookie for a development stack with no
Google client, and is how the web app's end-to-end flows sign in (`make
web-e2e`), so no test reaches Google.

**The flow** is OAuth 2.0's authorization code with PKCE, done server-side, as
OpenID Connect describes it:

1. `GET /api/auth/google?next=/somewhere` mints a `state` and a PKCE
   verifier, hands them to the browser in a ten-minute cookie
   (`scorsese_google`, `HttpOnly; Secure; SameSite=Lax; Path=/api/auth/google`),
   and sends the browser to Google with the state and the verifier's `S256`
   challenge.
2. Google sends it back to `GET /api/auth/google/callback` with a code and the
   state. The state must equal the cookie's — no other site can set a cookie
   for this origin, which is what stops somebody planting their own sign-in in
   your browser — and the server trades the code for an ID token
   back-channel, with the verifier and the client's secret.
3. The ID token names the Google account, the account is found, linked or
   made, and the browser gets **the session cookie below** and lands on
   `next` when that is a path on this site, `/projects` otherwise. Anything
   that goes wrong lands on `/login?error=…` instead (`cancelled`, `expired`,
   `unverified`, `linked`, `unconfigured`, `failed`), which the page says in
   words; the server's log names the cause of a `failed`.

The flow's cookie is **unsigned and not a table**: nothing in it needs to be
believed (the state is proven by matching, the verifier is Google's to check),
and a row would need a sweep for every sign-in somebody abandoned. It is `Lax`
where the session is `Strict` because the callback is a top-level navigation
from `accounts.google.com`, on which a `Strict` cookie is not sent.

**The ID token's signature is not checked**, deliberately: it arrives in the
server's own HTTPS call to Google's token endpoint, never from a browser, and
OpenID Connect Core §3.1.3.7 lets TLS validate the issuer in that case;
Google's page says the same of its server flow. What is checked is what TLS
does not say: `iss` is Google, `aud` is this client, `exp` has not passed, and
`email_verified` is true. `crates/providers/src/sign_in` has the citations.
The exchange lives in `scorsese-providers` like every other call to a vendor,
behind a trait, so the tests trade codes with a recorded token.

**Configuration**: `GOOGLE_OAUTH_CLIENT_ID`, `GOOGLE_OAUTH_CLIENT_SECRET` and
`SCORSESE_PUBLIC_ORIGIN`. All three or Google sign-in is off: the server
still starts and serves everything else, says so in its log, and a sign-in
lands on `/login?error=unconfigured` — so a development stack without a Google
client keeps running. The redirect URI is `$SCORSESE_PUBLIC_ORIGIN` plus
`/api/auth/google/callback`, **configured, never read off a request's
`Host`**, which is whatever the client typed. *Sign in with Google: the
client* below has the console steps.

**The browser holds a session cookie**: 32 random bytes, of which Postgres
keeps only the SHA-256, valid 30 days from sign-in and not extended by use.
`HttpOnly; Secure; SameSite=Strict; Path=/api` — the web app and the API share
one origin, so Strict costs nothing, and with every write taking a JSON body
it is the CSRF defence. `Secure` means a browser signs in only over `https://`
or on `localhost`; *Testing before there is a domain* has the HTTPS paths
there are before a domain, and why plain HTTP is not one of them. Server-side rather than signed, so logout, `user logout` and
a deletion end sessions *now* — and so **there is no signing key**: the server
has no secret of its own for `docs/credentials.md`'s resolver to find. (The
OAuth client's secret is Google's: it proves the server to Google and signs
nothing here.)

**Beside it, a hint that a session exists** (#1246): `scorsese_signed_in=1`,
`Path=/; Secure; SameSite=Strict`, the session's lifetime, and **not**
`HttpOnly`, because its only reader is the page. No script can read the
session itself, so without it `/` would draw the landing page (and start its
film) before `/api/me` answered for a signed-in visitor. It carries nothing — no
id, no token — and decides only what is drawn while `/api/me` is asked, never
who anybody is: a forged one costs its owner a moment's wait, and the page drops
a stale one when `/api/me` answers `401`. Sign-in sets it and logout clears it,
in the same response as the session (`crates/server/src/http/auth.rs`, *The
hint beside it*).

**Anything that is not a browser sends a per-user API token** —
`Authorization: Bearer scor_…`. That is the answer for web MCP (#539) in v1:
the MCP spec's remote authorization is OAuth 2.1, but the clients people will
point at it first accept a static bearer header, and an authorization server
(client registration, consent, refresh, PKCE) is a large surface for no client
that needs it yet. When one that *only* speaks OAuth matters — claude.ai's
custom connectors are the likely one, #942 — that is its own issue, and its
flow ends by issuing these same tokens; it may reuse the Google sign-in as its
login step. Tokens are shown once, hashed like sessions,
revoked one at a time, and **issued only from a browser session**, so a leaked
token cannot mint its own replacement.

| route | who | what |
| --- | --- | --- |
| `GET /api/auth/google` | anyone | `?next=` a path; `303` to Google with a flow cookie, or to `/login?error=unconfigured` |
| `GET /api/auth/google/callback` | anyone | Google's `code` and `state`; `303` to `next` with the session cookie, or to `/login?error=…` |
| `POST /api/logout` | a member | ends the session |
| `GET /api/me` | a member | the account |
| `GET /api/tokens` | a member | their tokens, never the values |
| `POST /api/tokens` | a member, by session | `{name}` → `{id, token}`, shown once |
| `DELETE /api/tokens/{id}` | a member | `404` for an id that is not theirs |

### Sign in with Google: the client

The OAuth client is the one part Terraform cannot make: Google has no public
API for the consent screen or a standard *Web application* client (the
`google_iap_*` resources are IAP-only). The Google Cloud project itself, a
dedicated `scorsese` one, is Terraform's (#1239). Then, in the console, on
that project:

1. *Google Auth Platform → Branding*: the app's name (scorsese), a support
   email, the logo, and the domain `scorseseestudio.com` under authorised
   domains. *Audience*: **External**, and publish it (in *Testing* only
   listed test users can sign in). The scopes asked for are `openid`,
   `email` and `profile` (for the person's name, #1266), none of which need
   verification by Google. The profile picture comes with `profile` and is
   never stored.
2. *Clients → Create client*: type **Web application**, name it, and add the
   authorised redirect URI `https://scorseseestudio.com/api/auth/google/callback`
   — exactly `$SCORSESE_PUBLIC_ORIGIN` plus `/api/auth/google/callback`. For a
   tailnet or quick-tunnel test, add that origin's callback too.
3. Copy the client id and secret into `deploy/.env` as
   `GOOGLE_OAUTH_CLIENT_ID` and `GOOGLE_OAUTH_CLIENT_SECRET`, set
   `SCORSESE_PUBLIC_ORIGIN=https://scorseseestudio.com`, and `docker compose
   up --detach` from `deploy/`. The server's log says `signing in with
   Google, back to https://scorseseestudio.com`.
4. Sign in once with the maintainer's own Google account and check `user
   list`: account 1 now shows a Google id, and no new account was made.

### The access record

Marco Civil da Internet (Lei 12.965/2014) art. 15 asks an application run by
a company for profit to keep the address, date and time of each access for
at least six months (#1267). The server writes it to the `access_log` table,
privileged, at two moments, never on every request:

- **`sign_in`**: each Google sign-in that opens a session;
- **`day`**: an account's first request of each day (UTC), by session or by
  API token, so a month-long session or an MCP client's token still leaves a
  row for every day it is used. Which accounts are done today is kept in the
  server's memory, so the database is asked once per account per day; a
  restart forgets it, and a unique index makes the repeat write a no-op.

Each row is `user_id`, `kind`, `address` (an `inet`) and `at`. The address is
the one nginx names in `X-Scorsese-Client`; with no nginx in front, the
server's peer. A write that fails is logged and never fails the request, and a
missed `day` is written on the account's next request.

**Two doors, so nobody names another's address** (`deploy/nginx.conf`, #1245).
nginx listens on two ports, and the port a request came through decides whose
word is taken for its address:

- `web:80` is the tunnel's door. Compose publishes it to nothing, so only
  `cloudflared` connects there, and Cloudflare writes `CF-Connecting-IP`
  itself: it is believed.
- `web:8080` is the one compose publishes, on `$SCORSESE_WEB_BIND`: loopback
  by default, which `tailscale serve` can front, or the LAN if `.env` widens
  it. There
  `CF-Connecting-IP` is whatever the client typed, so it is ignored and the
  connection's own address is recorded.

One port that took `CF-Connecting-IP` from everyone would let any client on
the published port write somebody else's address into the record, and leave
loopback requests with no address at all. The door is a port rather than an
address range because container addresses are Docker's to hand out. The
login's brake (#557) was the header's first reader; it went with the
passwords (#952).

**Kept forever, and past the account** (the maintainer, 2026-10-14): six
months is the law's minimum, not a ceiling, so nothing prunes the table. It
is not per user (`tests/isolation.rs` lists it with the reason): `user_id`
has no foreign key, so `user delete` leaves the rows, and account ids are
never reused, so they keep naming the same account. No member may read it,
their own rows included, and nothing in the server does.

The operator reads it as the server's login role, read-only, where the
server runs:

```sh
docker compose exec -e PGOPTIONS='-c default_transaction_read_only=on' \
  postgres psql -U scorsese scorsese -c "
SELECT a.at, a.kind, host(a.address) AS address, a.user_id, u.email
FROM access_log a LEFT JOIN users u ON u.id = a.user_id
WHERE a.user_id = 7 AND a.at > now() - interval '6 months'
ORDER BY a.at"
```

A deleted account's rows show no email: the record names an account by its id
alone. To ask who used an address, filter on
`a.address = '203.0.113.9'` (or `<<= '2001:db8::/64'`) and a time range
instead.

### Per-user isolation: how a new table follows it

Enforced by Postgres, so that forgetting it is an error rather than a leak.
`crates/server/src/db/scope.rs` has the argument; the rule for whoever adds a
table (projects, library, jobs, credits…) is:

1. **The table carries `user_id BIGINT NOT NULL REFERENCES users (id) ON
   DELETE CASCADE`**, enables row-level security, and has a policy
   `USING (user_id = (SELECT member_id()))` — copy `0001_accounts.sql`.
   `tests/isolation.rs` reads the catalog and fails on any table that does
   not. The cascade is also what makes account deletion complete.
2. **Queries that act for a user run in `db::scoped(pool, user)`**, and need no
   owner filter: the policy is the filter. Inserts write `member_id()` as the
   owner, so a user id never travels through query text.
3. **`db::privileged` is only for what is cross-user by nature** — finding
   whose a cookie or token is, signing in, operator commands. Keep that list
   short; every call is a place review reads twice.

What makes forgetting loud: the server's connections sit in
`scorsese_unscoped`, a role granted no table at all, so a query on the pool
directly is `permission denied` — on the first run of the first test, even
against an empty table. Only a scoped transaction steps into
`scorsese_member`, the role the policies bind. This guards against mistakes,
not SQL injection (`SET ROLE NONE` is open to any role); binding values is the
injection defence. It also means **the login role must be a superuser or hold
`CREATEROLE`**, because migration `0001` creates those two roles — the compose
file's `POSTGRES_USER` is a superuser, so nothing needs configuring.

**A user's files live under `$SCORSESE_STORAGE/users/<user id>/`**, and
what can be rebuilt from them under `$SCORSESE_CACHE/users/<user id>/`, and
nowhere else, so deleting an account is two directories. Named by id, never
email: an email is personal data with no business in a path or a backup
listing. Deletion removes the rows first, then the directory; if the files
cannot all be removed, the command names the directory for the operator to
finish by hand.

Not in v1: other identity providers (Apple, GitHub) — one is enough to have
no passwords (#952).

## Jobs

Long work — a render, a Veo shot, a spoken line, a music track, a thumbnail, a proxy — is a
row in the `jobs` table, run by a worker inside the server (#536). The code is
`crates/server/src/jobs/`, and its module doc carries the argument.

**States**: `waiting → running → done | failed | stuck | cancelled`. `stuck` is
a provider job that outlasted its patience; it is not lost (below).
`cancelled` is one its owner stopped (#660).

**Stopping one.** Its owner may stop a `render` or a `preview` — nothing else
(`jobs::kinds::STOPPABLE`): a paid generation is billed whether or not anybody
still wants it, and a thumbnail or proxy is the server's own housekeeping. A
waiting job is marked `cancelled` in the table and never claimed. A running one
is stopped through the `scorsese_render::Cancel` the worker gives every job,
held in the queue by job id: the render stops within a frame, reaps its ffmpeg
children, removes its unfinished file, and the job ends `cancelled` — not
`failed`, because nothing went wrong — with how far it got as its `error`. A
graceful stop of the worker trips every flag too, so a render does not hold a
core in a process that is shutting down; the job is recovered as before.

**Claiming** is the one cross-user thing the worker does — *which job next*,
whoever's it is — so it runs `db::privileged`, `FOR UPDATE SKIP LOCKED`, and
returns the owner. Everything after it (keeping a ticket, finishing, whatever a
handler reads) runs `db::scoped` as that owner. The claim is fair between
users: whoever has the fewest jobs running goes first, so one person's batch
does not hold everybody else up.

**Concurrency is per kind**, declared beside each kind in `jobs/kinds.rs`,
**and per vendor where the vendor counts** (#1261). ElevenLabs refuses calls
past the plan's concurrent requests with a `429`, every endpoint counted
together, so every ElevenLabs kind draws on one shared budget,
`jobs::kinds::ELEVENLABS`, sized by `SCORSESE_ELEVENLABS_CONCURRENCY` (3,
Starter's), and a kind runs only while both its own limit and its budget have
room. Cloning a voice (#1263) runs as a `voice_keep` job and draws on it
too; music and sound join it when they arrive.

| kind | at once | why |
| --- | --- | --- |
| `render` | 2 | compositor and encoder each take several of the four cores |
| `preview` | 1 | a render too, but small, and superseded by the next edit — its own kind so a preview never holds a slot a finished render is waiting for |
| `proxy` | 1 | a whole transcode, as heavy as a render |
| `thumbnail` | 2 | one decoded frame |
| `veo_shot` | 4 | minutes of waiting on Google, almost no machine |
| `spoken_line` | the ElevenLabs budget | seconds, mostly network |
| `still_image` | 4 | a Gemini still (#461): seconds, mostly network |
| `batch_still` | 64 | a Gemini still in a half-price batch (#947): one order, then asking once a minute for up to a day — many at once, so a slot held for a day never makes the next person's still wait a day to be ordered |
| `voice_design` | the ElevenLabs budget | an ElevenLabs voice design (#572): seconds, mostly network |
| `voice_keep` | 1, within the ElevenLabs budget | a designed candidate kept as a voice (#572), or a voice cloned from a user's recordings (#1263): one free call, one at a time so the voice-slot count it checks first is never raced |
| `voice_forget` | 1, within the ElevenLabs budget | a kept voice deleted at ElevenLabs, then its row (#1381): one free call, freeing the voice slot it took |

**Every kind has a handler** — `thumbnail` and `proxy` (#535, #542, *Library*),
`render` and `preview` (#541, #542, *Renders*), `veo_shot`, `still_image`,
`batch_still` and `spoken_line` (#539, #461, #947, *Web MCP*, below), `voice_design`, `voice_keep`
and `voice_forget` (#572, #1381, *Designing a voice*, below). A paid generation pays through credits (*Credits*,
below: a failed one is free) and keeps what it made in the library. Each
registers its handler in `jobs::kinds::registry()`; a kind nothing registers
waits rather than failing. The worker, the claim, recovery and
the event stream are exercised end to end by test handlers, one of which stands
in for Veo.

**After a crash.** One worker per database, held by a Postgres advisory lock,
so a job found `running` when the worker starts belongs to a dead process. It
goes back to `waiting`, stamped `interrupted_at`; after three interruptions it
is given up on — `failed`, or `stuck` if it holds a ticket — except a
`batch_still` holding its batch's name (`jobs::kinds::PATIENT`, #947), which
goes back however often: it waits a day across that day's deploys, and with its
ticket it only asks, so it cannot be what took the process down. A graceful stop
(`docker compose stop`, a deploy) takes the same path, so the crash path runs
on every deploy. Who a power cut affected:

    docker compose exec server scorsese-server job interrupted

**Veo: never pay twice.** The rule `scorsese_providers::video` keeps for a
local project, kept here:

- The moment Google accepts a shot, its operation ticket is committed to the
  job's row (`Context::keep_ticket`), before the handler does anything else.
- A job that comes back with a ticket **polls it and never submits again**:
  Google keeps generating and bills either way, and the video stays fetchable
  for two days.
- A Veo job polls for up to **15 minutes** (`PROVIDER_PATIENCE`) — shots have
  taken ten — then marks the job `stuck`, ticket kept for a later collect.
- **A batched still is the same** (#947): the batch job's name is its ticket,
  kept on the job and the audit row the moment Google accepts, and a job that
  comes back with one asks and never orders again. It asks once a minute
  (`BATCH_EVERY`) for up to `BATCH_PATIENCE`, two days and an hour: Google
  answers within 24 hours and expires an unfinished batch at 48, and an expired
  batch answers as stopped, which settles it free — so `stuck` is only for a
  batch that never answers at all.

**Live state** reaches the browser over **`GET /api/events`**, server-sent
events, one stream per user carrying everything live: each message is a JSON
object with a `type` — `job`, `job_progress`, and the assistant's (*Assistant
turns*). `job_progress` (`id`, `progress`) is how far a running render or
preview has got (#698): `{percent, phase, done, of}`, sampled from the
renderer in memory at most twice a second and sent only when the percentage
or the phase moved — never a row written per frame. A job read while it runs
carries the same object as `progress`; any other job has `null`.
In memory and allowed to drop: a reader that falls behind gets `resync`, and
the answer to that, or to reconnecting, is to re-read `GET /api/jobs`. The
stream ends when the server stops, and `EventSource` reconnects by itself.

| route | who | what |
| --- | --- | --- |
| `GET /api/jobs` | a member | their last hundred jobs, newest first |
| `GET /api/jobs/{id}` | a member | one; `404` for one that is not theirs |
| `POST /api/jobs/{id}/cancel` | a member | stop one of their renders (above); `409` for a kind that is never stopped |
| `GET /api/events` | a member | their live updates, as server-sent events |

There is no route to enqueue a job directly: the feature that needs one (a
render, a generation) enqueues it inside its own transaction and announces it.
Not in v1: priority tiers, cleaning out old finished jobs.

## Projects

A user's projects in Postgres (#534). The code is `crates/server/src/projects/`,
and its module docs carry each argument; this is the whole of it in one place.

**A row is the whole `project.json`**, in a `JSONB` column, read into
`scorsese_core::Project` and written back whole. `JSONB` because key order and
whitespace are not meaning, and it lets Postgres look inside: `name` is a column
*generated* from the document, so a list never reads documents and the two
cannot disagree. A new project is `Project::new`, exactly as `scorsese new`
makes one.

**The conflict rule is a revision number.** Every write adds one; a save names
the revision its document was read at, and is refused (`409`) when that is no
longer current — the check and the write are one `UPDATE … WHERE revision = $n`,
so two racing writers cannot both land. A number rather than a fingerprint of the
bytes (which `JSONB` does not keep), and a check rather than a lock (a lock held
while an assistant thinks is a project wedged when it dies). A change that takes
no time — a rename — is made under the row's lock instead and cannot conflict.

**Media is named by hash; the path is only where it sits.** A stored document
refers to files exactly as a folder's does — a project-relative `path` and a
`sha256` — so "no absolute paths, ever" holds and `core` reads it unchanged. A
library file sits at **`assets/<sha256>.<ext>`** (`projects::media::library_path`):
unique per file, the same in every project, extension kept. Generated media keeps
its `generated/…` path.

**`project_assets` is derived, never edited**: one row per distinct `sha256` in
the document, rewritten in the same transaction as every write. It answers
"which projects use this file?" for the library (#535). It names a file by
**(user, sha256)** — the library's own identity for a file — and carries a
foreign key to `library_items (user_id, sha256)` (#535): a document naming a
file its owner does not have is refused with the assets named, and a file a
project uses cannot be deleted. A composite key to `projects (id, user_id)` stops a row
pointing at another user's project.

**Recipes, pages and the script live in `project_files`** (#560, #777): one
row per file, keyed by (project, project-relative path), holding its text —
every file under `recipes/` and `pages/`, plus the script and any recipe the document names elsewhere; never
`project.json` or anything under `assets/`, `generated/` or `cache/`. These are
the authored files a `.scor` folder keeps beside its document and cannot
rebuild. A table keyed by path rather than library items (a recipe is text
edited in place, not media addressed by content) or fields in the document (a
format change, and a 30 KB script in the file an agent opens to learn the
edit). They are **written with the document under its revision**
(`projects::save_with_files`), so a recipe edit conflicts exactly as a timeline
edit does; at most 1 MiB each, 500 files and 16 MiB to a project. Per-user like every table.

**Rendering a stored project** lays it out as a temporary `.scor` folder
(`projects::media::materialise`): the document written, its kept files written
beside it, each media file **symlinked**
from where the user's storage keeps it, looked up **by hash** in that user's
storage and never by the document's path — so a document cannot point the server
at another user's file or at the host's. `render` and `compositor` run on it
unchanged; the folder is removed when dropped. The render job (*Renders*) is
what builds one.

**A schema bump migrates every stored document on start** — the rule in
`CLAUDE.md`. After the SQL migrations and before listening, the server finds
every document whose `schema_version` is not this build's and carries it
forward with `scorsese_core::migrate`, in one transaction: either every project
is readable by this build or the server does not start, naming the project that
could not be carried. A document from a *newer* build is that case — an older
binary deployed over a newer one — and the way back is the dump taken before
updating. Local folders go through the same steps with `scorsese migrate`.

| route | who | what |
| --- | --- | --- |
| `GET /api/projects` | a member | their projects, newest write first, without documents |
| `POST /api/projects` | a member | `{name, fps?, platform?, style?, assets?}` → a new project, `201`: with the starting brief in `script.md` when a platform or style is chosen, and the library files in `assets` imported; `400` for a platform, style or file the server refuses, and nothing is kept |
| `GET /api/projects/{id}` | a member | the project, its `document` and `revision` |
| `PUT /api/projects/{id}` | a member | `{revision, document}` → `{revision}`; `409` if it moved on, `400` for a document this build does not read |
| `PATCH /api/projects/{id}` | a member | `{name}` → `{revision}` |
| `DELETE /api/projects/{id}` | a member | `204`; `404` for an id that is not theirs |
| `PUT /api/projects/{id}/start` | a member | `{platform?, style?, message?}` → `{turn, note}`: the project is made for that now, and when it changed a turn starts with `message` and a note saying so; `400` for a choice the library refuses |
| `GET /api/styles` | a member | the menu a project starts from: `{platforms, styles}`, each style with its name, description, platforms and `preview` (`null` until #1017) |

**What a project is started for** (#1016): a placement (`scorsese_core::style::Platform`)
and a kind of video (a style from the library), both optional, chosen in the
new-project dialog. They do two separate things. The **brief** —
`style::Start::brief`, the same words `scorsese new` and the local `project_new`
write — goes into the project's script, `script.md`, which is what the
assistant reads and from then on edits like any other text. The **columns**
`projects.platform` and `projects.style` are the web's memory of the choice:
what the editor shows as chosen, and the size a render or preview that asks for
none is delivered at. Neither enters the document; a platform is a render
preset and a style a prompt. Changing either later (`PUT …/start`) rewrites no
script: like a confirmed quote, it starts a turn whose server note says what
changed, with the new choice's brief, and the assistant updates the script.
Every conversation is told when its project has a script, and the system
prompt puts the script first for every project: scenes proposed, nothing
edited until the person answers. When anything is to be generated, the
proposal carries its storyboard in words as well (`guide prompts`), so the
person reads every paid shot as a paragraph before a quote ever reaches them.

Deliberately storage verbs only: what a project *says* is changed by `core`'s
editing functions, through the tools (*Web MCP*, #540) and the editor (#545).
Recipes, pages and the script are read and written through the tools too
(`synth_*`, `page_read`, `page_write`, `script_read`, `script_write`); there is
no route for them and no recipe or page editor in the front-end — the user never
sees a page's HTML (#594).

## Credits

What each user has paid in and spent, and the record of every paid generation
(#537). The code is `crates/server/src/credits/`, and its module docs carry
each argument; this is the whole of it in one place.

**The ledger is `credit_entries`, and it is append-only.** A balance is the sum
of a user's entries, in signed integer micro-dollars: money in positive, money
out negative. An entry is never edited — a correction is a new entry — and
Postgres holds that, not care: the member role has no `UPDATE` or `DELETE` on
the table, and a trigger refuses both (and `TRUNCATE`) to everyone else,
the login role included. The one exception is the delete that cascades from
deleting the account, which is how a user's rows leave.

| kind | amount | what |
| --- | --- | --- |
| `top_up` | + | credits a user bought (*Buying credits*), or money the operator recorded |
| `reservation` | − | a paid generation's price, held while the provider works |
| `release` | + | a reservation given back — the provider answered |
| `charge` | − | what a generation that worked, or an assistant call, costs |
| `refund` | + | money given back, with the reason |

**Pricing** is the provider's cost times its kind's multiplier, rounded up to
the micro-dollar, and that is all a user pays: **there is no monthly fee**
(#1243, the maintainer, 2026-10-13; the $10/month #527 set went with it, and
so did the `monthly_fee` kind, whose two rows on the hosted server were test
money and were deleted by `0028`). Veo and ElevenLabs are priced by `scorsese_providers::prices` — the same cents
the quote (#538) showed, converted at the server's boundary; the assistant by
`prices::chat` from the token counts its response reports, which is exact.

**Spending a generation is reserve, then settle.** `credits::generations::start`
prices it, takes a lock on the user's own row (so two spends at once cannot
both see the last dollar), and reserves — or refuses, writing nothing, when the
balance cannot cover it. When the provider answers, `finish` releases the
reservation and, if the generation **worked**, charges it — liked or not. A
generation the provider **failed** nets to zero: free. A unique index allows
one release per reservation, so nothing is settled twice. A Veo or batch job that
goes `stuck` settles nothing and its reservation stays held, since Google may
still be billing. A batched still reserves its half price, charged when its
picture lands and released when its batch stops without one. The assistant is charged after each call rather than reserved for:
its cost exists only once the tokens are counted, so a turn checks the balance
is positive before starting and the call may dip a little below zero.

**Audit tables.** `veo_generations` (model, resolution, seconds, aspect,
prompt, brief hash, operation ticket, state, estimated cost, error),
`image_generations` (model, resolution, aspect, how many references, prompt,
brief hash, state, estimated cost, error — #461; and since #947 `batch` and
the batch job's name as `ticket`, which only a batched still has: one drawn now
comes back on the call), `speech_generations` (model, voice, text, characters,
settings, estimated cost, error) and `voice_designs` (description, passage,
characters, seed, guidance, brief hash, state, the three candidates once it
worked, estimated cost, error — #572, *Web MCP*) hold one row per paid generation, bound to the ledger entries that paid
for it. Each carries a nullable `tool_call_id`, keyed to the tool call that
asked for it by (call, owner) (#540; `0007_tool_calls.sql` added the keys
`0004` left out), a nullable `library_item_id` keyed to the item it
made by (item, owner) — kept, set null, when the item is deleted (#535) — and a
`project_id` that deliberately has none: a project can be deleted, and the
record of what it cost cannot.

**The multipliers** (`credits::Product::percent`, the one table; #1247, the
maintainer, 2026-10-13):

| kind | multiplier |
| --- | --- |
| Veo shot | ×1.5 |
| still (drawn now or batched), spoken line, music track, voice design, assistant call | ×1.8 |

The principle: charge in line with what scorsese actually spends, which is
third-party APIs, and make it profitable. Electricity, time, the machine and
its disk are deliberately not priced, and the user sees each generation's price
in dollars of credit, never a percentage. Every dollar of API spend passes
three costs that scale with it, whatever the vendor:

- **paying the vendor**: a Brazilian card pays the commercial rate × 1.035 IOF
  × ~1.04 card spread, so $1 costs ~R$ 5.37 at R$ 4.99/USD: every 100 of API
  cost at the commercial rate is 107.6 actually paid;
- **tax**: Simples Nacional on gross revenue, 6% (Anexo III, first bracket) or
  15.5% (Anexo V) — which one is not known yet;
- **taking the payment**: Stripe card 3.99% + R$ 0.39, ≈4.8% on R$ 50 (#548).

Profit per R$ 100 a user spends is `100 × (1 − tax − fee) − 107.6 / multiplier`:

| multiplier | tax 6%, card | tax 15.5%, card |
| --- | --- | --- |
| ×1.1 (until #1247) | −8.6 | −18.1 |
| ×1.5 | 17.4 | 8.0 |
| ×1.8 | 29.4 | 19.9 |

A thinner margin on video was accepted, since it is the most expensive thing a
user buys. These are conservative defaults until the tax rate is known.
**Expected to change: re-derive when the tax rate, the card spread or a
vendor's price changes.** Local CLI and MCP quotes are untouched: they are the
providers' own prices (`docs/prices.md`), since there the key is the user's.

A quote can hold a shot, a line and a still at once, so what it takes from the
balance is the sum of each item at its own multiplier — exactly what the
reservations will be — and the `quotes` table keeps it in `micros` beside the
providers' `cents`, for the assistant's confirmation box to show.

**The user's own history** is the same ledger, scoped and written for them: one
row per thing that moved the balance — a reservation and what settled it fold
into one row, *charged*, *free: the provider failed*, or *pending* — each with
the balance after it, the project's name while the project exists, and what set
its price. Filterable by project, kind and UTC days, paged, with a total over
everything the filter matched. It is also described as a read-only tool,
`spending_history` (`credits::tool`), which web MCP (#539) registers: the
`scorsese-mcp` registry is stateless and database-free by rule, and this tool
is nothing but a user's rows in Postgres.

| route | who | what |
| --- | --- | --- |
| `GET /api/credits` | a member | their balance, in micro-dollars |
| `GET /api/credits/history` | a member | `?project=&kind=&since=&until=&before=&limit=` |

The operator's side, where the server runs:

```text
scorsese-server credit top-up ana@example.com --dollars 20
scorsese-server credit refund ana@example.com --dollars 1.06 --reason "…"
scorsese-server credit balance ana@example.com
```

A top-up records the dollars credited and nothing else: whatever currency the
money arrived in, the ledger tracks dollars (#703). One a user bought links to
its payment (`payment_id`), which holds the reais and the rate.

The Veo and ElevenLabs job handlers that call `start` and `finish` are web
MCP's (*Web MCP*, below), which also registers `spending_history`. Not here
yet: the refund policy's text (#547).

## Buying credits

Users buy credits themselves (#548), in reais, through **Stripe's hosted
Checkout**. The code is `crates/server/src/payments/` and, for the calls to
Stripe and the rate sources, `crates/providers/src/payments/` and
`crates/providers/src/fx/`; their module docs carry each argument.

**The flow.** Beside the balance, *Add credits* takes an amount in reais and
shows, live, what it buys: "R$ 50,00 → $10,02 em créditos". Paying sends the
browser to Stripe's page; Stripe sends it back to `/credits/return`, which
shows the result once the balance moves. Customers are assumed Brazilian:
every charge is in **BRL**, from **R$ 20** (so the card's fixed R$ 0,39 stays
small) to **R$ 5.000** a purchase (which bounds what a stolen card can do,
and fits under Pix-on-Stripe's 3,000 USD a payment).
No payment method is named in code: the session offers what Stripe's
Dashboard has switched on — cards now, and Pix once Stripe's invitation for
it arrives, with no change here.

**The rate is kept, not fetched per purchase** (the maintainer, 2026-10-14):
the server reads AwesomeAPI's commercial USD-BRL `ask`, else the Central
Bank's PTAX selling rate from the last week, both with no key, **every two
minutes** in the background (`payments::refresh`), and keeps the latest in
memory with when it was read; a failed read keeps the last good one and is
logged, and a server just started reads one before its first quote. A quote
and a purchase use the rate kept **while it is at most ten minutes old**; past
that, or with none read, they are refused with `503` and **no session is
created**: a rate is never guessed. A rate outside 2 to 8 reais a dollar
(inclusive, on the exact decimal: 8.01 is out) is treated as no answer: a real
rate has sat near 5 for years, and a wider band only lets a garbage figure sell
credits at a wild price. **A purchase names the rate it was shown**, and is
refused with `409` and the quote at the rate now when it has moved, so the
rate locked on the payment is always the one the person saw.
**Credits are the reais divided by that rate, floored** to the micro-dollar,
and Stripe's fee is absorbed rather than added. The session expires after 31
minutes (Stripe's least is 30, measured on its clock). AwesomeAPI's keyless
quota is small — a cloud machine was answered `429` the day this was written
— which is what the fallback is for.

**`payments` is the record**: one row per Checkout Session — its owner, the
session id (unique), the reais in centavos, the credits in micro-dollars, the
rate **as its source sent it** and which source, the state (`open`, `paid`,
`expired`) and when it was paid. Per-user like every table. The `top_up` it
causes carries its `payment_id`.

**Crediting is exactly once, keyed on the session id**, with two ways in that
both call `payments::settle::settle`:

- **Stripe's webhook**, `POST /api/payments/stripe/webhook`, public, believed
  only when the `Stripe-Signature` HMAC-SHA256 over the raw body matches
  `STRIPE_WEBHOOK_SECRET` within five minutes. It acts on
  `checkout.session.completed` and `checkout.session.async_payment_succeeded`
  (credited when the session says `paid`, in BRL, for exactly the centavos
  asked) and `checkout.session.expired`; anything else signed is answered
  `200` and ignored.
- **The return page**, whose `POST /api/payments/{session}/confirm`
  retrieves the session from Stripe while the payment is still open — which
  covers a webhook that is late or never comes.

`settle` finds the payment's owner by session id (a `privileged` read: a
webhook carries no user), then works scoped as them under the payment row's
lock, so the second of two racing calls finds it `paid`. A unique index on
`credit_entries (payment_id)` refuses a second top-up for one payment even if
everything above it went wrong. An amount or currency that disagrees with what
was asked credits nothing and is logged for the operator to look at in Stripe.

| route | who | what |
| --- | --- | --- |
| `GET /api/payments/quote` | a member | `?centavos=`: the credits they buy, the rate and its source, and the limits |
| `POST /api/payments` | a member | `{"centavos", "brl_per_usd"}`: a Checkout Session at the rate shown — its `url` to send the browser to, and its `session`; `409` with `quote` when the rate moved |
| `POST /api/payments/{session}/confirm` | a member | their payment by session, credited first if Stripe says it is paid |
| `POST /api/payments/stripe/webhook` | Stripe, signed | how a session went |

**Configuration**: `STRIPE_SECRET_KEY` and `STRIPE_WEBHOOK_SECRET` beside
`SCORSESE_PUBLIC_ORIGIN`, in `deploy/.env` (the table under *`crates/server`*).
The webhook endpoint at `<origin>/api/payments/stripe/webhook`, listening for
the three events above, is Terraform's (`deploy/terraform/stripe`, #1256):
`make infra-apply` makes it and `deploy/terraform/tf stripe secret` writes its
signing secret into `deploy/.env` (*Infrastructure, as code*). The code runs
on **test keys** (`sk_test_…`); switching to live keys waits for #547 (terms,
refund and privacy pages) and is a change to `deploy/.env`, not to code — but a
live key is a different endpoint at Stripe, with its own secret, so the apply
and `tf stripe secret` run again after the switch.

**Not here**: refunds stay by hand — the Stripe Dashboard, and
`scorsese-server credit refund` — until #547 sets the policy. Foreign customers
and other currencies wait for demand.

## Library

A user's files, reusable in any of their projects (#535). The code is
`crates/server/src/library/`, and its module docs carry each argument; this is
the whole of it in one place.

**Stored once per (user, SHA-256).** A file sits at
`$SCORSESE_STORAGE/users/<id>/library/<sha256>.<ext>` and appears in a stored
project at `assets/<sha256>.<ext>` — the hash and the extension are the whole
of where it is. A byte-identical second upload is refused (`409`) with "you
already have this as *X*" and the item's id, whatever its name. The browser
hashes a file first and asks `GET /api/library?sha256=`, so a duplicate never
crosses the network; the refusal at upload is the backstop.

**The kinds `scorsese import` takes** — video, image, audio and an `.html`
page, by the same extension list (`scorsese_core::pool::infer_kind`). Any
other file, a `.mid` included (#964), is refused at announcement (`415`)
before a byte is sent. **A page (#1144) is text, not media**: never probed,
held to UTF-8 and to the size a project keeps a page at (1 MiB, refused at
announcement when the length says so), given no thumbnail (`404` with
`pending: false`), and served as `text/plain`, never run from the app's
origin. `import` **copies** it into the project's `pages/` as an `html`
asset, the way `scorsese import` does locally, so `page_write` edits the
project's copy and never the library's. On
arrival the server **hashes the bytes itself**
and refuses (`422`) a file whose hash is not the one announced, then probes it
and holds it to its kind with `pool::measure`, exactly as import does — a
`.mp4` with no picture is refused, and the prober's words (which name server
paths) go to the log, not the user.

**Uploads are tus 1.0.0**, the subset Uppy speaks: creation (`POST`),
`HEAD`, `PATCH`, termination (`DELETE`). Written in `http/uploads.rs` rather
than taken from a crate: the Rust tus servers are applications, not libraries a
router mounts, and the subset is a page. `Upload-Metadata` carries `filename`
and `sha256`. Chunks stay under Cloudflare's 100 MB request cap (the client's
`chunkSize`); how far an upload got is the length of its file in the cache, so
a dropped chunk resumes from whatever reached the disk. One writer per upload
(`423` to a second); an upload untouched for a day is swept by the user's next
announcement. The last `PATCH` names the new item in `Scorsese-Library-Item`.
A tus client retries `409` by default — right for a misplaced chunk, wrong for
"you already have this", whose body carries `item`; the web app's
`onShouldRetry` stops on it.

**Thumbnails are a job** — the first handler the queue runs — drawn by
`scorsese-render` (`frames::thumbnail`: a video's frame one second in, a
picture scaled down, a sound's waveform; at most 320 px a side) into
`$SCORSESE_CACHE/users/<id>/thumbnails/`. Queued with the item, in the same
transaction; queued again when one is asked for and missing, so clearing the
cache costs nothing but time. A list carries each item's thumbnail URL, which
answers `404 {"pending": true}` until it is drawn.

**Proxies are a job too** (#542): a small copy of a heavy video — its short
side brought down to 540 px, the same frames at the same times, H.264 with a
keyframe every fifteen — which a **preview** decodes instead of the original
(*Renders*, *The editor*). Made by `scorsese-render` (`preview::make`) into
`$SCORSESE_CACHE/users/<id>/proxies/`. Only a video worth one gets one: larger
than a proxy, and known to be opaque (H.264 has no alpha, and a preview that
lost an overlay's transparency would lie about more than resolution). Queued
**on arrival**, with the thumbnail, so the first preview a file appears in is
already a fast one; queued again by a preview that finds one missing, so
clearing the cache costs only time. Removed with the item, and otherwise kept:
they are a small fraction of the library they stand in for, and the render
quota is about renders. **A finished render never reads one** — nothing but a
preview is handed proxies at all, and a test holds it with a proxy of another
colour sitting in the cache.

**Opening a file streams it in ranges**: `Range: bytes=…` gets `206`, so a
video plays and seeks without being fetched whole. A file never changes under
its id, so it is sent `private, immutable`, tagged by its hash.

**A file a project or a template uses cannot be deleted**: `409` naming the
projects and the templates (*Templates*), and the foreign keys from
`project_assets` and `template_assets` hold it if two requests race. Removing
an asset from a project, locally or here, is `asset_remove` (#396, *The web
editor*); that takes it out of the document and leaves the library file, which
this rule alone deletes.

**Generated output is an item too**, carrying the hash of its brief.
`Library::find_generated` finds it again for the **same user** — scoped, so
never across users — and `Library::keep_generated` keeps what a generation
made. Its details carry the generation record (#537): the brief, model,
settings, when, and what it cost — `estimated_cost_micros` by scorsese's own
table, `charged_micros` from the ledger. The generation jobs that call these are web
MCP's (*Web MCP*).

**A spoken line keeps its word timings on its row** (#886): the `words`
column, the document `scorsese_core::words::Words` serialises to, and `NULL`
for everything else — uploads, shots, stills, and any line kept before it
existed (never backfilled: that would mean paying for the line again). Laying
a project out writes them beside the audio as `<audio>.words.json`, where a
local project keeps them, so a render's pages get `scorsese.words` and
`project_describe` names the word being said, as they do locally.

| route | who | what |
| --- | --- | --- |
| `GET /api/library` | a member | their files, newest first: `id`, `name`, `kind`, `size_bytes`, `thumbnail`; `?kind=`, `?search=`, `?sha256=` and `?project=` (the files that project uses) narrow it |
| `GET /api/library/{id}` | a member | everything known, `used_by` (projects), `templates`, `generation` (or `null`), `source` — where a stock file came from (or `null`, #1237) |
| `PATCH /api/library/{id}` | a member | `{name?, description?}`; an empty description removes it |
| `DELETE /api/library/{id}` | a member | `204`; `409` with `projects` and `templates` when one uses it |
| `GET /api/library/{id}/file` | a member | the file, whole or in the range asked for; `?download=1` adds `Content-Disposition: attachment` with its name and real extension (#711) |
| `GET /api/library/{id}/thumbnail` | a member | the picture, or `404` while it is drawn |
| `OPTIONS /api/uploads` | anyone | what tus this server speaks |
| `POST /api/uploads` | a member | announce: `Upload-Length`, `Upload-Metadata` → `201`, `Location` |
| `HEAD /api/uploads/{id}` | a member | `Upload-Offset`, `Upload-Length` |
| `PATCH /api/uploads/{id}` | a member | the next chunk at `Upload-Offset`; the last admits the file |
| `DELETE /api/uploads/{id}` | a member | abandon it |

Not in v1: folders (#527), sharing across users, a quota per user.

## Renders

A user's finished video, made from a stored project by the job queue and kept
for download (#541). The code is `crates/server/src/renders/`, and its module
docs carry each argument; this is the whole of it in one place.

**One render per (document, settings).** A render is keyed by a SHA-256 of the
project's document and the render settings, so asking again for an unchanged
project in the same shape answers with the file already made, at once. The
settings are exactly what `docs/output-formats.md` allows — a container, its
codecs and, for a picture, a resolution — built by the constructor the CLI and
MCP use, so every refusal reads the same. The frame rate is the project's own
and the whole timeline is rendered. The job carries the document as it was when
the render was asked for, so an edit made while it waits is not rendered under
the old key.

**The job** lays the project out as a `.scor` folder (*Projects*, the
materialiser), each file linked by hash from the owner's library, renders it
with `scorsese-render` exactly as `scorsese render` renders a folder, and moves
the finished file into the cache only when it is complete. A render is not a
paid provider call: it costs no credits. **A render never bakes**, exactly as
`scorsese render` does not: a `synth_audio` clip renders from its bake, which
`synth_bake` keeps in the library (*Web MCP*). One whose bake is not there
fails the job naming the asset and its recipe, rather than delivering a video
that silently lost its music.

**Where they live:** `$SCORSESE_CACHE/users/<user>/renders/<project>/<key>.<ext>`
— the cache, never the library, because a render can always be made again.
The `renders` table (owner, project, key, settings, path relative to the cache,
size, created, **last used**) is per-user like every other.

**Eviction is the maintainer's rule, deliberately simple.** The operator sets
`SCORSESE_RENDER_QUOTA`. When a new render would take the cache past it,
**every render not used in the last 48 hours is deleted** — anyone's, which
makes it one of the few privileged steps. A download counts as use, and so does
asking for a render that is already there. If nothing is that old, the new
render is kept anyway and a warning is logged: **the quota is a target, not a
wall.** A **weekly sweep** applies the same rule whatever the pressure, and
removes files no row names any more (a deleted project's or account's); its
clock is a file's age in the cache, so restarts do not reset it. Deleting a
render is always safe — it is rebuilt from the stored project on the next ask —
which is why the rule can be this simple, and why clearing `cache/` by hand is
safe too: a row whose file is gone is forgotten and the render made again.

**Never deleted mid-download**, by two things that hold because there is one
server process: a download stamps the row as used, committed, *before* it opens
the file, and eviction's `DELETE` re-checks the 48 hours against that stamp; and
every open file is **pinned** — an in-process count per path, released when the
response body is dropped — which eviction and the sweep skip. The pin covers
what the stamp cannot: a download still going 48 hours on, or a project deleted
while its render streams.

| route | who | what |
| --- | --- | --- |
| `POST /api/projects/{id}/renders` | a member | `{container?, video_codec?, audio_codec?, resolution?, platform?, fps?, range?, narration_bands?, loudness?}`, spelled and checked as MCP `render`'s arguments (`scorsese_render::RenderAsk`, #1070) → `200 {render}` when kept, `202 {job}` when queued or already on its way; `400` for a shape `docs/output-formats.md` does not allow, a picture setting (`resolution`, `platform`, `fps`) on sound only, or `resolution` beside `platform`. No `resolution` or `platform` is the project's frame with 1080 lines on its shorter side (`Resolution::of_project`); no `fps` is the project's own rate; no `range` is the whole timeline |
| `GET /api/projects/{id}/renders` | a member | the project's kept renders, most recently used first |
| `GET /api/renders/{id}/file` | a member | the file as an attachment, in HTTP ranges; counts as use |

The job's state arrives on `GET /api/events` like any job's, and how far it
has got as `job_progress` while it runs — shown as a bar with the phase beside
it, since the percentage counts frames and stands at 0 while the sound is
mixed and at 99 while the file is finished. Its result names the render and
where to download it, and its `notes` say how loud the delivered soundtrack
came out — before and after, when a `loudness` target was asked for (#990) —
and anything the render could not draw.

**Previews are renders, marked** (#542). `POST /api/projects/{id}/previews`
`{resolution?, quality?}` asks for the cut at a **preview quality** — `full`,
`half` (the default) or `quarter` of the delivery size named, 1920x1080 unless
said — as an mp4, answered exactly as a render is (`200 {render}` / `202
{job}`). The settings carry the quality (`preview`), so a preview is keyed
apart from a finished render of the same size; a finished render's settings
have no such field, so its key is what it was before previews existed. Four
things differ, all in `renders::preview`:

- **Its own kind**, `preview`, one at a time, so previews never hold a slot a
  finished render is waiting for.
- **Proxies**: at half or a quarter, each heavy video's proxy is decoded where
  one is made (*Library*), and any that are missing are queued. Full reads
  originals, as a finished render always does.
- **Superseded, then stopped.** When a preview's turn comes it checks the
  project still hashes to its key; if the project has moved on it finishes at
  once, `{"superseded": true}`, having rendered nothing — a waiting job retires
  itself, with no race with the claim. One already *running* is stopped when
  the next preview of the project is asked for (#660), and ends `cancelled`.
  So edits faster than renders cost a frame of an old render at most, and the
  editor asks again for the revision it is on.
- **One kept per shape.** A finished preview replaces the project's earlier
  previews at the same settings, which are of documents it has moved past —
  except a file somebody has open. They are left out of the project's list of
  renders, since nobody downloads one, and otherwise live by the render cache's
  quota and 48-hour rule like any render.

| route | who | what |
| --- | --- | --- |
| `POST /api/projects/{id}/previews` | a member | `{resolution?, quality?}` → as `POST …/renders`; `400` for a quality or size that is not one |

## Page captures

An `html` clip is a web page drawn frame by frame (`docs/pages.md`). On the web
that page is **somebody else's code on the maintainer's machine**, so it never
runs where the server does (#594 decision 1, #778). The code is
`crates/server/src/captures/`, whose module doc carries the argument.

**One container per capture** (#852). Each capture runs the server's image as
`scorsese-server capture-one`, with no network at all, no capability,
`no-new-privileges`, a read-only root, a non-root user, and **Chromium's own
sandbox on**. The sandbox needs exactly one grant, the seccomp rule in
`deploy/capture/seccomp.json` (Docker's default profile plus
`clone`/`unshare`/`chroot`, so a renderer can get a user namespace of its own);
#773 measured that and nothing more. There is no `--no-sandbox` and no switch
for one. The caps are #773's: 2 CPUs, 1 GiB of memory with no swap, 256
processes. The image bakes the pinned `chrome-headless-shell`
(`tools/chromium/pin`) and `SCORSESE_CHROME` names it; the server never
downloads one.

**What a capture can see** is its own job and nothing else: the job's folder,
read-only; the media files its project links to, one by one, read-only; and its
project's page cache, the one place it may write. Each is mounted at the path
the server laid it out with, so the project's links resolve inside. Another
user's library is not filtered out, it is simply not there. Every linked file
must also be the job's owner's (`library/users/<owner>/`), so even a server bug
cannot hand one user's file to another's page; a job that links to anything
else is refused, and the render's notes say why. The capture is told that
folder too (`capture-one --follow`, #857): a page's request for its own
`../assets/photo.png` follows the link there, while a link leading anywhere
else (`/etc/passwd`, or on from the library) is refused as it is locally.

**Who starts them: `capture-launcher`.** Starting a container takes the Docker
socket, which is root on the host, and the server is the one container the
internet talks to. So the socket goes to a sidecar that holds nothing else: no
network, no database, no secret, no capability, the server's binary and a
Docker client (`deploy/server.Dockerfile`'s `launcher` target). The `docker
run` it makes is written out in `crates/server/src/captures/launch`, flag for
flag. **A job gives it nothing but its folder**: no image, no flag, no path it
does not check. So a server that has been taken over can ask for captures, and
still cannot start anything else. It joins the socket's group,
`SCORSESE_DOCKER_GID` in `deploy/.env`. When it starts it removes any capture
container a previous run left behind, and refuses to run without the image or
the seccomp profile.

`capture-worker` is the same loop without containers, each capture a child
process: for a machine running the server without Docker, and for the tests.
Its `--library` names the library media is linked from, so pages can load
their media there too. `--no-sandbox` exists there for CI and development machines that run as root;
compose runs neither.

**How a render reaches it: a spool.** A render job whose project shows a page
lays the project out under `$SCORSESE_CACHE/captures/jobs/job-<id>/`, writes
`ask.json`, and waits for `answer.json`. The launcher takes one capture at a
time, which is the captures' concurrency limit: one capture keeps ~3.6 cores
busy. The frames land in `captures/pages/<user>/<project>/`, the project's own
page cache, so a second render of an unchanged page reuses them and one user's
captures are never another's. The render then draws pages only from there and
never starts a browser itself.

**A still asks the same way** (#1434). `still` on the web, and the editor's
frame preview through it, draws without a browser too: before drawing, the
server works out which frames of which pages the asked instants show, reads
whatever the project's page cache already holds, and asks for the rest as
`captures/jobs/still-<call>/`, a copy of the laid-out project, with one
capture per page for the missing stretches of it that overlap or touch (a
five-cell sheet of one page is one launch, not five; what the cache holds is
taken out first, so it never widens a stretch). The launcher answers jobs and stills in
the order they were asked. A still waits as long as the launcher would give
those captures (60 s plus a second a frame) but never more than 90 s, well
inside nginx's 300 s; one still queued behind a render's pages after that is
drawn with the page as *being captured*, and the capture finishes into the
cache for the next look — left to the launcher for at most an hour. A still
stopped (the assistant's Stop, or web MCP's `notifications/cancelled`) stops
waiting at once, its capture left to finish the same way; one stopped before
it asked copies and launches nothing. A launcher that is not running is said at once, since nobody
waits on a still for 30 s to learn it.

**The walls, and what a page that hits one gets.** A request to the internet,
a path out of the project, a WebSocket or WebRTC connection: refused, and the
render's notes say so. A page that stops answering is stopped by the capture's
own patience (60 s per answer); one that keeps answering slowly is killed with
its container at **60 s plus a second a frame**. A page that eats
memory is killed at the cap. In every case the clip shows its slug card, and
the job's result carries `notes`, the render's notes with the reason a page was
not captured among them. A failed capture is never a failed render, so the
notes are the only place it is said. If the launcher is not running at all,
the render waits 30 s for a sign of it and then draws the cards, saying so.

**Disk.** About 1.7 GB per minute of 1080p page clip, so page captures live
under the renders' rule (#849, `crates/server/src/renders/evict/pages.rs`):
they **count against `SCORSESE_RENDER_QUOTA`** with the renders, one quota for
everything rebuildable, and on pressure or the weekly sweep every capture **not
used in 48 hours** goes. A capture is used when it is made and again whenever a
render reuses it (the server stamps its frames). A project's folder is never
touched while a render job holds it, and the sweep also removes the folder of a
project or account that no longer exists, and any `frames.<pid>.partial.mkv`
a capture killed at its deadline left. Clearing `captures/pages/` by hand is
still safe: the next render captures again.

## Web MCP

scorsese's tools served over HTTP (#539), so a user can point their own Claude,
Gemini CLI or any MCP client at the web app and edit their projects. The code
is `crates/server/src/tools/` (the tool surface, per user) and
`crates/server/src/http/mcp.rs` (the transport); their module docs carry each
argument. The built-in assistant (#540) calls the same surface in-process —
`Toolbox::call`, with `Client::Assistant` — so there is one tool set and two
ways in.

**The endpoint is `POST /api/mcp`**, MCP's Streamable HTTP transport:
JSON-RPC in, `application/json` out, a batch answered with a batch, a body of
notifications with `202`. No server-initiated stream and no sessions — `GET`
and `DELETE` are `405` — because no tool reports progress mid-call (long work
is a job) and every call names its project, exactly as over stdio. What a
message *means* is `scorsese_mcp::protocol`, the code the stdio server answers
with, so the handshake and every refusal read the same either way.

| route | who | what |
| --- | --- | --- |
| `POST /api/mcp` | a member, by token | JSON-RPC in, JSON-RPC out; a batch answered with a batch |
| `GET /api/mcp` | anyone | `405`: no server-initiated stream |
| `DELETE /api/mcp` | anyone | `405`: no sessions to end |

**Stopping a call: `notifications/cancelled`.** Over this transport the
cancel arrives in a `POST` of its own while the call's is still waiting, so
the calls in flight are kept per signed-in user, not per connection: a cancel
trips the `Cancel` of that user's call with that request id (`1` and `"1"`
differ), and nobody else's. The tool gets it through `Tool::call_cancellable`,
as over stdio (#647); a cancelled call is not answered, as the specification
asks, and one whose client hung up mid-call is cancelled the same way.

**Who: an API token, only.** `Authorization: Bearer scor_…` (*Accounts*); a
browser session is `403` here, which is also what keeps a web page from
borrowing somebody's login to reach it. Every call acts on the token's user
alone — their projects, their library, their credits, their jobs.

Pointing Claude Code at it:

    claude mcp add --transport http scorsese https://<the site>/api/mcp \
      --header "Authorization: Bearer scor_…"

Other clients take the same URL and header. Locally, with the stack up, the
URL is `http://127.0.0.1:8088/api/mcp`.

**One registry, a project id instead of a path.** A stored project is laid
out as a `.scor` folder for the length of one call — the document, every file
linked by hash from the user's own library, every generation of its current
briefs linked where the brief lands, its recipes and script written in — the
registry's tool runs on it unchanged, and the document and those files are
saved back together with its revision check (retried on
a conflict, since a tool is a function of the document). The folder's path
never reaches the client. Each registry tool keeps the registry's own
description, word for word; only `project` changes, to **the id** of one of the
caller's projects. Every tool and argument is described, held by
`tests/mcp/described.rs` as `docs/mcp.md` holds the registry, and every
registry tool is decided about in `tools/surface/` — one name a line, the
lists below generated from it by `make mcp-table` — so a new one cannot reach
the web — or be left off it — without a reason written down.

| served | how |
| --- | --- |
| the *document tools*, listed below | as they are, on the stored project |
| the *project-file tools*, listed below | as they are; the script, pages and recipes they read and write are the project's `project_files` (*Projects*). A script or recipe written under `assets/`, `generated/` or `cache/` is refused whole, since nothing there is kept |
| `synth_bake` | without `out`; each new bake is **kept in the library** as a generation, its address (recipe and synthesiser) as its brief hash, before the document naming it is saved — so it renders, and is linked into every later layout by hash. A partial bake's file is gone with the folder; its report is in the reply |
| `stock_search`, `stock_import` | as they are, with the folder's `cache/stock/` linked to the user's own stock cache under `SCORSESE_CACHE/users/<id>/stock`, so Pixabay results stay cached 24 hours as Pixabay asks and an id a search returned is found again on the next call (#900). Each file `stock_import` downloads is **kept in the library** like an upload, before the document naming it is saved (`tools/fetched.rs`). A Lottie (`kind: lottie`, #903) is not media: its JSON lands in `pages/` and is kept with the project's files, up to their 1 MB cap, with nothing on the server knowing the difference. Free: the operator's `PIXABAY_API_KEY`, shared by every user's 100 requests a minute; LottieFiles needs no key; no credits |
| `look`, `hear` | their file arguments must be paths inside the project (`assets/…`, `generated/…`) — locally they may name anything on the machine, and here the machine is everybody's |
| `still` | without `out`: nothing is kept on the server's disk; the picture is in the reply |
| `project_list`, `project_new` | the server's own: a project is a row, named by an id the client asks for |
| `library`, `import` | the server's own: files come from the user's library by id (`core`'s `reference_asset`, the document half of an import), never from a path on the server — so an image sequence's stills come in one library file each, and `sequence` makes them one |
| `render`, `jobs` | the server's own: a render is a job (*Renders*), downloaded from `/api/renders/{id}/file` with the same token; `jobs` says where any job is, and with `cancel` stops a render — locally a client stops one by cancelling the `render` call, which here has already answered |
| `generate` | the server's own: paid from credits, made by the queue — below |
| `voice_design` | the server's own: paid from credits, made by the queue, the samples kept in the library and the voices in the user's own record — *Designing a voice*, below |
| `voice_clone` | the server's own: free, made by the queue from the user's uploaded recordings, the voice in the same record as a designed one — *Cloning a voice*, below |
| `spending_history` | the ledger, read for the caller (*Credits*) |
| `template_list`, `template_save`, `template_insert` | the server's own: a user's templates are rows (*Templates*) |

**The document tools**, served as they are on the stored project:

<!-- BEGIN STORED. Generated from `tools/surface` by `make mcp-table`; edit the list there, not here. -->

- `project_read`
- `project_describe`
- `project_check`
- `project_assets`
- `project_reframe`
- `project_write`
- `track_new`
- `track_move`
- `asset_set`
- `sequence`
- `asset_remove`
- `track_remove`
- `place_clip`
- `clip_set`
- `clip_animate`
- `clip_follow`
- `clip_move`
- `clip_remove`
- `clip_group`
- `clip_ungroup`
- `dissolve`
- `duck_music`
- `caption_narration`
- `set_volume`
- `scale_pacing`
- `cut_to_voice`
- `icons`
- `guide`
- `voices`
- `stock_search`
- `stock_import`

<!-- END STORED -->

**The project-file tools**, served as they are on the project's script and
recipes:

<!-- BEGIN PROJECT_FILES. Generated from `tools/surface` by `make mcp-table`; edit the list there, not here. -->

- `script_read`
- `script_write`
- `page_write`
- `page_read`
- `synth_new`
- `synth_kit`
- `synth_read`
- `synth_write`
- `synth_set`
- `synth_check`
- `vectorize`

<!-- END PROJECT_FILES -->

**Why the registry did not move.** The tools that need the database —
`spending_history`, `project_list`, a `generate` that pays through credits —
have no meaning for a `.scor` folder at all, so moving the registry beneath both
crates would put Postgres-shaped tools in a crate the stdio binary links and
could never call. The web surface is the registry's tools plus the server's own,
under one set of rules, and `scorsese-mcp` still never learns about users
(`crates/server/src/tools/mod.rs` has the argument).

**Paying: quote, credits, a job, the library.** `generate` quotes with
`scorsese_providers::quote::generation` over the laid-out project — so a brief
the user already generated, in this project or another of theirs, is priced at
nothing, and so is one whose job is still on its way. Those are left out of
the quote's lines, so the confirmation box and the model's reply list only
what costs money; a call with nothing to pay holds no token, shows no box, and
says so in one line with how many were already made (#1436). A call without `confirm`
answers with the quote, what it takes from the balance (*Credits*, *Pricing*) and a
token, kept in the `quotes` table under the providers' rules (#538: bound to
what was quoted, once, fifteen minutes). A call with the token reserves every
charged brief and queues it as a `veo_shot`, `still_image`, `batch_still` or
`spoken_line` job in one transaction — all, or none when the balance cannot cover the lot — and answers
at once with the job ids. Each job (`crate::generations`) sends the brief
exactly as quoted, keeps a shot's ticket before anything else, keeps what came
back with `Library::keep_generated`, charges it — or releases it, free, when the
provider refused — and brings it into the project as it is by then, measured.
The provider keys are the server's: `GEMINI_API_KEY` and `ELEVENLABS_API_KEY`,
through the one credentials resolver; a missing key fails the job, free.

**Stills in a half-price batch (#947).** The web does what the CLI and local
MCP do (#894, `docs/prices.md`): `generate` with `batch` quotes the stills with
`quote::batch` — stills only, refused naming any shot or line that would be
sent too — under its own `Spend::Batch`, and the yes queues each still as a
`batch_still` job, one still per batch (the payload is one brief; the half price
is per request, so grouping can come later). **The offer is a second token.** A
quote for now whose stills come to `quote::OFFER_FROM_CENTS` or more, with
nothing but stills to pay for, also quotes them as a batch and issues that its
own token under the same tool call; the reply says both prices in one line and
names only the first token. The assistant's confirmation box shows both —
*now: $X · within 24 hours: $Y* — as two yeses, neither the default; the yes
picked spends its own token (`POST /api/chat/turns/{id}/quote` with `batch`),
and the other is withdrawn. The model never holds either. A still a project
brings in **already queued in a batch ordered elsewhere** — a local `.scor`
folder's — cannot be collected here (the batch is that key's), so its quote
line says so plainly; a batch this server orders is a job, never an
`operation` in the document.

**Designing a voice (#572).** `voice_design` designs, keeps and lists as the
stdio tool does, and pays like `generate`: a call without `confirm` answers with
the estimate (`voices::design::estimate`, billed once for the passage, three
candidates) times 1.8 and a token; a call with the token reserves it and queues
a `voice_design` job in one transaction. A confirming call need not repeat the
brief — it is read from the call that issued the token, which is how the
assistant's confirmation box, sending only the token, designs what was quoted.
Locally a design writes its samples and a `design.json` into `generated/` and a
kept voice into `designed-voices.json`, beside `project.json`; here those have
homes that outlive a call (`crate::designs`):

- **the samples are library items**, each kept under a hash of its own (the
  design's brief hash and its place among the three, since a library holds one
  item per brief hash) — playable in the library, importable into any project;
- **the design is a row of `voice_designs`**: its audit row as a paid
  generation, with `credit_entries.voice_design_id` linking the ledger to it,
  and once it worked its three candidates. A design the user already has —
  every sample still in their library — or one whose job is on its way is
  answered for nothing, with no token; an unchanged brief is never paid twice
  across the user's projects, and never shared between users;
- **kept voices are rows of `designed_voices`**, per user. A voice lives in the
  operator's ElevenLabs account and any project may name it, so it belongs to
  the user, not a project; each row points at the design holding the
  description and seed that made it;
- **a kept voice is its maker's alone** (#687). Every user's voices sit in the
  same vendor account and a `voice_id` is only a string, so row-level security
  cannot reach them: `generate` refuses a line read in a voice another user's
  `designed_voices` names, before quoting it, and `voices` refuses to `check`
  one. The listings need no filter — they are the built-in set and the Voice
  Library, and designed voices are in neither — and built-in and Library voices
  stay open to everyone;
- **a kept voice takes one of the account's voice slots** (#1261), shared by
  every user: 10 on Starter, `SCORSESE_ELEVENLABS_VOICE_SLOTS`. ElevenLabs'
  own page settles which voices count: "Voices saved from the Voice Library do
  not use custom voice slots. Custom voice slots are only used for voices you
  create with Voice Design or Voice Cloning"
  (elevenlabs.io/docs/eleven-creative/voices/voice-library, read 2026-10-16).
  So narrating with a Library voice, which the vendor adds to the account by
  itself, is free of slots, and what takes one is exactly a `designed_voices`
  row — counted from those rows (`designs::owners::slots`), since the
  operator's key lacks the `user_read` permission the vendor's live count
  needs. A `voice_keep` job checks before it asks the vendor: a user keeps at
  most three, so the first few cannot take every slot, and nobody keeps one
  once the account is full. Either is refused in plain words, free, pointing
  at the voices the user has, at `forget`, and at Library voices.
- **A kept voice is forgotten at the vendor, not only in the count** (#1381).
  `forget` with one of the user's own voice ids queues a `voice_forget` job:
  `DELETE /v1/voices/{voice_id}` at ElevenLabs, then the row
  (`designs::forget`). The vendor goes first, so a failure in between leaves
  a row over-counting until the next try — which the vendor answers with a
  404, read as already forgotten — rather than a voice taking a slot nothing
  counts. The design stays, so the candidate can be kept again. Deleting an
  account (`user delete --yes`) forgets each of its voices before the rows go
  by cascade; one the vendor will not delete does not stop the deletion, and
  the command's output names it, with the vendor's reason, for the operator to
  delete in the ElevenLabs dashboard.

Every verb that reaches the vendor is a job. A design is paid, and a job interrupted after the
reservation is run again and settles it, where a call cut off mid-way would
leave the money held; keeping is free, but it reaches the same vendor, so the
vendor stays behind one seam (`generations::Vendors::studio`) a test replaces.
`keep`, `forget` and `list` take no token.

**Cloning a voice (#1263).** `voice_clone` clones a voice from recordings the
user uploaded to their library and imported into a project: the project's
audio assets, each an upload (`library_items` of kind `audio` with no
`brief_hash` — never a generated line). Consent comes first, exactly as
locally: `consent: true` only once the voice is the user's own or its owner
said yes, and without it nothing is read or queued; the description says the
rule and links ElevenLabs' terms and prohibited use policy. Creating a clone is
not billed (elevenlabs.io/pricing lists nothing for it, read 2026-10-16), so it
takes no quote, no token and no `credits::Product`. It runs as a `voice_keep`
job — the same kind as keeping a design, one at a time, so the slot check
before it (`owners::no_slot`) cannot be raced by a keep or another clone — and
is recorded in `designed_voices` with `kind = 'cloned'` and the library items
it was made from in `recordings` (migration `0035`). Being a row there is what
makes every rule above hold for it unchanged: it takes a slot, it is its
maker's alone, `forget` queues the same `voice_forget` job, and deleting the
account deletes it at the vendor (`designs::clones`).

**Every call is recorded** in `tool_calls` — tool, arguments, project, how it
ended and its words (not its pictures), with `client` `external` for web MCP,
`assistant` for the built-in assistant, `user` for the user's own yes to a
quote the assistant showed them, and `editor` for an edit made by hand in the
web editor (*The editor*); `assistant` and `user` name their chat turn and their
place in it. Each reply's estimated size in tokens, and an external call's
client, are kept too (*Assistant turns*, *What is recorded*). A paid generation's audit row names the call that asked for it,
so *prompt → turn → tool call → generation → credits* reads back whole.

**Rate limit:** 120 tool calls a minute per user, the rest `429` with
`Retry-After`. Spending already needs a quote and a yes; this keeps one
runaway loop from filling the queue on a shared machine.

**The web editor (#545) calls the same path**: a thin JSON route over the
toolbox for a browser session (*The editor*), so no edit is written twice —
once in Rust and again in TypeScript.

## Assistant turns

The built-in assistant (#540): a model editing a user's project with the tools
web MCP serves, while their browser watches. The code is
`crates/server/src/assistant/` (the turn, the conversation, the quote box, the
model a project runs on), `crates/server/src/http/chat.rs` (the routes) and,
for the models themselves, the vendor-neutral seam in
`crates/providers/src/chat/` over Claude (`claude/`, wire in `api/anthropic/`);
their module docs carry each argument. The seam stays vendor-neutral though
one vendor is left (#1220), so a second is an addition. **The chat panel is
#545's**; this section is the API it calls.

**A turn** is one message from the user and everything the assistant does
with it: call the project's model with the conversation and every tool
`Toolbox::listing` serves, run the tools it asks for — in-process, as `Client::Assistant`, in
order — send their results back, and loop until it answers. Each project has
conversations (`chat_sessions`); a turn joins the newest unless it asks for a
fresh one, and one turn runs at a time per conversation. The system prompt asks
for a short progress line before each step and one full summary at the end,
and to lay a cut out as free sketches before spending on generation. Its words
are shown as Markdown (#768) — rendered with raw HTML off and images shown as
their alt text, since the text is the model's and untrusted — and the prompt
says light Markdown is fine and tables and images are not.

**The model's thinking is not shown** (#767). Claude still streams it as
short notes, and is still asked for them — what a request asks for
is part of the stored bytes and the cached prefix, and changing it buys nothing
a user can see — but the server drops them before the event stream, so the
browser never hears them. A working turn shows one playful status word instead,
changing every few seconds, from a list in each language's catalogue. The
thinking still happens and is still billed as output: hiding it saves the user
reading, not credits.

**Which model** (#705). A project names one (`projects.assistant_model`):
Claude Opus 5.5 (`Model::DEFAULT`, migration `0037`, #1444) for every new
project until its owner picks another; a project made before that day keeps
the model it was on, Claude Sonnet 5.5 for most (the default from migration
`0022`; migration `0031` moved every project still on a Gemini model there when
#1220 cut them, and set aside any question a Gemini turn was waiting on). The
picker offers Sonnet 5.5, Opus 5.5 and Fable 5.1, cheapest first. A project's
model is chosen in the chat panel's dropdown (`PUT /api/projects/{id}/chat/model`). It is a routing flag:
every turn runs on the model its project names when the turn starts, is charged
at that model's rates, and records the model it ran on. It can be changed at
any time, mid-conversation included, and changing it rewrites nothing. A model
whose key the server lacks is listed as unavailable with the reason in the
server's own words, and a turn on it is refused with exactly those words.

**The conversation is append-only, stored as the text that was sent — twice.**
A turn's messages are kept in `chat_turns.messages` as the exact JSON its model
was first sent, in that vendor's wire — `TEXT`, not `JSONB`, which would
reorder keys — and in `chat_turns.record` as a vendor-neutral record (words,
tool calls, tool results, server notes). The next turn's history is every
earlier turn in order: **its own bytes when it ran on the same model**, its
record written in the new model's wire otherwise. On Claude a thinking block is
valid only while everything before it is byte-for-byte what it was, and an
unchanged prefix is also what every prompt cache reads; the translation is
deterministic, so after a switch the history is the same bytes from the second
turn on. Turns from before #705 have no record and are read back from their
Anthropic JSON when a later turn needs them.

**A switch costs one uncached turn.** Thinking blocks stay with the bytes of
the model that wrote them — Sonnet's are not sent to Opus, nor Opus's to
Sonnet, since the history compares by model, not vendor — and the
new model has never seen the conversation, so the first turn after a switch
reads it all at the full input rate, and on Claude writes the cache afresh.
That is accepted; the panel says so before switching (*The chat panel*
below).

What the server vouches for — which project this is, that the user confirmed a
quote — is a server note: a mid-conversation `system` message, which no user
text or tool output can forge.
Nothing that spends depends on a note: the quote token never enters the
conversation at all. A turn cut off before the model replied (a refusal, a stop, a restart)
is closed by a one-line reply at the start of the next, and a tool call it
never ran is answered there as not run.

**Paid tools: the user's yes, never the model's.** `generate` quotes first
and spends only when called again with the quote's token (*Web MCP*). The
model never sees a token: when a call issues a quote, the token's line is cut
from what the model reads, the quote is held on the turn and sent to the
browser as `chat_quote`, and any call naming `confirm` is refused without
running. A quote **pauses the turn** (#1440) in `asking`, as a question does
(below), so nothing is charged while it waits and Stop or a new conversation
sets it aside. The user's answer is `POST /api/chat/turns/{id}/quote`, and it
resumes **the same turn**, as the result of the call that quoted: yes makes
the paid call itself — recorded in `tool_calls` as client `user` — and the
result is what it answered; no withdraws the token and the result says so.
**A change** (#709, `{confirm: false, change}`) is a no with words: the token
is withdrawn, nothing is spent, and the result carries the user's words as a
change to the quoted items — so the model rewrites those briefs (`asset_set`)
and quotes again, and the new quote needs its own yes. Writing a new message
instead withdraws it too, and the result carries the message. The model is
told to put every paid item of the plan in one call, so the person approves
once. A turn that ends any other way while it holds a quote (a Stop heard
mid-call, a failure) withdraws it, so no box is left that nothing can answer,
and the browser shows the box's buttons only while the turn is `asking`. Every
answer is kept in the turn's `quote_replies` — `{answer, micros, words}`, with
`answer` one of `confirmed`, `declined`, `changed` or `withdrawn` and `words`
the person's own on a change or a message written instead — and the
transcript shows each as one line, their words under it. A quote is answered once. The box shows each quoted item with what it
would send — the prompt, the line, the voice's description — read from the
project's brief (or the call's `prompt`) when the quote is held
(`assistant::described`).

**A question mid-edit** (#710). Beside the registry's tools the loop declares
one function of its own, `ask_user` — `{question, options}`, two to four
options — for a choice only the person can make, halfway through a turn whose
rest depends on it. It is **not a tool**: it lives in
`crates/server/src/assistant/ask/`, never in `crate::tools`, so web MCP never
lists it (a user's own client asks its user its own way). Called alone, it
**pauses the turn** in state `asking`: the turn's `questions` gains the
question, `chat_turn` carries it to the browser, which shows a card with the
options and a free-text field. `POST /api/chat/turns/{id}/answer` — or simply
the user's next chat message, which *is* the answer — resumes **the same
turn**, the answer being that call's result, so the model carries on with its
plan and its thinking intact and the history stays append-only. Called beside
other tools, they run and `ask_user` is refused with a line saying to ask
alone, so a pause never splits results that must be sent together. **Waiting
costs nothing and never expires**: a paused turn holds no process, no money and
no token, survives a restart as it was, and a late answer costs at most one
uncached call. Stop, or a new conversation, sets the question aside (the turn
ends `stopped`); a quote cannot be answered while a question waits. The system
prompt says to ask sparingly — only when the answer changes what comes next
and the request points to no default — and never about money.

**Pictures to pick from** (#901). The same pause with stock results for
options: `pick_stock` — `{question, candidates: [{kind, id}], resolution?}`,
two to eight — declared beside `ask_user` and, like it, never a tool. Every
candidate must be one **this turn's** `stock_search` replies listed (read back
from the turn's record) and still in the user's 24-hour stock cache, which is
where what the browser shows of it comes from; a picker naming anything else is
refused and the turn goes on. The turn's `questions` gains the question with
its `candidates` filled, and the chat panel opens a modal: a grid of the
source's own previews, each enlarged or played (muted) on a click, one or more
selected, or none, and a field for words. Nothing is downloaded to show them.
The answer, `{picked: [keys], answer?}`, imports **only what was picked** —
as the user's own `stock_import` call (`client = 'user'`, from
`Toolbox::stocked_from`'s library, Pixabay and LottieFiles by default), into
`assets/` like any import, admitted to the library before the document is
saved — and then resumes the same turn with the new asset ids as the call's
result. A Lottie (#908) is a candidate like any other, shown by its animated
GIF with LottieFiles named on its tile, and candidates of different kinds share
one picker; a pick of one runs the registry's `stock_import` itself, so its
JSON is kept under `pages/` with the project's files — never an asset — and
the call's result is that reply: the file a page loads and the page that plays
it. A typed
message instead is the answer, as for a question. The system prompt says to
show one only when several results are equally good or the model is unsure
between them; one clearly right is imported without asking.

**Money.** Every call is charged from its reply's usage — input, output,
Claude's five-minute and one-hour cache writes, cache reads, each at its own rate for the turn's model in
`prices::chat`, so a cache hit is cheaper for the user too — times 1.8, as one `charge` entry naming the turn
(`credit_entries.chat_turn_id`); the spending history folds a turn's calls into
one row. Not reserved for, since the cost exists only once counted: a turn is
**refused up front (`402`) at a balance of zero or less**, and stops between
calls once the balance runs out or the turn reaches the operator's cap
(`SCORSESE_ASSISTANT_TURN_CAP`, default $3.50: the cap is on what the user
is charged, so it rose with the multiplier, #1247). That figure is Sonnet
5.5's; every other model's cap is it times the model's blended rate over
Sonnet's, nine input tokens to one output as the picker's bar weighs them
(#1461) — about $7 on Opus 5.5 and $17.50 on Fable 5.1 — so one turn may do
the same amount of work whichever model runs it. The call that crosses either
line was already made and is charged; one call is bounded by `max_tokens`.

**Caching** is where the money is, since every call resends the conversation.
On Claude, one breakpoint on the system prompt caches the tools and the prompt
together for an hour — identical for every user, so one write serves the whole
server — and the API's automatic breakpoint caches the conversation's tail for
five minutes. Nothing about a user or the time goes in the prefix.

**Effort is the user's, per message** (#769) — Claude's `effort` — chosen beside Send as **Quick** (`low`), **Balanced**
(`medium`) or **Thorough** (`high`, the default), and remembered per project
in the browser. A message without one is `high`; every turn records its
effort, and one that resumes after a question or a quote's answer keeps the
effort it began with (`crates/server/src/assistant/mod.rs`, *Effort*). A refusal by a
vendor's safety filters ends the turn as `refused`, charged for what the call
used; it is not retried on another model, because the user chose the model.

**Without the model's key** (`ANTHROPIC_API_KEY`) a turn on it is refused with `503` and the variable's name, and the
rest of the server runs as it did.

**The chat panel** has the model dropdown. Picking another model while the
conversation's cache may still be warm — its last answer less than the cache
lifetime of the model being left ago (`cache_seconds`, an hour on every model:
Claude's longest entry), or a turn still running — first asks: *switching
models causes a cache miss on the next prompt*, Switch or Cancel. An empty
conversation, or one whose last answer is older than that, switches at once.
The age is the server's `finished_at`, so the check survives a reload.

**Each model in the dropdown carries a cost bar** (#706), green at the cheapest
to red at the dearest, never a price. Its length is the server's `cost`: the
model's rates blended at nine input tokens to one output token (assistant turns
are input-heavy — every tool reply goes back in as input), as a percentage of
the dearest model's blend (`crates/server/src/assistant/cost.rs`). The server
computes it so the rate table stays the one source of every number. The mix is
fixed until measured usage (#707) says otherwise.

| route | who | what |
| --- | --- | --- |
| `GET /api/projects/{id}/chat` | a member | `{project, session, turns, model, models}`: the newest conversation's turns, oldest first; the project's model; every model offered as `{id, label, vendor, unavailable, cache_seconds, cost}` |
| `PUT /api/projects/{id}/chat/model` | a member | `{model}` → the model as listed; from the next turn the project runs on it. `400` for a model not offered |
| `POST /api/projects/{id}/chat` | a member | `{prompt, fresh?, effort?}` → `202` with the turn; `effort` is `low`, `medium` or `high` (absent: `high`), `400` for anything else; `402` no credit, `409` a turn is running, `503` not configured |
| `GET /api/chat/turns/{id}` | a member | `{turn, tools}`: the turn and the log of every tool call it made, in order |
| `POST /api/chat/turns/{id}/stop` | a member | `202`; the turn stops before its next step — at once, its question set aside, when it is `asking`. `409` if it is neither |
| `POST /api/chat/turns/{id}/quote` | a member | `{confirm: true\|false, change?}` → `{spent, refused, turn, note}`; `turn` is the one the quote paused, carrying on with the answer as the quoting call's result (#1440); `change` with `confirm: true` is `400`, and so is any answer on a turn not paused on a quote |
| `POST /api/chat/turns/{id}/answer` | a member | `{answer}` — or, for a picker, `{picked: [key…], answer?}` (`[]` is none of them) — → `202` with the same turn, running again (what was picked is imported before the model hears of it); `400` when no question waits on it, when `picked` answers a question in words, or when it names a candidate the picker did not offer; `402`/`503` as for a new message |

A turn (`TurnView`) carries its state — `running` (or `asking`, paused on a
question), then `answered`, `refused`, `capped`, `stopped`, `failed` or
`interrupted` (the server stopped under it) —
its `answer`, the model, token totals, `charged_micros`, and `quote`
(`{tool, items: [{subject, says, brief, description}], lines, micros,
expires_at}` — `brief` is `prompt`, `line` or `voice`; `lines` is the total and
any line no item claims; a quote held before #709 has no `items` and every
line in `lines`) with `quote_answer` (`null` while the box
should show on an `asking` turn, then `confirmed`, `declined` — a change asked
for included — or `withdrawn`) and `quote_replies`, every answer its quotes
were given (`{answer, micros, words}`, above), and `questions`, every question it asked in order as
`{question, options, answer}`, `answer` `null` while it waits. A picker's
(#901) also carries `candidates` — each `{key, source, kind, id, preview_url,
look_url, width, height, seconds, author, page_url, tags}`, `key` being
`<source>-<kind>-<id>` (`pixabay-video-39009`, `lottiefiles-lottie-121035`)
and `look_url` what the enlarged view shows — and, once answered, `picked`, the keys chosen
in the order shown (absent when the answer was words alone); its `options`
are empty.

**On `GET /api/events`**, beside `job`:

| `type` | fields | what the panel does with it |
| --- | --- | --- |
| `chat_turn` | `turn`, `balance_micros` | a turn started, was charged for a call, or ended: replace it, show what it cost and the balance |
| `chat_text` | `turn`, `text` | more of the words the assistant is writing: append |
| `chat_tool` | `turn`, `tool`, `state`, `said` | a tool `running`, then `answered` or `refused`, with the start of its answer |
| `chat_quote` | `turn`, `quote` | show the confirmation box |
| `project` | `id`, `revision` | the assistant changed the project: re-read it, and the preview refreshes |

After `resync`, or on reconnecting, re-read `GET /api/projects/{id}/chat`.

Not here yet: compacting a conversation that outgrows the context window (a
fresh conversation is the answer for now), and a per-user daily limit.

### What is recorded

How each model performs and what it costs is queryable straight from Postgres
(#707) — recording only: nothing here is shown to a user, and there is no page
for it. Each figure is written as a side effect of work that already writes a
row, in the same transaction.

- **`model_calls`** — one typed row per call the assistant made to a model,
  the same shape on every vendor (`providers::chat::Usage`): `model`, `vendor`
  (`anthropic`; rows from before #1220 also hold `google`), `input_tokens`
  (full price — the cached part is not here), `output_tokens` (thinking
  included, as Anthropic bills it), `thinking_tokens` (the part of output
  that was thinking, which only Gemini reported apart, so `NULL` on every row
  since #1220), `cache_write_5m_tokens`, `cache_write_1h_tokens`,
  `cache_read_tokens`, `latency_ms` (the answering attempt, streaming
  included), `stop_reason`, `cost_micros` (at the vendor's rates, before the
  multiplier), the turn and its `position` in it, and `credit_entry_id` — the
  charge it caused. A table of its own rather than columns on the ledger,
  which holds every kind of entry and is about money.
- **`tool_calls.reply_tokens_estimate`** — what a tool's reply weighs as input
  to the model's next call, which is where a tool is paid for. An estimate,
  and named one: text at four bytes a token, a picture at width × height / 750
  (Anthropic's figure, read from the PNG header). A vendor's own count would be
  a round trip per call, and an external client's tokenizer is unknown.
- **`tool_calls.client_name`, `client_version`, `api_token_id`** — for an
  `external` call, the `clientInfo` the token's last MCP `initialize` named
  (kept on `api_tokens`; web MCP has no sessions, so the token is the
  connection). An external client's model and tokens are on its side; this is
  what tells its use apart.

Run them as the server's own login role (`psql` on its `DATABASE_URL`), which
the row-level security policies do not scope to one user (`db::scope`):

```sql
-- Average tokens and latency per call, by model, over the last week.
SELECT model, count(*) AS calls,
       avg(input_tokens + cache_read_tokens)::bigint AS avg_input,
       avg(output_tokens)::bigint AS avg_output,
       avg(thinking_tokens)::bigint AS avg_thinking,
       avg(latency_ms)::bigint AS avg_ms,
       sum(cost_micros) / 1e6 AS cost_dollars
FROM model_calls WHERE created_at > now() - interval '7 days'
GROUP BY model ORDER BY calls DESC;

-- Which tools are expensive: reply size, by tool and by who called it.
SELECT tool, client, count(*) AS calls,
       avg(reply_tokens_estimate)::bigint AS avg_reply_tokens,
       sum(reply_tokens_estimate) AS total_reply_tokens
FROM tool_calls WHERE finished_at IS NOT NULL
GROUP BY tool, client ORDER BY total_reply_tokens DESC NULLS LAST;

-- External use, by client.
SELECT coalesce(client_name, '(no initialize)') AS client, client_version,
       count(*) AS calls, count(DISTINCT user_id) AS users
FROM tool_calls WHERE client = 'external'
GROUP BY 1, 2 ORDER BY calls DESC;
```

**Reviewing the tool surface** (#779). The queries that decided which tools to
merge, kept so the same review can be run after a reshape and the two compared.
Pass `PGOPTIONS='-c default_transaction_read_only=on'` to `psql` so a review can
only read. The record holds other users' arguments and replies: what leaves the
machine is counts, rates, sizes and tool names, never a value or a reply.

```sql
-- Use and refusals per tool, one column per client. `editor` is the web
-- editor's own hands; `user` is a person's yes to a quote.
SELECT tool,
       count(*) FILTER (WHERE client = 'assistant') AS assistant,
       count(*) FILTER (WHERE client = 'assistant' AND outcome = 'refused') AS refused,
       count(*) FILTER (WHERE client = 'editor') AS editor,
       count(*) FILTER (WHERE client = 'external') AS external
FROM tool_calls GROUP BY tool ORDER BY assistant DESC, editor DESC;

-- Refusal reasons, grouped: the reply of a refused call is its reason. Read
-- here, summarised elsewhere by shape (it quotes the user's ids and fields).
SELECT tool, left(reply, 120) AS reason, count(*)
FROM tool_calls WHERE outcome = 'refused' AND client = 'assistant'
GROUP BY 1, 2 ORDER BY 3 DESC;

-- Back-to-back pairs within a turn: overlap, a retry loop, or a missing tool.
SELECT a.tool AS first, b.tool AS then, count(*)
FROM tool_calls a
JOIN tool_calls b ON b.turn_id = a.turn_id AND b.position = a.position + 1
WHERE a.client = 'assistant'
GROUP BY 1, 2 ORDER BY 3 DESC;

-- Listing cost: the first call of a conversation is the tool list, the system
-- prompt and one message, nothing cached — so it measures the list.
SELECT m.model, m.input_tokens + m.cache_read_tokens AS first_call_input
FROM model_calls m JOIN chat_turns t ON t.id = m.turn_id
WHERE m.position = 1
  AND t.id = (SELECT min(id) FROM chat_turns WHERE session_id = t.session_id);

-- How input grows: per turn, the input each call carries and the cached share.
-- Divide the list's size by avg_input for the share of each call that is list.
SELECT turn_id, count(*) AS calls,
       avg(input_tokens + cache_read_tokens)::bigint AS avg_input,
       round(sum(cache_read_tokens)::numeric
             / nullif(sum(input_tokens + cache_read_tokens), 0), 3) AS cached_share,
       sum(output_tokens) AS output, sum(cost_micros) / 1e6 AS cost_dollars
FROM model_calls GROUP BY turn_id ORDER BY turn_id;

-- What a reply really costs: every later model call in the conversation
-- re-reads it, so weight each reply by the calls that came after it.
SELECT t.tool, count(*) AS calls, sum(t.reply_tokens_estimate) AS reply_tokens,
       sum(t.reply_tokens_estimate * (
         SELECT count(*) FROM model_calls m JOIN chat_turns c ON c.id = m.turn_id
         WHERE c.session_id = ct.session_id AND m.created_at > t.finished_at
       )) AS reread_tokens
FROM tool_calls t JOIN chat_turns ct ON ct.id = t.turn_id
WHERE t.client = 'assistant'
GROUP BY t.tool ORDER BY reread_tokens DESC NULLS LAST;
```

The baseline from the first review (one conversation, 2026-10-05, measured
on Gemini 3.8 Flash, since cut from the picker by #1220): the web tool list was **≈27.4k tokens a model call** (a 28,565-token
first call, less ~1.1k of system prompt and opening), which was 15% of all the
input the conversation's 115 calls read and ~9% of its cost; `still` and
`project_read` replies were ~96% of what later calls re-read from tool replies.

**Local use is not in this database.** A Claude Code user's calls are in its
transcripts under `~/.claude/projects/*/` as JSONL. Count `tool_use` blocks
only — a bare `"name":"mcp__scorsese__…"` also matches the tool definitions
Claude Code records when it loads a deferred tool, which overcounts every tool
loaded at least once:

```sh
grep -rho '"type":"tool_use","id":"[^"]*","name":"mcp__scorsese__[a-z_]*"' \
  ~/.claude/projects/ --include='*.jsonl' \
  | sed 's/.*mcp__scorsese__//; s/"$//' | sort | uniq -c | sort -rn
```

Drop `-h` and `sort | uniq -c` for each transcript's calls in order, which is
what a sequence question (a `synth_check` before each `synth_write`?) needs.

## The pages

What a user sees before the editor (#544); `web/README.md` has how the code is
laid out.

| URL | what |
| --- | --- |
| `/` | the landing page for a visitor without a session (#904): a strip of films made with scorsese, one whole video in the visitor's language, how it works and what it handles, shown one section at a time with crossfades between them (`web/src/landing/`); with a session, the projects list |
| `/login` | the landing page with the sign-in popup open: one "Continue with Google" button, which is sign-up too, and what went wrong when a sign-in came back with `?error=`; under the button, the line saying that signing in accepts the terms and the privacy policy |
| `/terms`, `/privacy` | the terms of use and the privacy policy (#547), public and crawlable whether signed in or not, in Portuguese only, the contract's language, with a note for a visitor reading in another (`web/src/legal/`). They are data (`terms.ts`, `privacy.ts`) stating what the code and the deployment do, so a change to prices, retention, cookies or vendors changes them too. The seller's identification (`seller.ts`) is also in the landing page's footer, as Decreto 7.962/2013 asks |
| `/projects` | create, open, rename, delete |
| `/projects/{id}` | the library files that project uses (`GET /api/library?project=`) |
| `/library` | every file: filter by kind, search by name, sort; upload by button or by dropping files |
| `/library?item={id}` | the same, with that file's details open — what "you already have this" and the spending history link to |
| `/spending` | the history, filterable, with the filter's total; the balance in the header opens it |
| `/projects/{id}/edit` | the editor — *The editor*, below |

**Flat and Drive-like, as #527 settled.** A tile is thumbnail, name, kind and
size; a click opens the details (dimensions, duration, the projects using it,
the description the assistant reads, and for a generated file how it was made
and what it cost); a double click views or plays it, streamed in ranges.

**Money on a page** is always the server's integers, in micro-dollars, shown
as dollars and nothing else (*Money*, above) — the page converts nothing.

**Adding a library file to a project is not a button here.** The server keeps
no per-edit endpoints (`http/projects.rs`): what a project says changes through
`core`'s editing functions, reached by the tools and the editor. Bringing a
library file into a project is web MCP's `import` (*Web MCP*), and the editor's
(#545) by the same path.

## The editor

`/projects/{id}/edit` (#545), laid out as the desktop app is: a sidebar on the
left — a **Library** button at the top (#702, #1006), then the project's
**Assets** and the user's **Templates** — the preview in the middle, the
timeline under both, and on the right the selected clip's inspector over the
assistant's chat panel. The code is `web/src/editor/`, the route
`crates/server/src/http/editor.rs`; their module docs carry each argument. It
is a first version by the rule in `CLAUDE.md` — *the user can start editing
with it* — and is meant to be tuned from use.

**Assets are what is in the project; the library is what is owned** (#702).
The sidebar lists only the first. The Library button opens the library page's
own browser in a modal, in picking mode, over the editor blurred behind it:
picking a file is an `import`, which brings it into Assets without placing it,
and the user drags it from there onto a lane. Templates are not in the modal — a template has no
"in this project" state, it is inserted at the playhead, which the sidebar's
Templates section already does.

**The hand-edits are few, and each is a tool call.** Add a track, drop a file
on it, drag a clip along it or onto another lane, drag an edge to trim, press
Delete on the selected clip, and type a plain value into the inspector —
`track_new`, `import` (from the Library modal) and `place_clip`, `clip_move`,
`clip_remove`, `clip_set` — through `POST /api/projects/{id}/tools/{name}`,
which runs the toolbox web MCP and the assistant run, recorded as client
`editor`. The browser reads `project.json` to draw the timeline and never writes
it. Anything with structure to it — a title, a dissolve, a ramp — is a sentence
to the assistant, not a menu (`CLAUDE.md`, *The GUI is thin*).

| route | who | what |
| --- | --- | --- |
| `POST /api/projects/{id}/tools/{name}` | a member, **by session** | `{arguments, revision?}` → `{said, project}`: the tool's words and pictures, and the project as it is now (`null` after a `still`) |

- **An allowlist**, not the whole surface: `track_new`, `track_remove`,
  `asset_remove`, `place_clip`, `clip_set`, `clip_move`,
  `clip_remove`, `sequence` (the edits, `http::editor::EDITS`) and `import`,
  `still`, `template_save`, `template_insert`. Anything else is
  `404` — a page has no business writing a whole document or spending money.
  An API token is `403`; a program uses web MCP.
- **An edit names the revision it was worked out on**, and a project at any
  other revision — when the tool opens it or when it saves — is `409` with
  nothing written: the conflict rule of *Projects*. Web MCP and the assistant
  re-run a call on a project that moved, since a tool is a function of the
  document; a drag is not, because it was computed on the timeline the user
  saw. The page reads the project again and says so. `import`, `still` and the
  two template tools take no revision: none changes what a drag is computed
  on, and where an inserted template lands is `core`'s rule applied to the
  project as the server finds it, not something the user drew.
- A refused edit is `422` with the tool's own reason, shown above the timeline;
  the clip springs back, since the page only draws what the server holds.
- It shares web MCP's 120 calls a minute. An edit that lands sends `project`
  on `GET /api/events`, so another tab redraws too.

**The inspector** shows start, duration, speed, fit, and position, rotation and
scale by the desktop inspector's rule: a property with no keyframes, or one held
point, is a value and gets a field; a ramp shows as *animated* and offers none,
so typing never flattens somebody's animation. A value is sent when the field
is left, not per keystroke. An **image sequence**'s clip adds its hold (frames
per still) and loop, sent as `sequence` on the asset, so every clip of it
changes. A **generated** clip shows its brief and its state, read-only: a
changed brief is a sentence to the assistant, which quotes before anything is
spent (#538).

**`clip_set`** is the registry tool this added: speed (retimed, as a 2× button
means), fit, and position, rotation and scale as single held values —
`docs/mcp.md` has it. The assistant and web MCP get it too. **`clip_move`** and
**`clip_remove`** came after (#576), for the same reason: a clip dropped on the
wrong lane had nowhere to go but the assistant. A drag that ends on another
lane is one `clip_move` with the new start, never a move then a trim — and
since #781 a drag along a lane is a `clip_move` too, the one tool for both; Delete or
Backspace removes the selected clip — its asset stays, and nothing closes up
behind it. Neither fires while a text field has the keyboard.

**`asset_remove`** and **`track_remove`** (#396) are the bin on an asset's tile
in the assets panel (shown on hover, #766) and on a lane's header. Each is a
plain confirm first, listing the clips that go with it — the timeline is
already in the page — and *yes* sends exactly that list. The tools refuse any
other list, so a timeline that moved on since the confirm was drawn is a
refusal, never a guess.

**The preview doctrine** (#542, recorded at the maintainer's asking). **The
server draws every picture; the browser plays a video.** Real-time compositing
in the browser is not attempted.

- **The server renders a low-resolution preview video of the cut** at the
  chosen quality (*Renders*, previews), from proxies of heavy videos, and the
  browser plays it in an ordinary `<video>`. **Scrubbing is seeking** in that
  video.
- **After an edit, the preview is re-rendered** — by itself, once the revision
  has rested for a second, with no press. Sketch and stale generated assets
  render as slug cards, which is free and keeps it fast. An unchanged cut
  answers at once from the cache; a preview of a revision the project has
  already moved past retires when its turn comes (*Renders*), so a burst of
  edits costs one render, not one per edit. The moment the revision moves the
  old video is dropped, and until the new one is ready the frame under the
  playhead is the `still` tool's — the render's own compositor, asked for once
  the playhead rests for a fifth of a second, at the same raster, cached per
  (revision, frame). The page follows the job on the event stream and asks
  again every few seconds too, because the quick tunnel carries no events.
- **Preview quality** — Full, 1/2, 1/4 — is a fraction of the delivery size of
  the project's shape (1920x1080 for 16:9, 1080x1920 for 9:16), chosen beside the transport and said
  under it, and it means the same as the desktop app's control: one
  `scorsese_render::Quality`, one arithmetic. Full is the render's own picture
  from the originals; the reduced ones draw fewer pixels and read proxies, and a
  `native` layer shrinks with them so the layout is the delivery's. It is kept
  **per browser**, not per project and not in `project.json`: it is about the
  device and the person's patience, not about the edit. Half is the default.
- **This is the v1 default, chosen to be simple**, and it will be tuned from how
  real users behave: how often they edit between looks, how long a preview
  takes on the shared machine, whether a quarter is ever chosen. The settle
  time, the default quality and the rule that re-renders without a press are
  the three knobs, and none of them is a decision worth defending against
  evidence.

The frame's **shape** is the project's own, the document's `aspect` (#1278):
a project started for a platform opens in its shape, and nobody chooses
anything. The header shows it (16:9, 9:16, 1:1, 4:5, and the project's own if
it is none of those), and choosing another calls `project_reframe` with the
revision — the re-fit every surface makes — and shows its reply: what it
re-fitted, and what it left to settle, which the assistant can be asked to
settle. Changing the platform to one of another shape re-fits the same way. The
shape sets the preview raster and the sizes the **Render** dialog offers, 1080
lines on the shorter side first (`Ratio::delivery`); that dialog asks for a
finished render, follows it through the queue and lists the project's kept
renders to download.

**The chat panel** is *Assistant turns* on screen: each turn's prompt, a status
word while it works, its answer, its question cards and pickers, the quote
confirmation box, what it cost and the balance after it, and Stop while it runs
(#1437). **Tool calls are not shown**, not collapsed and not behind a click: a
tool name is implementation, and the user never deals with it. Every call is
still stored in `tool_calls`. Generations show as one "Generating…" line while
any the conversation ordered is waiting or running. The chat follows new text
only while the reader is at the bottom. "New conversation" is a button with a
confirmation just above it; Start empties the chat at once and the next message
is sent with `fresh: true`, which opens a new `chat_sessions` row. The old
conversation stays in the database. Without the project's
model's key (`ANTHROPIC_API_KEY`) the server's `503` "not configured" is shown as a note, and nothing else changes.

## Templates

A piece of an edit saved to be copied into any of the same user's projects —
an intro, an outro, a running gag, the whole shape of a daily video (#546). The
code is `scorsese_core::template` (what a template is, and where one lands),
`crates/server/src/templates/` (the rows) and the three tools in
`crates/server/src/tools/own/templates.rs`; their module docs carry each
argument.

**A template is a `project.json` document**: `core`'s `extract` keeps the
chosen clips, the tracks they are on and the assets they show, moved so the
earliest clip starts at zero, under the template's name and the source's frame
rate. Not a new format — so `Project::validate` says whether one is coherent,
and **a schema bump migrates stored templates on start** with the same
`scorsese_core::migrate` steps as projects, in the same all-or-nothing way. An
arrow is saved with the clip it follows or not at all; a brief in flight is
kept as a sketch.

**Inserting copies; it never links** (#527). `core`'s `insert` writes the clips
into the project at the time asked for, so changing or deleting a template
never changes a video it went into. The files are not copied: a template names
library files by hash exactly as a project does, and a file the project
already has is shared, so a Veo intro is paid for once. Ids are kept where free
and suffixed (`-2`) where not.

**Where its tracks land** — decided in `core`, and the one rule to know: by
position among tracks of the same kind, from the bottom. The template's first
video track goes on the project's first video track, its second on the second,
audio alike, and a project with no tracks takes the template's own. Where
something is already in the way, that track **and every one above it** of that
kind go onto new tracks on top, in the template's order — sending only the
blocked one up would draw the template's background over its own title.
Nothing already in the project moves; an insert that pushes the rest of the
cut later is a different edit, not built.

**The rows.** `templates` (owner, the document, a `name` generated from it and
unique per user whatever its case) and `template_assets`, derived from the
document on every save exactly as `project_assets` is, with the same key to
`library_items` — which is what makes the library refuse to delete a file a
template uses, naming the template. Both follow *Per-user isolation*. Saving
under a taken name is refused unless the caller says `replace`; replacing is
how a template is updated.

**What a template is for** (#560) is a `description` column beside the
document — prose the assistant reads before inserting, at most 2000
characters, held to `scorsese_core::template::Description`'s rule. Not a field
in the document, because the document is a `project.json` document and a field
there would be a format change (schema bump and migration) for something no
project has; the local side, when it keeps templates, stores the same
`Description` beside its document the same way. Replacing a template without
a description keeps the old one.

| tool | what |
| --- | --- |
| `template_list` | the caller's templates: id, name, length, clips, tracks, the assets shown, and what each is for |
| `template_save` | `{project, clips, name, description?, replace?}` — those clips of that project, as a template |
| `template_insert` | `{project, template, at_seconds}` — a copy of it, its first clip at that time; the reply names every clip and the track it went on |

All three are served to web MCP and the assistant, and the editor reaches the
last two through its tool route (*The editor*): **Save as template** on the
selected clips (shift-click adds to a selection), and a **Templates** list in
the assets panel whose entries go in at the playhead.

| route | who | what |
| --- | --- | --- |
| `GET /api/templates` | a member | their templates, by name |
| `DELETE /api/templates/{id}` | a member | `204`; the videos it went into keep their copies |

Not in v1: sharing templates across users, and templates for local `.scor`
folders — which `core`'s two functions already serve when the CLI wants them.

## Out of scope for now

Pix payments (#548), folders in the library, a shared cross-user library,
collaboration, cloud hosting.
