# The repository as a graph

A knowledge graph of this repository, built by
[graphify](https://github.com/safishamsi/graphify) from the tree at
`50f193c`. **6,468 nodes, 13,954 edges, 353 communities.**

It exists to answer questions about the codebase without reading the codebase.
`graphify benchmark` puts the corpus at ~431,200 tokens read naively against
~9,123 for an average query against the graph — 47× — which is the whole point
for an agent that has to orient in a repo of 587 Rust files before it can
change one of them.

## What is here

| file | what it is |
|---|---|
| `graph.json` | the graph itself, node-link JSON. This is the artifact everything else is derived from, and the one `graphify query` reads. |
| `graph.html` | interactive view, self-contained, open it in a browser. Above 5,000 nodes graphify aggregates, so this shows the 353 communities and the 1,137 edges between them, not every node. |
| `GRAPH_REPORT.md` | the audit trail: god nodes, import cycles, community cohesion scores, and which edges were extracted versus inferred. |
| `.graphify_labels.json` | the community names, kept separate so relabelling does not rebuild. |
| `cost.json` | what the build cost in tokens. |

## Regenerating it

```sh
graphify .                    # full rebuild
graphify . --update           # re-extract only what changed
graphify query "how does the render crate decide an output format?"
graphify path "Asset" "Compositor"
graphify explain "Frames"
```

Rust is extracted structurally, by AST — deterministic, and no model and no API
key are involved. Only the prose (`README.md`, `CLAUDE.md`, `docs/`,
`.github/workflows/`) goes through a model, which is where the concept nodes
and the stated-rationale attributes come from.

## What it is not

**It is a snapshot, and it goes stale.** Nothing regenerates it on merge; the
commit it was built from is recorded in `graph.json` under `built_at_commit`,
and comparing that to `HEAD` is the only honest way to know how far it has
drifted. Wiring it to a post-commit hook or a CI job is possible and is
deliberately not done here — that would be a gate nobody asked for.

**Some of the corpus was deliberately left out.** The 91 golden-fixture PNGs
under `crates/golden/fixtures/*/expected/` are rendered video frames, and
running vision over them buys nothing a human would not already know from the
fixture name. The 22 `expected/decoder.txt` files are one line of ffmpeg
version each, and `OFL.txt` is a font licence. Excluding them is a judgement
call, and reversing it is a flag away.

**1,464 edges have an endpoint that is not a node.** All of them come from the
AST pass, pointing at symbols outside the corpus — `Option`, `String`, and the
rest of std. None come from the prose extraction. They are not corruption, but
they are the reason a raw edge count reads higher than the graph's.

**45 of the 353 community names were written by hand**; the other 308 are
derived mechanically from the directory most of their nodes live in. A name
like `tests / pool` is a filing decision, not a claim about what the cluster
means.
