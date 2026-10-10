# crates/core

Rules for working inside `core`. The root `CLAUDE.md` still applies; this file
holds what only matters here.

## A schema bump ships with a migration

`project.json` format changes are `architecture`-label work and require a
`schema_version` bump. The format is the contract between the CLI, the MCP
server and the GUI — the contract *now*, not across time.

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
- **The server migrates every stored document** when it starts on a new
  build, before serving a request — a stored project is never read by code
  that does not understand it.
- **Local `.scor` folders are migrated by the same steps, through the CLI**:
  one command that rewrites a folder's `project.json` to this build's
  version. One implementation, two callers; the desktop and CLI user is the
  maintainer, and a separate path for them is a second thing to get wrong.
- Each step has a test: a document at the old version, migrated, loads and
  validates. A step that cannot be written — a change whose old meaning has
  no new equivalent — is a question for the user **before** the bump, not
  after it.

Nothing else is compatibility: no reading an older version in place, no field
kept alive because something might still write it. The migration is the whole
of it, and it is what makes the rest unnecessary. Two branches that both bump
the version are one number too few: the `ci-merge` skill has the rebase.
