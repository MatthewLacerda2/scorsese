# The vendored icon set

[Lucide](https://lucide.dev), **pinned at 1.31.0** — the version in `VERSION`,
which is the one the blob was built from and the one the compositor reports.

```
LICENSE          upstream's, verbatim: ISC, plus the MIT section for the icons
                 that descend from Feather
VERSION          the pinned release, and the only place it is written down
lucide/          upstream's `icons/` directory, verbatim: 1767 `.svg` files and
                 the `.json` beside each one, which carries its tags, its
                 categories and any name it used to have
catalogue.bin    what the compositor actually compiles in
```

## Why the tree is here at all

`catalogue.bin` is generated, so the tree it is generated from has to be in the
repo for the conversion to be reproducible and for a bump to be reviewable. A
version bump is then two diffs that have to agree — the vendored files, and the
bytes they convert to — and `--check` below is how anyone confirms the second
follows from the first.

Upstream renames and retires icons, so this is pinned rather than tracked. A
bump is a deliberate change with a note in its pull request, never a re-sync.

## Re-vendoring

```sh
# the same release the repo was pinned at is tagged in the lucide repo, and its
# `icons/` directory is what goes into `lucide/`
curl -L https://github.com/lucide-icons/lucide/archive/refs/tags/1.31.0.tar.gz \
  | tar xz --strip-components=2 -C crates/compositor/icons/lucide '*/icons/*'

cargo run -p scorsese-icon-vendor            # rewrite catalogue.bin
cargo run -p scorsese-icon-vendor -- --check # confirm it is current
```

The source repository rather than the `lucide-static` npm package, and the
difference is the metadata: the npm build ships the SVGs and a merged
`tags.json` but no categories at all, while the repository ships a `.json` per
icon carrying both. Both are built from this same tag, and the SVGs are
identical.

`--check` regenerates in memory and compares, so it is what to run when a diff
touches `lucide/` and you want to know whether the blob still matches. It is
deliberately **not** a merge gate: it would be one more thing on the path of
every commit to catch a mistake that is possible about once a year, and the
compositor's own suite already draws every entry the blob claims to have.

## The blob

The format is specified in `crates/compositor/src/icon/catalogue.rs`, which
reads it; `tools/icons/src/blob.rs` writes it. Neither is allowed to be the only
one that changes.

A record carries a name, its tags, its categories, the names upstream retired
for it, and its two contour lists. The format's own version is a byte in the
header and moves whenever that list does — it is at **2**, aliases having joined
a record after the first cut — so a blob a newer tool wrote is refused by an
older reader rather than read short. Moving it means regenerating and committing
the blob in the same change, which is what `--check` above then confirms.

Upstream's `aliases` are objects — a retired `name` beside the reason it was
retired — and it is the name the blob carries. They are there to be **found**,
never to be written: the catalogue has one name per icon, and an `icon` asset
naming a retired one is refused like any other unknown name.

The conversion happens here, at vendor time, rather than in the compositor: the
elements Lucide draws with are `path`, `circle`, `line`, `rect`, `polyline`,
`polygon` and `ellipse`, and every one of them reduces to moves, lines and
Béziers. Reading them at render time would mean an SVG implementation in the
crate that draws frames, to read files that never leave that list.
