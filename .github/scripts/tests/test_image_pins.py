#!/usr/bin/env python3
"""Every image the deploy builds on is the version the repo develops with.

The server image compiles with `rust:<version>` and the web image installs with
`oven/bun:<version>`; everywhere else those versions come from one file
(`rust-toolchain.toml`'s channel, `web/package.json`'s `packageManager`, which
CI's `setup-bun` and the session hook both read). A Dockerfile names its own,
so a toolchain bump that misses it ships a server built by a compiler no gate
ran, or a front-end whose lockfile a different bun wrote (#1226).
"""

from __future__ import annotations

import json
import re
import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def image_versions(dockerfile: str, image: str) -> set[str]:
    """Every version a Dockerfile's `FROM <image>:<version>` names."""
    pattern = re.compile(rf"^FROM\s+{re.escape(image)}:(\d[\w.]*?)(?:-[\w.-]+)?(?:\s|$)", re.M)
    return {m.group(1) for m in pattern.finditer(dockerfile)}


def disagreement(where: str, image: str, found: set[str], want: str, source: str) -> list[str]:
    """One line if the Dockerfile names no version, several, or not `want`."""
    if found == {want}:
        return []
    named = ", ".join(sorted(found)) or "none"
    return [f"{where} builds on {image}:{named}; {source} says {want}"]


def problems() -> list[str]:
    toolchain = tomllib.loads((ROOT / "rust-toolchain.toml").read_text(encoding="utf-8"))
    rust = toolchain["toolchain"]["channel"]
    manager = json.loads((ROOT / "web/package.json").read_text(encoding="utf-8"))["packageManager"]
    bun = manager.removeprefix("bun@")
    server = (ROOT / "deploy/server.Dockerfile").read_text(encoding="utf-8")
    web = (ROOT / "deploy/web.Dockerfile").read_text(encoding="utf-8")
    return disagreement(
        "deploy/server.Dockerfile", "rust", image_versions(server, "rust"), rust, "rust-toolchain.toml"
    ) + disagreement(
        "deploy/web.Dockerfile", "oven/bun", image_versions(web, "oven/bun"), bun, "web/package.json"
    )


class ImagePins(unittest.TestCase):
    def test_the_deploy_images_build_with_the_pinned_toolchains(self) -> None:
        self.assertEqual(problems(), [])

    def test_a_tag_suffix_is_not_part_of_the_version(self) -> None:
        dockerfile = "FROM rust:1.96.0-slim-trixie AS build\nFROM debian:trixie-slim\n"
        self.assertEqual(image_versions(dockerfile, "rust"), {"1.96.0"})
        self.assertEqual(image_versions("FROM oven/bun:1.3.14 AS build\n", "oven/bun"), {"1.3.14"})

    def test_a_stale_or_missing_image_is_named(self) -> None:
        self.assertEqual(
            disagreement("d", "rust", {"1.95.0"}, "1.96.0", "rust-toolchain.toml"),
            ["d builds on rust:1.95.0; rust-toolchain.toml says 1.96.0"],
        )
        self.assertEqual(
            disagreement("d", "rust", set(), "1.96.0", "t"), ["d builds on rust:none; t says 1.96.0"]
        )


if __name__ == "__main__":
    unittest.main()
