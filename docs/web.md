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

**It runs on the maintainer's machine in Docker Compose**, four service
containers and one that keeps them safe:

| container | role |
| --- | --- |
| `postgres` | the database, on a persistent volume |
| `scorsese-server` | the Rust server, with ffmpeg inside; library files, render cache and scratch mounted from host disk |
| `web` | nginx serving the built React files |
| `named-tunnel` | `cloudflared` holding a **Cloudflare Tunnel** — how the site reaches the internet without a static IP; it also terminates HTTPS, so there is no reverse proxy of our own. Before there is a domain, `quick-tunnel` instead, or no tunnel (#568, *Testing before there is a domain*) |
| `backup` | scheduled `pg_dump` plus a sync of the library **off the machine** (#532). Not part of serving a request — the fifth container exists because a home machine has no redundancy, and a backup that depends on someone remembering is not one |

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
feature*). How a turn runs, what it costs and the API the web editor calls
are *Assistant turns*, below.

## `crates/server`

The API (#530): axum over HTTP, sqlx over Postgres. The crate's own `lib.rs`
doc argues both choices and the tool-registry decision; this is what a
developer needs to run it.

**Every route lives under `/api`**, the health check included
(`GET /api/health`: `200` when the database answers, `503` when it does not).
One prefix is one routing rule — for `web/`'s dev proxy, which forwards `/api`
unchanged to `SCORSESE_API`, and for whatever fronts the server in production.

**Configuration** comes from the environment, through the same lookup the
provider keys use, and is documented in `.env.example`:

| Variable | What it is |
|---|---|
| `DATABASE_URL` | The Postgres to connect to. Required; the server will not start without reaching it, and it is never printed. |
| `SCORSESE_STORAGE` | The absolute directory users' files are kept under. Required. |
| `SCORSESE_CACHE` | The absolute directory what can be rebuilt is kept under — thumbnails, uploads still arriving, finished renders. Required, and refused inside `SCORSESE_STORAGE`, which is backed up. |
| `SCORSESE_RENDER_QUOTA` | How much disk finished renders aim to stay under: `500MB`, `20GB`, `1TB`. Defaults to `20GB`. See *Renders*. |
| `SCORSESE_BIND` | Where to listen. Defaults to `127.0.0.1:8080`, this machine only. |
| `GEMINI_API_KEY`, `ELEVENLABS_API_KEY` | The provider keys paid generations are made with (*Web MCP*), read by the one resolver `docs/credentials.md` describes. Optional: without one, a generation needing it fails, free. |
| `ANTHROPIC_API_KEY` | The key the assistant calls Claude with (*Assistant turns*), by the same resolver. Optional: without it the assistant answers "not configured" and nothing else changes. |
| `SCORSESE_ASSISTANT_TURN_CAP` | The most one assistant turn may cost a user, in dollars: `2.50`. Defaults to `2.00`. |
| `SCORSESE_ASSISTANT_MODEL` | The model the assistant runs on. Defaults to `claude-opus-5-5`; one with no rate in `prices::claude` stops the server from starting. |

**Schema migrations** are embedded in the binary and run at startup, before the
first request. Files in `crates/server/migrations/` are numbered
`NNNN_name.sql` without gaps, only go forward, and are never edited once merged
— the folder's README has the rules, and a test refuses a file sqlx would
silently skip.

**Database tests run, or they fail — they never skip.** `make test` and
`make coverage` go through `tools/with-postgres`, which starts a throwaway
`postgres:17-alpine` container on a random local port and removes it however
the run ends. So **Docker is needed for `make gates`**, unless
`SCORSESE_TEST_DATABASE_URL` names a Postgres the tests may create and drop
databases on. An ambient `DATABASE_URL` is deliberately ignored, so a test run
never touches the development database. CI's `check` job and the weekly coverage
workflow run a Postgres service container instead.

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

**The traffic path.** Cloudflare's edge terminates HTTPS and sends the request
down the tunnel `cloudflared` holds open from this machine. `cloudflared` sends
everything to `web` on its port 80; nginx serves the React build and passes
`/api/` to `server`, unchanged but for naming the client's address
(*Accounts*, *The login's brake*). Nothing is published to the network: `web`
also answers on `$SCORSESE_WEB_BIND:$SCORSESE_WEB_PORT` — its port 8080 inside,
a second door so it can tell the two apart — loopback unless `.env` says
otherwise, for checking the site from this machine and for `tailscale serve`.

**Which tunnel is a setting, not an edit.** `COMPOSE_PROFILES` in `deploy/.env`
names one: `named-tunnel` (production — a domain and a token), `quick-tunnel` (a
throwaway `*.trycloudflare.com` address, for testing), or blank for none. Both
are the same `cloudflared` image told two different things, and Compose starts
only the one named. `make deploy` checks the file in all three modes, and that
each starts its own tunnel and no other.

**The `/api` split lives in nginx, not in the tunnel's ingress rules.** A tunnel
run by token keeps its rules in Cloudflare's dashboard, where no pull request
shows them and no local run exercises them; in `nginx.conf` they are versioned,
reviewed and identical on loopback and on the public hostname. nginx is needed
for the SPA fallback anyway, so the tunnel is a single rule that never changes.
The MCP endpoint (#539) is `/api/mcp`, under `/api` like everything else, so it
needs nothing of its own there.

**A fifth container, `backup`**, beside the four in *The shape*. It runs on the
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
arriving, finished renders (*Renders*) and the scratch they are made in — and
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
   the connector, the `cloudflared` container is the connector. Then add **one
   public hostname** — the site's domain, service type `HTTP`, URL `web:80`
   (nginx's tunnel-only door, never 8080 — *The login's brake* says why) —
   and no other rules. The token goes in `CLOUDFLARE_TUNNEL_TOKEN`, and
   `COMPOSE_PROFILES=named-tunnel` turns it on.
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

It is HTTPS, so logging in works as in production. What to know about it:

- **The address changes every time the container starts** — after a power cut,
  after an `up` that recreates it. Read the log again and send the new link.
- **It is public.** Anyone with the link reaches the login page, and the
  accounts are the only gate — there is no sign-up, so that is the operator
  creating an account per person (*Accounts*).
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
only from `https://` or `localhost`. From `http://192.168.x.x:8088` the login
answers `200`, the browser drops the cookie, and the next request is logged
out. The same goes for `http://100.x.y.z:8088` on the tailnet, encrypted
underneath or not — the browser sees plain HTTP, which is what `tailscale
serve` is for. So a device at home logs in through the tailnet or the quick
tunnel.

A setting that drops `Secure` for a "trusted network" was weighed and **not
built** (#568):

- Every path planned before a domain already has HTTPS for nothing — the quick
  tunnel and `tailscale serve` — and a phone joins a tailnet in about the time
  it takes to find the machine's LAN address.
- Over plain HTTP the password and the session cross the network in the clear.
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

### Updating

From the checkout, on `main`:

    git pull
    cd deploy
    docker compose exec backup backup          # a dump from just before
    CARGO_BUILD_JOBS=4 docker compose up --detach --build

Compose recreates only the containers whose image or settings changed. The new
server runs its migrations before serving, and they only go forward, so **the
way back from a bad update is the dump taken first**: check out the previous
commit, rebuild, and restore that dump. BuildKit's cache for the Rust build
grows over time; `docker buildx du` shows it.

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
  number of days — so a bug that deletes users' files is not faithfully
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
`crates/server/src/accounts/`, and each module's doc carries its argument; this
is the whole of it in one place.

**No public sign-up.** The operator creates accounts, resets passwords and
deletes accounts from the server's own binary, run where the server runs:

```text
docker compose exec server scorsese-server user create ana@example.com
scorsese-server user reset-password ana@example.com   # also logs out every browser
scorsese-server user delete ana@example.com --yes     # rows and files, for good
scorsese-server user list
scorsese-server user locks                            # who the login is braking
scorsese-server user unlock ana@example.com           # or an address, as `locks` prints it
scorsese-server token create ana@example.com "ana's laptop"
```

Passwords are **generated and printed once**, never typed on a command line
(shell history, `ps`); the user changes theirs from the web app
(`POST /api/me/password`). Stored as **argon2id** PHC strings with the crate's
OWASP-minimum parameters, which travel inside each hash, so raising them later
breaks nobody.

**The browser holds a session cookie**: 32 random bytes, of which Postgres
keeps only the SHA-256, valid 30 days from login and not extended by use.
`HttpOnly; Secure; SameSite=Strict; Path=/api` — the web app and the API share
one origin, so Strict costs nothing, and with every write taking a JSON body
it is the CSRF defence. `Secure` means a browser logs in only over `https://`
or on `localhost`; *Testing before there is a domain* has the HTTPS paths
there are before a domain, and why plain HTTP is not one of them. Server-side rather than signed, so logout, a reset and
a deletion end sessions *now* — and so **there is no signing key**: the server
has no secret of its own for `docs/credentials.md`'s resolver to find.

**Anything that is not a browser sends a per-user API token** —
`Authorization: Bearer scor_…`. That is the answer for web MCP (#539) in v1:
the MCP spec's remote authorization is OAuth 2.1, but the clients people will
point at it first accept a static bearer header, and an authorization server
(client registration, consent, refresh, PKCE) is a large surface for no client
that needs it yet. When one that *only* speaks OAuth matters — claude.ai's
custom connectors are the likely one — that is its own issue, and its flow ends
by issuing these same tokens. Tokens are shown once, hashed like sessions,
revoked one at a time, and **issued only from a browser session**, so a leaked
token cannot mint its own replacement.

| route | who | what |
| --- | --- | --- |
| `POST /api/login` | anyone | `{email, password}` → the account, and the cookie; `429` when braked |
| `POST /api/logout` | a member | ends the session |
| `GET /api/me` | a member | the account |
| `POST /api/me/password` | a member | `{current, new}`, at least 8 characters |
| `GET /api/tokens` | a member | their tokens, never the values |
| `POST /api/tokens` | a member, by session | `{name}` → `{id, token}`, shown once |
| `DELETE /api/tokens/{id}` | a member | `404` for an id that is not theirs |

An unknown email and a wrong password get the same answer in the same time.

### The login's brake

The login is public the moment a tunnel is up, and accounts hold paid credits
and people's files, so guessing is braked (#557). The code is
`crates/server/src/accounts/throttle/`; this is the whole of it.

**Every attempt is counted twice — against the email it names and against the
address it came from — and either can refuse it.** The email's counter stops a
patient attacker spread over many addresses; the address's stops one source
spraying guesses across many emails, which counting per email never sees. The
rules are the same for both:

- **Ten attempts in fifteen minutes**, then `429` with `Retry-After` and
  `{"error": "too many login attempts; try again in 15 minutes"}` — which the
  login page shows as it stands, like any refusal. Ten is room for a typo, the
  old password and the one before it, and for a household behind one address.
- **The lock-out is fifteen minutes and doubles each time it recurs** — thirty,
  an hour, two… up to a day. A fixed lock-out only sets a rate, and ten guesses
  every fifteen minutes is still nearly a thousand a day; doubling holds a
  patient attacker to about seventy over the first day and a half and ten a day
  after that — far inside NIST SP 800-63B's hundred. **A quiet day** after the
  last lock-out ends forgives it, and the counter is forgotten.
- **Counted before the password is checked**, so a burst of simultaneous
  guesses is counted as it arrives; a success gives its attempt back. A
  success **clears the email's counter** — its owner has just proved who they
  are — but only gives the address its one attempt back, so logging in to an
  account of one's own between guesses at others' buys nothing.
- **All or nothing**: an attempt one counter refuses is not counted by the
  other, so a locked address cannot run up a stranger's email.
- **It says nothing about who has an account.** A counter is kept for whatever
  was typed, real account or not, and nothing in the brake looks at `users`;
  the refusal is word for word the same, and comes before argon2 for every
  email alike, so the timing rule above holds on both sides of it.

The cost, stated: somebody who knows your email can keep your account locked
by guessing at it. At friends-and-family scale that is a conversation with the
operator, who lifts it with `user unlock`; and **`user reset-password` lifts
the lock on that email itself**, since whoever gets a new password is about to
type it.

**The counters live in Postgres** (`login_throttle`), not in the server's
memory as web MCP's per-minute limit does. One process on one machine could
keep a map — but the operator's `user locks` and `user unlock` run as a second
process that could neither see nor clear it; every restart, which a crash or a
deploy is, would forgive every lock-out; and the price is two indexed rows per
attempt. The table is not per user — an attempt is nobody's until it succeeds —
so it joins `tests/isolation.rs`'s exemptions, argued there,
and members may not read it at all.

**Which address.** The server's peer is always nginx, so it cannot use the
connection's address, and `CF-Connecting-IP` is a header anybody can type. The
answer is that **nginx has two doors** (`deploy/nginx.conf`):

| door | who comes in by it | the address used |
| --- | --- | --- |
| `web:80` | only the tunnel: published to nothing, it is what both `cloudflared`s point at | `CF-Connecting-IP`, which Cloudflare writes itself |
| `web:8080` | loopback, `tailscale serve`, the LAN: what compose publishes as `$SCORSESE_WEB_PORT` | the connection's own; `CF-Connecting-IP` is ignored |

nginx sends its answer as `X-Scorsese-Client`, overwriting anything a client
sent by that name, and the server believes that header only because the
compose file sets **`SCORSESE_TRUST_PROXY=true`** — safe there because the
server's port is published to nothing. Anywhere else the setting stays unset
and the server counts the connection's own address. So a friend on the tailnet
who sends `CF-Connecting-IP: 203.0.113.9` is counted as themselves: they can
neither dodge their own limit nor aim it at somebody else. The door is a port
rather than an address range because container addresses are Docker's to
hand out, and a rule written against them would break silently, in the
direction of believing a header it should not.

What that means on each path: behind **Cloudflare**, every visitor is their
own address (an IPv6 one by its `/64`, which is what one subscriber is
handed). On the **tailnet and loopback**, every request reaches nginx through
Docker's port forwarding from the same bridge address, so all of them share
**one** address counter — invitation-only people, and the email counters still
hold each account. On the **LAN**, each device is its own address.

Not in the brake: CAPTCHAs, alerts by email, and limits on API or MCP calls
(web MCP has its own, per user).

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
   whose a cookie or token is, logging in, operator commands. Keep that list
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

Not in v1: password-reset email, OAuth, public sign-up.

## Jobs

Long work — a render, a Veo shot, a spoken line, a thumbnail, a proxy — is a
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

**Concurrency is per kind**, declared beside each kind in `jobs/kinds.rs`:

| kind | at once | why |
| --- | --- | --- |
| `render` | 2 | compositor and encoder each take several of the four cores |
| `preview` | 1 | a render too, but small, and superseded by the next edit — its own kind so a preview never holds a slot a finished render is waiting for |
| `proxy` | 1 | a whole transcode, as heavy as a render |
| `thumbnail` | 2 | one decoded frame |
| `veo_shot` | 4 | minutes of waiting on Google, almost no machine |
| `spoken_line` | 4 | seconds, mostly network |
| `still_image` | 4 | a Gemini still (#461): seconds, mostly network |
| `voice_design` | 4 | an ElevenLabs voice design (#572): seconds, mostly network |
| `voice_keep` | 4 | a designed candidate kept as a voice (#572): one free call |

**Every kind has a handler** — `thumbnail` and `proxy` (#535, #542, *Library*),
`render` and `preview` (#541, #542, *Renders*), `veo_shot`, `still_image` and
`spoken_line` (#539, #461, *Web MCP*, below), `voice_design` and `voice_keep`
(#572, *Designing a voice*, below). A paid generation pays through credits (*Credits*,
below: a failed one is free) and keeps what it made in the library. Each
registers its handler in `jobs::kinds::registry()`; a kind nothing registers
waits rather than failing. The worker, the claim, recovery and
the event stream are exercised end to end by test handlers, one of which stands
in for Veo.

**After a crash.** One worker per database, held by a Postgres advisory lock,
so a job found `running` when the worker starts belongs to a dead process. It
goes back to `waiting`, stamped `interrupted_at`; after three interruptions it
is given up on — `failed`, or `stuck` if it holds a ticket. A graceful stop
(`docker compose stop`, a deploy) takes the same path, so the crash path runs
on every deploy. Who a power cut affected:

    docker compose exec scorsese-server scorsese-server job interrupted

**Veo: never pay twice.** The rule `scorsese_providers::video` keeps for a
local project, kept here:

- The moment Google accepts a shot, its operation ticket is committed to the
  job's row (`Context::keep_ticket`), before the handler does anything else.
- A job that comes back with a ticket **polls it and never submits again**:
  Google keeps generating and bills either way, and the video stays fetchable
  for two days.
- A Veo job polls for up to **15 minutes** (`PROVIDER_PATIENCE`) — shots have
  taken ten — then marks the job `stuck`, ticket kept for a later collect.

**Live state** reaches the browser over **`GET /api/events`**, server-sent
events, one stream per user carrying everything live: each message is a JSON
object with a `type` — `job`, and the assistant's (*Assistant turns*).
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

**Recipes and the script live in `project_files`** (#560): one row per file,
keyed by (project, project-relative path), holding its text — every file under
`recipes/`, plus the script and any recipe the document names elsewhere; never
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
| `POST /api/projects` | a member | `{name, fps?}` → a new empty project, `201` |
| `GET /api/projects/{id}` | a member | the project, its `document` and `revision` |
| `PUT /api/projects/{id}` | a member | `{revision, document}` → `{revision}`; `409` if it moved on, `400` for a document this build does not read |
| `PATCH /api/projects/{id}` | a member | `{name}` → `{revision}` |
| `DELETE /api/projects/{id}` | a member | `204`; `404` for an id that is not theirs |

Deliberately storage verbs only: what a project *says* is changed by `core`'s
editing functions, through the tools (*Web MCP*, #540) and the editor (#545).
Recipes and the script are read and written through the tools too
(`synth_*`, `script_read`, `script_write`); there is no route for them and no
recipe editor in the pages.

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
| `top_up` | + | money the operator received (later, Pix) |
| `reservation` | − | a paid generation's price, held while the provider works |
| `release` | + | a reservation given back — the provider answered |
| `charge` | − | what a generation that worked, or an assistant call, costs |
| `monthly_fee` | − | the $10 for one month of an account's life |
| `refund` | + | money given back, with the reason |

**Pricing** is the provider's cost plus 10%, rounded up to the micro-dollar.
Veo and ElevenLabs are priced by `scorsese_providers::prices` — the same cents
the quote (#538) showed, converted at the server's boundary; the assistant by
`prices::claude` from the token counts its response reports, which is exact.

**Spending a generation is reserve, then settle.** `credits::generations::start`
prices it, takes a lock on the user's own row (so two spends at once cannot
both see the last dollar), and reserves — or refuses, writing nothing, when the
balance cannot cover it. When the provider answers, `finish` releases the
reservation and, if the generation **worked**, charges it — liked or not. A
generation the provider **failed** nets to zero: free. A unique index allows
one release per reservation, so nothing is settled twice. A Veo job that goes
`stuck` settles nothing and its reservation stays held, since Google may still
be billing. The assistant is charged after each call rather than reserved for:
its cost exists only once the tokens are counted, so a turn checks the balance
is positive before starting and the call may dip a little below zero.

**Audit tables.** `veo_generations` (model, resolution, seconds, aspect,
prompt, brief hash, operation ticket, state, estimated cost, error),
`image_generations` (model, resolution, aspect, how many references, prompt,
brief hash, state, estimated cost, error — #461, no ticket: a still comes back
on the call), `speech_generations` (model, voice, text, characters,
settings, estimated cost, error) and `voice_designs` (description, passage,
characters, seed, guidance, brief hash, state, the three candidates once it
worked, estimated cost, error — #572, *Web MCP*) hold one row per paid generation, bound to the ledger entries that paid
for it. Each carries a nullable `tool_call_id`, whose foreign key lands with the
table it points at (#540), a nullable `library_item_id` keyed to the item it
made by (item, owner) — kept, set null, when the item is deleted (#535) — and a
`project_id` that deliberately has none: a project can be deleted, and the
record of what it cost cannot.

**The monthly fee is a sweep in the server**, hourly, not a queued job: the
queue is for long work with a handler and crash recovery, and a fee is one
idempotent insert whose unique (user, month) index makes charging twice
impossible. A month is owed for each month of an account's life **counted from
its first top-up**, so an account nobody paid for owes nothing. A fee is charged
**only when the balance covers it** (credits are prepaid; the ledger never
lends), retried by every sweep while its month lasts, and not charged after
the month ends uncovered. Both are the conservative reading of what #527 left
open, asked on #537.

**The display rate** (`display_rates`) is reais per dollar, set by the operator,
dated. Balances show as "≈ R$ …" at the newest one, with dollars beside,
because the ledger is in dollars and the dollar moves. It is the one table that
is nobody's in particular: members read it and write nothing.

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
| `GET /api/credits` | a member | their balance, in micro-dollars and ≈ centavos, and the rate |
| `GET /api/credits/history` | a member | `?project=&kind=&since=&until=&before=&limit=` |

The operator's side, where the server runs:

```text
scorsese-server credit top-up ana@example.com --reais 100 --rate 5.4321
scorsese-server credit refund ana@example.com --dollars 1.06 --reason "…"
scorsese-server credit rate 5.43        # the display rate, as of now
scorsese-server credit balance ana@example.com
```

A top-up records the reais received and the rate they were converted at, and
credits the dollars **rounded down** — the one place that direction is right.

The Veo and ElevenLabs job handlers that call `start` and `finish` are web
MCP's (*Web MCP*, below), which also registers `spending_history`. Not here
yet: Pix (#548); the refund policy's text (#547).

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

**Exactly the kinds `scorsese import` takes** — video, image, audio, by the same
extension list (`scorsese_core::pool::infer_kind`), refused at announcement
(`415`) before a byte is sent. On arrival the server **hashes the bytes itself**
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
`project_assets` and `template_assets` hold it if two requests race. Nothing
removes a file from a *local* project (#396); this does not decide that.

**Generated output is an item too**, carrying the hash of its brief.
`Library::find_generated` finds it again for the **same user** — scoped, so
never across users — and `Library::keep_generated` keeps what a generation
made. Its details carry the generation record (#537): the brief, model,
settings, when, and what it cost — `estimated_cost_micros` by scorsese's own
table, `charged_micros` from the ledger. The generation jobs that call these are web
MCP's (*Web MCP*).

| route | who | what |
| --- | --- | --- |
| `GET /api/library` | a member | their files, newest first: `id`, `name`, `kind`, `size_bytes`, `thumbnail`; `?kind=`, `?search=`, `?sha256=` and `?project=` (the files that project uses) narrow it |
| `GET /api/library/{id}` | a member | everything known, `used_by` (projects), `templates`, `generation` (or `null`) |
| `PATCH /api/library/{id}` | a member | `{name?, description?}`; an empty description removes it |
| `DELETE /api/library/{id}` | a member | `204`; `409` with `projects` and `templates` when one uses it |
| `GET /api/library/{id}/file` | a member | the file, whole or in the range asked for |
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
| `POST /api/projects/{id}/renders` | a member | `{container?, video_codec?, audio_codec?, resolution?}` → `200 {render}` when kept, `202 {job}` when queued or already on its way; `400` for a shape `docs/output-formats.md` does not allow |
| `GET /api/projects/{id}/renders` | a member | the project's kept renders, most recently used first |
| `GET /api/renders/{id}/file` | a member | the file as an attachment, in HTTP ranges; counts as use |

The job's progress arrives on `GET /api/events` like any job's; its result
names the render and where to download it.

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
registry tool is decided about in `tools/surface/` (one name a line, and the lists below are generated from it by `make mcp-table`), so a new one cannot reach
the web — or be left off it — without a reason written down.

| served | how |
| --- | --- |
| the *document tools*, listed below | as they are, on the stored project |
| the *project-file tools*, listed below | as they are; the script and recipes they read and write are the project's `project_files` (*Projects*). A script or recipe written under `assets/`, `generated/` or `cache/` is refused whole, since nothing there is kept |
| `synth_bake` | without `out`; each new bake is **kept in the library** as a generation, its address (recipe and synthesiser) as its brief hash, before the document naming it is saved — so it renders, and is linked into every later layout by hash. A partial bake's file is gone with the folder; its report is in the reply |
| `look`, `hear`, `audio_level` | their file arguments must be paths inside the project (`assets/…`, `generated/…`) — locally they may name anything on the machine, and here the machine is everybody's |
| `still` | without `out`: nothing is kept on the server's disk; the picture is in the reply |
| `project_list`, `project_new` | the server's own: a project is a row, named by an id the client asks for |
| `library`, `import` | the server's own: files come from the user's library by id (`core`'s `reference_asset`, the document half of an import), never from a path on the server — so an image sequence's stills come in one library file each, and `sequence` makes them one |
| `render`, `jobs`, `job_cancel` | the server's own: a render is a job (*Renders*), downloaded from `/api/renders/{id}/file` with the same token; `jobs` says where any job is, and `job_cancel` stops a render — locally a client stops one by cancelling the `render` call, which here has already answered |
| `generate` | the server's own: paid from credits, made by the queue — below |
| `voice_design` | the server's own: paid from credits, made by the queue, the samples kept in the library and the voices in the user's own record — *Designing a voice*, below |
| `spending_history` | the ledger, read for the caller (*Credits*) |
| `template_list`, `template_save`, `template_insert` | the server's own: a user's templates are rows (*Templates*) |

**The document tools**, served as they are on the stored project:

<!-- BEGIN STORED. Generated from `tools/surface` by `make mcp-table`; edit the list there, not here. -->

- `project_read`
- `project_describe`
- `project_check`
- `project_assets`
- `project_probe`
- `project_write`
- `track_new`
- `text_new`
- `color_new`
- `shape_new`
- `icon_new`
- `asset_set`
- `sequence`
- `asset_remove`
- `track_remove`
- `place_clip`
- `trim_clip`
- `clip_set`
- `clip_animate`
- `clip_follow`
- `clip_move`
- `clip_remove`
- `clip_group`
- `clip_ungroup`
- `dissolve`
- `duck_music`
- `set_volume`
- `scale_pacing`
- `rebrief`
- `icons`
- `voices`

<!-- END STORED -->

**The project-file tools**, served as they are on the project's script and
recipes:

<!-- BEGIN PROJECT_FILES. Generated from `tools/surface` by `make mcp-table`; edit the list there, not here. -->

- `script_read`
- `script_write`
- `synth_new`
- `synth_kit`
- `synth_read`
- `synth_write`
- `synth_set`
- `synth_check`
- `synth_survey`

<!-- END PROJECT_FILES -->

**Not served yet:** `synth_import` and `synth_export` — a `.mid` is neither
media the library holds nor text a project keeps, so there is nothing to import
from or to hand an export back as (#678).

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
nothing, and so is one whose job is still on its way. A call without `confirm`
answers with the quote, what it takes from the balance (cost + 10%) and a
token, kept in the `quotes` table under the providers' rules (#538: bound to
what was quoted, once, fifteen minutes). A call with the token reserves every
charged brief and queues it as a `veo_shot`, `still_image` or `spoken_line` job in one
transaction — all, or none when the balance cannot cover the lot — and answers
at once with the job ids. Each job (`crate::generations`) sends the brief
exactly as quoted, keeps a shot's ticket before anything else, keeps what came
back with `Library::keep_generated`, charges it — or releases it, free, when the
provider refused — and brings it into the project as it is by then, measured.
The provider keys are the server's: `GEMINI_API_KEY` and `ELEVENLABS_API_KEY`,
through the one credentials resolver; a missing key fails the job, free.

**Designing a voice (#572).** `voice_design` designs, keeps and lists as the
stdio tool does, and pays like `generate`: a call without `confirm` answers with
the estimate (`voices::design::estimate`, billed once for the passage, three
candidates) plus 10% and a token; a call with the token reserves it and queues
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
  description and seed that made it.

Both verbs are jobs. A design is paid, and a job interrupted after the
reservation is run again and settles it, where a call cut off mid-way would
leave the money held; keeping is free, but it reaches the same vendor, so the
vendor stays behind one seam (`generations::Vendors::studio`) a test replaces.
`keep` and `list` take no token. Voice cloning is not offered, as locally.

**Every call is recorded** in `tool_calls` — tool, arguments, project, how it
ended and its words (not its pictures), with `client` `external` for web MCP,
`assistant` for the built-in assistant, `user` for the user's own yes to a
quote the assistant showed them, and `editor` for an edit made by hand in the
web editor (*The editor*); `assistant` and `user` name their chat turn and their
place in it. A paid generation's audit row names the call that asked for it,
so *prompt → turn → tool call → generation → credits* reads back whole.

**Rate limit:** 120 tool calls a minute per user, the rest `429` with
`Retry-After`. Spending already needs a quote and a yes; this keeps one
runaway loop from filling the queue on a shared machine.

**The web editor (#545) calls the same path**: a thin JSON route over the
toolbox for a browser session (*The editor*), so no edit is written twice —
once in Rust and again in TypeScript.

## Assistant turns

The built-in assistant (#540): Claude Opus 5.5 editing a user's project with
the tools web MCP serves, while their browser watches. The code is
`crates/server/src/assistant/` (the turn, the conversation, the quote box),
`crates/server/src/http/chat.rs` (the routes) and, for the model itself,
`crates/providers/src/claude/` over the wire in `api/anthropic/`; their
module docs carry each argument. **The chat panel is #545's**; this section is
the API it calls.

**A turn** is one message from the user and everything the assistant does
with it: call Claude with the conversation and every tool `Toolbox::listing`
serves, run the tools it asks for — in-process, as `Client::Assistant`, in
order — send their results back, and loop until it answers. Each project has
conversations (`chat_sessions`); a turn joins the newest unless it asks for a
fresh one, and one turn runs at a time per conversation. The system prompt asks
for a short progress line before each step and one full summary at the end,
and to lay a cut out as free sketches before spending on generation.

**The conversation is append-only, stored as the text that was sent.** A
turn's Messages API messages are kept in `chat_turns.messages` as the exact
JSON text first sent — `TEXT`, not `JSONB`, which would reorder keys — and the
next turn resends every earlier turn's messages unchanged. On this model a
thinking block is valid only while everything before it is byte-for-byte what
it was, and an unchanged prefix is also what the prompt cache reads. What the
server vouches for — which project this is, that the user confirmed a quote —
is a mid-conversation `system` message, which no user text or tool output can
forge. A turn cut off before the model replied (a refusal, a stop, a restart)
is closed by a one-line reply at the start of the next, and a tool call it
never ran is answered there as not run.

**Paid tools: the user's yes, never the model's.** `generate` quotes first
and spends only when called again with the quote's token (*Web MCP*). The
model never sees a token: when a call issues a quote, the token's line is cut
from what the model reads, the quote is held on the turn and sent to the
browser as `chat_quote`, and any call naming `confirm` is refused without
running. The user's answer is `POST /api/chat/turns/{id}/quote`: yes makes the
paid call itself — recorded in `tool_calls` as client `user` — and starts a
turn that tells the model, as a `system` message, what it spent; no withdraws
the token and the next turn is told. Writing a new message instead withdraws
it too. A quote is answered once.

**Money.** Every call to Claude is charged from its reply's `usage` — input,
output, five-minute and one-hour cache writes, cache reads, each at its own
rate in `prices::claude` — plus 10%, as one `charge` entry naming the turn
(`credit_entries.chat_turn_id`); the spending history folds a turn's calls into
one row. Not reserved for, since the cost exists only once counted: a turn is
**refused up front (`402`) at a balance of zero or less**, and stops between
calls once the balance runs out or the turn reaches the operator's cap
(`SCORSESE_ASSISTANT_TURN_CAP`, default $2.00). The call that crosses either
line was already made and is charged; one call is bounded by `max_tokens`.

**Caching** is where the money is, since every call resends the conversation:
one breakpoint on the system prompt caches the tools and the prompt together
for an hour — identical for every user, so one write serves the whole server —
and the API's automatic breakpoint caches the conversation's tail for five
minutes. Nothing about a user or the time goes in the prefix.

**Effort is `high`** and the model is not downgraded (`CLAUDE.md`). The model
id is `SCORSESE_ASSISTANT_MODEL`, refused at startup unless the price table
has a rate for it. A refusal by Anthropic's safety classifiers ends the turn as
`refused`, charged for what the call used; it is not retried on another model,
because the price table has one row and the maintainer chose the model.

**Without `ANTHROPIC_API_KEY`** every turn is refused with `503` and "not
configured", and the rest of the server runs as it did.

| route | who | what |
| --- | --- | --- |
| `GET /api/projects/{id}/chat` | a member | `{project, session, turns}`: the newest conversation's turns, oldest first |
| `POST /api/projects/{id}/chat` | a member | `{prompt, fresh?}` → `202` with the turn; `402` no credit, `409` a turn is running, `503` not configured |
| `GET /api/chat/turns/{id}` | a member | `{turn, tools}`: the turn and the log of every tool call it made, in order |
| `POST /api/chat/turns/{id}/stop` | a member | `202`; the turn stops before its next step. `409` if it is not running |
| `POST /api/chat/turns/{id}/quote` | a member | `{confirm: true\|false}` → `{spent, refused, turn, note}`; `turn` is the one carrying on after a yes |

A turn (`TurnView`) carries its state — `running`, then `answered`, `refused`,
`capped`, `stopped`, `failed` or `interrupted` (the server stopped under it) —
its `answer`, the model, token totals, `charged_micros`, and `quote`
(`{tool, lines, micros, expires_at}`) with `quote_answer` (`null` while the box
should show, then `confirmed`, `declined` or `withdrawn`).

**On `GET /api/events`**, beside `job`:

| `type` | fields | what the panel does with it |
| --- | --- | --- |
| `chat_turn` | `turn`, `balance_micros` | a turn started, was charged for a call, or ended: replace it, show what it cost and the balance |
| `chat_text` | `turn`, `text` | more of the words the assistant is writing: append |
| `chat_progress` | `turn`, `text` | a whole progress note between tool calls: one status line |
| `chat_tool` | `turn`, `tool`, `state`, `said` | a tool `running`, then `answered` or `refused`, with the start of its answer |
| `chat_quote` | `turn`, `quote` | show the confirmation box |
| `project` | `id`, `revision` | the assistant changed the project: re-read it, and the preview refreshes |

After `resync`, or on reconnecting, re-read `GET /api/projects/{id}/chat`.

Not here yet: compacting a conversation that outgrows the context window (a
fresh conversation is the answer for now), and a per-user daily limit.

## The pages

What a user sees before the editor (#544); `web/README.md` has how the code is
laid out.

| URL | what |
| --- | --- |
| `/login` | email and password; there is no sign-up, so the page says accounts are by invitation |
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

**Money on a page** is always the server's integers. A balance and a history
row arrive in micro-dollars and, once the operator has set a rate, in centavos;
the page shows "≈ R$" with dollars beside and never converts a balance itself.
The one conversion it does — a generation record's cost, which the server
sends in dollars only — uses the rate from `GET /api/credits` and the server's
own rounding.

**Adding a library file to a project is not a button here.** The server keeps
no per-edit endpoints (`http/projects.rs`): what a project says changes through
`core`'s editing functions, reached by the tools and the editor. Bringing a
library file into a project is web MCP's `import` (*Web MCP*), and the editor's
(#545) by the same path.

## The editor

`/projects/{id}/edit` (#545), laid out as the desktop app is: the project's
assets and the user's library on the left, the preview in the middle, the
timeline under both, and on the right the selected clip's inspector over the
assistant's chat panel. The code is `web/src/editor/`, the route
`crates/server/src/http/editor.rs`; their module docs carry each argument. It
is a first version by the rule in `CLAUDE.md` — *the user can start editing
with it* — and is meant to be tuned from use.

**The hand-edits are few, and each is a tool call.** Add a track, drop a file
on it, drag a clip along it or onto another lane, drag an edge to trim, press
Delete on the selected clip, and type a plain value into the inspector —
`track_new`, `import` then `place_clip`, `trim_clip`, `clip_move`,
`clip_remove`, `clip_set` — through `POST /api/projects/{id}/tools/{name}`,
which runs the toolbox web MCP and the assistant run, recorded as client
`editor`. The browser reads `project.json` to draw the timeline and never writes
it. Anything with structure to it — a title, a dissolve, a ramp — is a sentence
to the assistant, not a menu (`CLAUDE.md`, *The GUI is thin*).

| route | who | what |
| --- | --- | --- |
| `POST /api/projects/{id}/tools/{name}` | a member, **by session** | `{arguments, revision?}` → `{said, project}`: the tool's words and pictures, and the project as it is now (`null` after a `still`) |

- **An allowlist**, not the whole surface: `track_new`, `track_remove`,
  `asset_remove`, `place_clip`, `trim_clip`, `clip_set`, `clip_move`,
  `clip_remove` (the edits) and `import`,
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
is left, not per keystroke.

**`clip_set`** is the registry tool this added: speed (retimed, as a 2× button
means), fit, and position, rotation and scale as single held values —
`docs/mcp.md` has it. The assistant and web MCP get it too. **`clip_move`** and
**`clip_remove`** came after (#576), for the same reason: a clip dropped on the
wrong lane had nowhere to go but the assistant. A drag that ends on another
lane is one `clip_move` with the new start, never a move then a trim; Delete or
Backspace removes the selected clip — its asset stays, and nothing closes up
behind it. Neither fires while a text field has the keyboard.

**`asset_remove`** and **`track_remove`** (#396) are the bin beside an asset in
the assets panel and on a lane's header. Each is a plain confirm first, listing
the clips that go with it — the timeline is already in the page — and *yes*
sends exactly that list. The tools refuse any other list, so a timeline that
moved on since the confirm was drawn is a refusal, never a guess.

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
  the chosen shape (1920x1080 for 16:9), chosen beside the transport and said
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

The frame's **shape** (16:9, 9:16, 1:1) is chosen in the header and remembered
per project in the browser: a project has no aspect of its own, a render's
resolution is its aspect. It sets the preview raster and the sizes the **Render**
dialog offers; that dialog asks for a finished render, follows it through the
queue and lists the project's kept renders to download.

**The chat panel** is *Assistant turns* on screen: each turn's prompt, its
progress lines and tool calls as they stream, its answer, what it cost and the
balance after it, the quote confirmation box, Stop while it runs, and the
generation jobs it queued (queued → generating → ready). Without an Anthropic
key the server's `503` "not configured" is shown as a note, and nothing else
changes.

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
