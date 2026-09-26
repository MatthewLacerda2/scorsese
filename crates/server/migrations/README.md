# Migrations

The server's schema, one SQL file per change, applied in order every time
`scorsese-server` starts and compiled into the binary.

- **`NNNN_what_it_does.sql`** — four digits, one after the last, no gaps.
- **Never edit one that has merged.** Write the next one.
- **Forward only** — there are no down migrations.
- **A per-user table** carries `user_id … ON DELETE CASCADE`, row-level
  security and a `member_id()` policy, like `0001_accounts.sql` — the rule is
  in `crates/server/src/db/scope.rs`, and `tests/isolation.rs` enforces it.

The reasons are in the module doc of `crates/server/src/db.rs`, and
`crates/server/tests/migrations.rs` holds this directory to the naming rule:
sqlx silently skips a file whose name it cannot parse, so a typo here would
otherwise be a table that never gets created.

This file is the only thing here that is not a migration, and sqlx ignores it
for the same reason.
