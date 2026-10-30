#!/usr/bin/env python3
"""Every BuildKit cache mount in a deploy Dockerfile has an id of its own.

A `--mount=type=cache` without `id=` is keyed by its target path, so any other
build on the same builder that mounts `/usr/local/cargo/registry` or
`/src/target` shares, and can lock or poison, this project's cache. So each
names an `id=` starting `scorsese-`, and one id never caches two targets.
"""

from __future__ import annotations

import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PREFIX = "scorsese-"
MOUNT = re.compile(r"--mount=(\S+)")


def dockerfiles() -> list[Path]:
    """Every Dockerfile under `deploy/`."""
    return sorted(
        path
        for path in (ROOT / "deploy").rglob("*Dockerfile*")
        if path.is_file() and not path.name.endswith(".dockerignore")
    )


def cache_mounts(text: str) -> list[tuple[int, dict[str, str]]]:
    """Each cache mount's line number and options, in order."""
    found = []
    for number, line in enumerate(text.splitlines(), start=1):
        for match in MOUNT.finditer(line):
            options = dict(
                (part.split("=", 1) + [""])[:2] for part in match.group(1).split(",")
            )
            if options.get("type") == "cache":
                found.append((number, options))
    return found


def problems(files: dict[str, str]) -> list[str]:
    """What is wrong with the cache mounts in `files`, a name to its text."""
    wrong: list[str] = []
    targets: dict[str, tuple[str, str]] = {}
    for name, text in files.items():
        for number, options in cache_mounts(text):
            target = options.get("target") or options.get("dst") or options.get("destination", "")
            where = f"{name}:{number}"
            mount = options.get("id", "")
            if not mount.startswith(PREFIX):
                wrong.append(
                    f"{where}: the cache mount at {target} needs an id of this project's own, "
                    f"so no other build on the builder shares it: add id={PREFIX}<image>-<what>"
                    f" (e.g. id={PREFIX}server-cargo-registry)"
                )
                continue
            first = targets.setdefault(mount, (target, where))
            if first[0] != target:
                wrong.append(
                    f"{where}: id={mount} caches {target}, but {first[1]} uses it for "
                    f"{first[0]}; give each target its own id"
                )
    return wrong


class CacheMounts(unittest.TestCase):
    def test_every_deploy_cache_mount_has_its_own_id(self) -> None:
        files = {path.relative_to(ROOT).as_posix(): path.read_text(encoding="utf-8") for path in dockerfiles()}
        self.assertIn("deploy/server.Dockerfile", files)
        self.assertEqual(problems(files), [])

    def test_a_mount_without_an_id_is_named_with_what_to_add(self) -> None:
        text = (
            "FROM rust AS build\n"
            "RUN --mount=type=cache,target=/src/target,sharing=locked \\\n"
            "    --mount=type=bind,target=/x \\\n"
            "    cargo build\n"
        )
        found = problems({"d": text})
        self.assertEqual(len(found), 1, found)
        self.assertIn("d:2", found[0])
        self.assertIn("/src/target", found[0])
        self.assertIn("id=scorsese-", found[0])

    def test_an_id_another_project_could_share_is_named(self) -> None:
        found = problems({"d": "RUN --mount=type=cache,id=cargo,target=/a cargo build\n"})
        self.assertEqual(len(found), 1, found)

    def test_one_id_for_two_targets_is_named(self) -> None:
        text = (
            "RUN --mount=type=cache,id=scorsese-a,target=/a true\n"
            "RUN --mount=type=cache,id=scorsese-a,target=/b true\n"
        )
        found = problems({"d": text})
        self.assertEqual(len(found), 1, found)
        self.assertIn("d:2", found[0])

    def test_a_mount_with_its_own_id_passes(self) -> None:
        text = (
            "RUN --mount=type=cache,id=scorsese-server-target,target=/src/target,sharing=locked \\\n"
            "    --mount=type=cache,id=scorsese-server-target,target=/src/target cargo build\n"
        )
        self.assertEqual(problems({"d": text}), [])


if __name__ == "__main__":
    unittest.main()
