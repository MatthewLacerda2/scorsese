#!/usr/bin/env python3
"""Which crate the weekly mutation sweep is on, derived from the surface itself.

`.github/workflows/mutation-sweep.yml` sweeps one whole crate a week, cycling.
It used to cycle over a hand-written list of four crate names, beside
`.cargo/mutants.toml`'s `examine_globs`, and nothing held the two together:
#428 put `crates/providers/src/synth/` on the surface and the rotation never
heard of it, so those mutants were audited by the pull request that wrote them
and never again — the gap the sweep exists to close, reopened silently (#431).

So the rotation is no longer written down. It is read from `examine_globs`:
every glob names a path under `crates/<dir>/`, that directory's `Cargo.toml`
names the package, and each package the surface touches gets its own week, in
the order the globs first name it. A glob added outside those crates adds a
week the moment it lands; a crate whose last glob is removed loses its week.

One crate per slot, never two together, because the sweep's history in #341 is
read as a column over time and *a crate is the same crate* is what makes two
readings of it comparable (the workflow's header has the argument).

    mutants-rotation.py list                 # every slot, one per line
    mutants-rotation.py pick rotation        # this week's crate
    mutants-rotation.py pick scorsese-core   # a named one, checked against the list

Weeks are counted since the Unix epoch, not by ISO week number, because `%V`
has a 53rd week that would repeat or skip a crate once a year. `--week` states
one outright, which is how the tests pin it.
"""

from __future__ import annotations

import argparse
import re
import sys
import time
import tomllib
from pathlib import Path

#: The repository root, which the globs and the crates' manifests are relative to.
ROOT = Path(__file__).resolve().parents[2]

#: The leading `crates/<dir>/` of a glob — the only shape the surface is written in.
CRATE_DIR = re.compile(r"^crates/(?P<dir>[^/*?\[]+)/")

#: Seconds in a week, the unit the rotation advances by.
WEEK = 7 * 24 * 60 * 60


def rotation(root: Path) -> list[str]:
    """The crates the mutated surface touches, one slot each, in glob order."""
    config = root / ".cargo" / "mutants.toml"
    globs = tomllib.loads(config.read_text(encoding="utf-8")).get("examine_globs", [])
    if not globs:
        raise SystemExit(
            f"{config} has no `examine_globs`, so there is no surface to rotate"
            " over. A sweep of nothing would report a perfect catch rate."
        )

    crates: list[str] = []
    for glob in globs:
        found = CRATE_DIR.match(glob)
        if found is None:
            raise SystemExit(
                f"`{glob}` in {config}'s `examine_globs` is not under"
                " `crates/<dir>/`, so it names no crate the sweep can take."
            )
        manifest = root / "crates" / found["dir"] / "Cargo.toml"
        try:
            name = tomllib.loads(manifest.read_text(encoding="utf-8"))["package"]["name"]
        except (OSError, KeyError) as why:
            raise SystemExit(
                f"`{glob}` points into crates/{found['dir']}/, but {manifest}"
                f" names no package: {why!r}"
            ) from why
        if name not in crates:
            crates.append(name)
    return crates


def pick(crates: list[str], requested: str, week: int) -> str:
    """The crate a run sweeps: the week's slot, or the one asked for by name."""
    if requested in ("", "rotation"):
        return crates[week % len(crates)]
    if requested not in crates:
        raise SystemExit(
            f"`{requested}` is not on the mutated surface; `examine_globs` yields"
            f" {', '.join(crates)}. Add a glob for it to .cargo/mutants.toml first."
        )
    return requested


def parse_args(argv: list[str]) -> argparse.Namespace:
    """The argv contract the workflow and the tests share."""
    parser = argparse.ArgumentParser(
        prog="mutants-rotation.py",
        description="The weekly mutation sweep's rotation, read from .cargo/mutants.toml.",
    )
    parser.add_argument("--root", type=Path, default=ROOT, help="the repository root")
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("list", help="print every slot of the rotation, one per line")
    chosen = commands.add_parser("pick", help="print the crate this run sweeps")
    chosen.add_argument(
        "requested",
        nargs="?",
        default="rotation",
        help="a crate name, or `rotation` (the default) for this week's slot",
    )
    chosen.add_argument(
        "--week",
        type=int,
        default=int(time.time()) // WEEK,
        help="weeks since the Unix epoch; defaults to now",
    )
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    """Answer the subcommand asked for."""
    args = parse_args(argv)
    crates = rotation(args.root)
    if args.command == "list":
        print("\n".join(crates))
    else:
        print(pick(crates, args.requested, args.week))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
