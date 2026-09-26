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

**It runs on the maintainer's machine in Docker Compose**, four service
containers and one that keeps them safe:

| container | role |
| --- | --- |
| `postgres` | the database, on a persistent volume |
| `scorsese-server` | the Rust server, with ffmpeg inside; library files, render cache and scratch mounted from host disk |
| `web` | nginx serving the built React files |
| `cloudflared` | a **Cloudflare Tunnel** — how the site reaches the internet without a static IP; it also terminates HTTPS, so there is no reverse proxy of our own |
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
feature*).

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
| `SCORSESE_BIND` | Where to listen. Defaults to `127.0.0.1:8080`, this machine only. |

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
never touches the development database. CI's `check` and `coverage` jobs run a
Postgres service container instead.

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
everything to `web`; nginx serves the React build and passes `/api/` to
`server`, unchanged. Nothing is published to the network: `web` also answers on
`127.0.0.1:$SCORSESE_WEB_PORT`, for checking the site from this machine.

**The `/api` split lives in nginx, not in the tunnel's ingress rules.** A tunnel
run by token keeps its rules in Cloudflare's dashboard, where no pull request
shows them and no local run exercises them; in `nginx.conf` they are versioned,
reviewed and identical on loopback and on the public hostname. nginx is needed
for the SPA fallback anyway, so the tunnel is a single rule that never changes.
The MCP endpoint (#539) is expected under `/api` like everything else; if it
lands elsewhere it is one more `location` block there.

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
dumps and the time of the last good backup. Anything rebuildable a later issue
adds — render cache, scratch — belongs beside them, never inside `library/`,
so it is not shipped off the machine every night.

### First-time setup

Steps 3 and 4 need the maintainer's own accounts; nothing in the repo can do
them. The rest is commands, from the checkout on the machine that serves.

1. **Docker starts at boot**: `sudo systemctl enable --now docker`. Without
   it, a power cut is an outage until someone logs in.
2. **The data directory**, owned by the user the containers run as — Compose
   refuses to create it, so a typo or an unmounted disk fails loudly:

       mkdir -p /path/to/scorsese-data/library /path/to/scorsese-data/backups

3. **The Cloudflare Tunnel.** The domain must be on Cloudflare (its
   nameservers pointed there). In the dashboard: *Zero Trust → Networks →
   Tunnels → Create a tunnel → Cloudflared*, name it, and copy the token from
   the install command shown (the string after `--token`); skip installing
   the connector, the `cloudflared` container is the connector. Then add **one
   public hostname** — the site's domain, service type `HTTP`, URL `web:80` —
   and no other rules. The token goes in `CLOUDFLARE_TUNNEL_TOKEN`.
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
   http://127.0.0.1:8088/api/health` answers `ok`, and so does the public
   hostname. `docker compose logs backup` shows the first backup reaching the
   remote.
8. **Restore once, on purpose** (below), into this fresh install. A backup
   that has never been restored is a hope.

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
docker compose exec scorsese-server scorsese-server user create ana@example.com
scorsese-server user reset-password ana@example.com   # also logs out every browser
scorsese-server user delete ana@example.com --yes     # rows and files, for good
scorsese-server user list
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
it is the CSRF defence. Server-side rather than signed, so logout, a reset and
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
| `POST /api/login` | anyone | `{email, password}` → the account, and the cookie |
| `POST /api/logout` | a member | ends the session |
| `GET /api/me` | a member | the account |
| `POST /api/me/password` | a member | `{current, new}`, at least 8 characters |
| `GET /api/tokens` | a member | their tokens, never the values |
| `POST /api/tokens` | a member, by session | `{name}` → `{id, token}`, shown once |
| `DELETE /api/tokens/{id}` | a member | `404` for an id that is not theirs |

An unknown email and a wrong password get the same answer in the same time.

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

**A user's files live under `$SCORSESE_STORAGE/users/<user id>/`** and
nowhere else, so deleting an account is one directory. Named by id, never
email: an email is personal data with no business in a path or a backup
listing. Deletion removes the rows first, then the directory; if the files
cannot all be removed, the command names the directory for the operator to
finish by hand.

Not in v1: a login rate limit (Cloudflare's rules sit in front, and argon2
makes each guess cost tens of milliseconds), password-reset email, OAuth,
public sign-up.

## Jobs

Long work — a render, a Veo shot, a spoken line, a thumbnail, a proxy — is a
row in the `jobs` table, run by a worker inside the server (#536). The code is
`crates/server/src/jobs/`, and its module doc carries the argument.

**States**: `waiting → running → done | failed | stuck`. `stuck` is a provider
job that outlasted its patience; it is not lost (below).

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
| `proxy` | 1 | a whole transcode, as heavy as a render |
| `thumbnail` | 2 | one decoded frame |
| `veo_shot` | 4 | minutes of waiting on Google, almost no machine |
| `spoken_line` | 4 | seconds, mostly network |

**No kind has a handler yet.** Rendering a stored project needs #534; a paid
generation needs credits, which #537 provides (*Credits*, below: a failed one
is free), and the library (#535) to put its result in. Each registers its handler in `jobs::kinds::registry()`; until then a
job of that kind waits rather than failing. The worker, the claim, recovery and
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
object with a `type` — `job` today, the assistant's (#540) as it adds them.
In memory and allowed to drop: a reader that falls behind gets `resync`, and
the answer to that, or to reconnecting, is to re-read `GET /api/jobs`. The
stream ends when the server stops, and `EventSource` reconnects by itself.

| route | who | what |
| --- | --- | --- |
| `GET /api/jobs` | a member | their last hundred jobs, newest first |
| `GET /api/jobs/{id}` | a member | one; `404` for one that is not theirs |
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
**(user, sha256)** — the library's own identity for a file — and has no foreign
key to the library yet: that table does not exist, and adding the key is #535's
job, with the table. A composite key to `projects (id, user_id)` stops a row
pointing at another user's project.

**Rendering a stored project** lays it out as a temporary `.scor` folder
(`projects::media::materialise`): the document written, each file **symlinked**
from where the user's storage keeps it, looked up **by hash** in that user's
storage and never by the document's path — so a document cannot point the server
at another user's file or at the host's. `render` and `compositor` run on it
unchanged; the folder is removed when dropped. Where it goes and what starts the
render are the job queue's (#536, #541).

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
editing functions, through the tools (#539, #540) and the editor (#545).
Recipes and a project's `script` are documents rather than media, and have
nowhere to live in the server yet; the materialiser does not lay them out.

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
prompt, brief hash, operation ticket, state, estimated cost, error) and
`speech_generations` (model, voice, text, characters, settings, estimated cost,
error) hold one row per paid generation, bound to the ledger entries that paid
for it. Each carries a nullable `tool_call_id` and `library_item_id`, whose
foreign keys land with the tables they point at (#540, #535), and a
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

Not here yet: the Veo and ElevenLabs job handlers that call `start` and
`finish` (they need the library, #535, and land with the issue that enqueues
generations, #539/#540); Pix (#548); the refund policy's text (#547).

## Out of scope for now

Pix payments (#548), folders in the library, a shared cross-user library,
collaboration, cloud hosting.
