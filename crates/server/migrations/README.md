# Migrations

The server's schema, one SQL file per change, applied in order every time
`scorsese-server` starts and compiled into the binary.

- **`NNNN_what_it_does.sql`** — four digits, one after the last, no gaps.
- **Never edit one that has merged.** Write the next one. sqlx refuses to
  start against a database that applied a different version of a file, so an
  edit is a web app that does not come up after the deploy. `merged.txt`
  holds every migration's checksum, and `tests/migrations.rs` fails on any
  file that no longer matches its line.
- **A new migration appends its line to `merged.txt`** — the test prints it.
  The file is append-only: a line is never changed or removed.
- **Forward only** — there are no down migrations.
- **A per-user table** carries `user_id … ON DELETE CASCADE`, row-level
  security and a `member_id()` policy, like `0001_accounts.sql` — the rule is
  in `crates/server/src/db/scope.rs`, and `tests/isolation.rs` enforces it.

The reasons are in the module doc of `crates/server/src/db/mod.rs`, and
`crates/server/tests/migrations.rs` holds this directory to the naming rule:
sqlx silently skips a file whose name it cannot parse, so a typo here would
otherwise be a table that never gets created.

This file and `merged.txt` are the only things here that are not migrations;
sqlx skips any file not ending in `.sql`, so it runs neither.
