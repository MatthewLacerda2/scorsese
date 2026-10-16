# crates/core

Rules for working inside `core`. The root `CLAUDE.md` still applies; this file
holds what only matters here.

## A schema bump ships with a migration

`project.json` format changes are `architecture`-label work and require a
`schema_version` bump. The format is the contract between the CLI, the MCP
server, the GUI, the web app and every project already stored — the contract
*now*, not across time.

This used to read *there is no backwards compatibility*, and said the day
somebody had a project they could not afford to lose was the day to revisit it
— and that it was the user's call. The user made it on 2026-09-25: other
people's projects now live in Postgres (#534), and a bump that makes them
unloadable breaks paying users. So:

- **Every `schema_version` bump carries a migration** from the previous
  version to the new one, in the same pull request, written here as a step
  over the JSON document (`vN-1 → vN`). Steps chain, so a document several
  versions behind walks forward one step at a time. The migration is what is
  compatible — **`Project::load` still refuses any version that is not this
  build's**, so the bump still turns a silent reinterpretation into a loud
  refusal, and the only way past the refusal is the migration.
- **The server migrates every stored document** — projects and templates
  (`crate::template`) alike — when it starts on a new build, before serving a
  request — a stored project is never read by code
  that does not understand it.
- **Local `.scor` folders are migrated by the same steps, through the CLI**:
  one command that rewrites a folder's `project.json` to this build's
  version. One implementation, two callers; the desktop and CLI user is the
  maintainer, and a separate path for them is a second thing to get wrong.
- Each step has a test: a document at the old version, migrated, loads and
  validates. A step that cannot be written — a change whose old meaning has
  no new equivalent — is a question for the user **before** the bump, not
  after it.

**Writing one**, in `src/migrate/`:

- The step goes in `STEPS` (`migrate/mod.rs`). It changes the content and
  never sets `schema_version` itself — `walk` does that afterwards. A bump
  that changes nothing stored still gets a step, one that returns `Ok(())`.
  `every_version_since_the_oldest_has_exactly_one_step` fails the build
  otherwise.
- Its test goes in `migrate/versions.rs` (or beside the step, when it has a
  folder of its own).
- `OLDEST_MIGRATABLE` (33) is the floor: older documents are refused as
  `MigrateError::TooOld`. It moves back never, and forward only if the user
  decides.
- **The bump records the new format.** `crates/core/tests/format` holds
  `project.json`'s shape — one project with every field set, and every enum
  variant as written — against `recorded.json`, so a format change without
  a bump fails there, saying what changed. After the step is in, `RECORD_FORMAT=1
  cargo test -p scorsese-core --test format` rewrites the record; it refuses
  to overwrite a record of the version it is already at. A field or variant
  added to the model does not compile until that test's document sets it.
- The bump also moves `docs/project-format.md`'s title (held to
  `SCHEMA_VERSION` by `the_title_states_this_builds_schema_version`), adds
  the step's row to its migration table (held to the same range by
  `tests/migration_table.rs`) and moves the
  `"schema_version"` in every `.json` fixture (`crates/golden/fixtures`,
  `crates/core/tests/fixtures`, `web/landing`), and the web's copy of the
  constant, `SCHEMA_VERSION` in `web/src/api/document.ts`, which `make
  size` holds equal to this one. Change them last, to `main`'s version + 1.
  A test document never needs it: a Rust one interpolates `SCHEMA_VERSION`
  and a web one imports the web's copy, and `make size`'s `schema-literal`
  rule refuses one that writes the current version as a number (in `web/`,
  any number). A Rust document at an *older* version (a migration's input)
  keeps its literal; that one is meant.

Nothing else is compatibility: no reading an older version in place, no field
kept alive because something might still write it. The migration is the whole
of it, and it is what makes the rest unnecessary. Two branches that both bump
the version are one number too few: the `ci-merge` skill has the rebase.
