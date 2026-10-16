#!/usr/bin/env python3
"""Every Postgres the project runs is the same major version.

Four places name one: the deploy's database (`deploy/compose.yaml`), its backup
sidecar built on the same image (`deploy/backup/Dockerfile`), the throwaway
container `make test` runs against (`tools/with-postgres`), and the native
server CI installs because pulling an image there hit every registry's
anonymous limit (`tools/ci-postgres`, #1260); and `docs/web.md`, which tells
a reader which one those are (#1226). Each said "change these
together" in a comment; this holds them to it, so a bump that misses one
fails here instead of letting the tests pass against a server production
does not run.
"""

from __future__ import annotations

import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
# An image reference only (`image:`, `FROM`, `IMAGE=`): a connection string's
# `postgres:5432` is a host and port, not a version.
IMAGE = re.compile(r"(?:image:\s*|FROM\s+|IMAGE=)\S*\bpostgres:(\d+)")
PG_MAJOR = re.compile(r"^PG_MAJOR=(\d+)$", re.MULTILINE)
# Prose names it as an image in backticks or as "Postgres 17".
PROSE = re.compile(r"(?:`postgres:|\bPostgres )(\d+)\b")

# Each place, and how its major version is read.
PLACES = {
    "deploy/compose.yaml": IMAGE,
    "deploy/backup/Dockerfile": IMAGE,
    "tools/with-postgres": IMAGE,
    "tools/ci-postgres": PG_MAJOR,
    "docs/web.md": PROSE,
}


def majors(texts: dict[str, str]) -> dict[str, set[str]]:
    """Every major version each place names (an empty set when it names none)."""
    return {
        path: {found.group(1) for found in PLACES[path].finditer(text)}
        for path, text in texts.items()
    }


def disagreements(found: dict[str, set[str]]) -> list[str]:
    """What is wrong, one line per place: none, several, or not the others'."""
    problems = [
        f"{path} names no Postgres major version" for path, seen in found.items() if not seen
    ]
    problems += [
        f"{path} names several: {', '.join(sorted(seen))}"
        for path, seen in found.items()
        if len(seen) > 1
    ]
    every = set().union(*found.values())
    if not problems and len(every) > 1:
        problems.append(
            "the places disagree: "
            + "; ".join(f"{path} runs {next(iter(seen))}" for path, seen in sorted(found.items()))
        )
    return problems


class PostgresVersions(unittest.TestCase):
    def test_every_place_runs_the_same_major(self) -> None:
        texts = {path: (ROOT / path).read_text(encoding="utf-8") for path in PLACES}
        self.assertEqual(disagreements(majors(texts)), [])

    def test_a_place_left_behind_is_named(self) -> None:
        found = {"a": {"17"}, "b": {"17"}, "c": {"18"}}
        [problem] = disagreements(found)
        self.assertIn("c runs 18", problem)

    def test_a_place_naming_none_or_several_is_named(self) -> None:
        found = {"a": set(), "b": {"16", "17"}}
        self.assertEqual(
            disagreements(found),
            ["a names no Postgres major version", "b names several: 16, 17"],
        )

    def test_the_image_and_the_script_are_both_read(self) -> None:
        texts = {
            "deploy/compose.yaml": "image: postgres:17-alpine\nurl: postgres://postgres:5432/x\n",
            "tools/with-postgres": "IMAGE=public.ecr.aws/docker/library/postgres:17-alpine\n",
            "tools/ci-postgres": "set -e\nPG_MAJOR=17\n",
            "docs/web.md": "a `postgres:17-alpine` container, or a native Postgres 16\n",
        }
        self.assertEqual(
            majors(texts),
            {
                "deploy/compose.yaml": {"17"},
                "tools/with-postgres": {"17"},
                "tools/ci-postgres": {"17"},
                "docs/web.md": {"16", "17"},
            },
        )


if __name__ == "__main__":
    unittest.main()
