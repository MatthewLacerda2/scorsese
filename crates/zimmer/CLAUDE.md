# crates/zimmer

Rules for working inside `zimmer`. The root `CLAUDE.md` still applies; this
file holds what only matters here.

## The name

**`zimmer` is ours, and its name is the joke told twice.** The project is named
for Martin Scorsese; the crate that writes the music is named for Hans Zimmer,
surname only both times. It is not a vendor, not a third-party dependency and
not an acronym. The name deliberately says nothing about what the crate does —
the first line of its own `lib.rs` doc does that instead (*sound from a
document*), and `missing_docs` is a merge gate, so that line cannot quietly go
missing. **The rename stops at the crate boundary**: `scorsese synth`, the
`synth_*` MCP tools and `"kind": "synth_audio"` all stay, because an
agent-facing surface has to describe itself and the asset kind is the format
contract.

## Its second consumer: rusty

**`zimmer` has a second consumer: [rusty](https://github.com/MatthewLacerda2/rusty)**,
the owner's game engine, which depends on it as a git dependency pinned to a
commit (rusty#413) instead of keeping its own copy. Three things follow. Its
**public API is rusty's contract** — renaming, removing or narrowing anything
`pub` still lands, but breaks rusty the next time it moves its pin, so the pull
request that does it says so. **`SYNTH_VERSION` governs rusty's bakes too** —
the same rule, one more reader. And **game-facing needs are in scope** for
`zimmer`, inside its no-I/O boundary: a game loops music forever where a video
never does, and the north star's "must add to this vision" does not exclude
them. None of this makes it a vendor here; everything above still holds.

## `SYNTH_VERSION`

**A change to what a recipe renders to requires a `SYNTH_VERSION` bump**, in
the same commit, for the same reason a format change requires a
`schema_version` bump: breaking loudly is the point. A bake in `generated/`
is addressed by a hash of the recipe **and** that number, so bumping it is
what makes every affected file miss the cache and be re-rendered — and not
bumping it leaves every project on disk holding audio its own recipe no
longer describes, silently. The number is declared rather than derived
because deriving it means hashing rendered output, and a digest has no
tolerance to spend on a platform's `sin` and `exp` differing; the constant's
own doc in `src/lib.rs` carries the whole argument. So: touch a source, a
filter, an envelope or an effect and the samples move — bump it. Touch only
prose, a name or a document type — leave it alone.

**Verify by rendering only when the change touches rendering maths.** Baking a
probe corpus against two checkouts settles a genuine doubt — an edited note
loop, a shared helper moved, a stage reordered — and is waste when the diff
already answers it: a new optional field defaulting to old behaviour cannot
move an existing recipe's bytes. Say so in one line and move on. The
constant's own doc records what previous branches checked, and why; the
`ci-merge` skill has how to run probes and what a conflict on the number means.
