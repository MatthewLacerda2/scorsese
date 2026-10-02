#!/usr/bin/env python3
"""The sweep's rotation is the surface's crates, and cannot come apart from it.

Written against #431: the rotation was a hand-written list of four crates
beside `examine_globs`, #428 added a fifth crate's subtree to the globs, and
the sweep never heard of it. These hold the derivation to the config on a tree
no real repo produced, and then hold the real workflow's hand-pickable list to
what the real config derives — the one place a list still has to be written.

Exercised as the workflow calls it: as a subprocess, on its argv contract.
"""

from __future__ import annotations

import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "mutants-rotation.py"
REPO = SCRIPT.parents[2]
WORKFLOW = REPO / ".github" / "workflows" / "mutation-sweep.yml"


def run(*argv: str) -> subprocess.CompletedProcess[str]:
    """The script, invoked as the workflow invokes it."""
    return subprocess.run(
        [sys.executable, str(SCRIPT), *argv], capture_output=True, text=True, check=False
    )


class Derived(unittest.TestCase):
    """A stand-in repository: a config and the manifests its globs point into."""

    def setUp(self) -> None:
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name)
        for crate in ("core", "render", "providers"):
            (self.root / "crates" / crate).mkdir(parents=True)
            (self.root / "crates" / crate / "Cargo.toml").write_text(
                f'[package]\nname = "x-{crate}"\nversion = "0.1.0"\n', encoding="utf-8"
            )

    def globs(self, *globs: str) -> None:
        """Write `examine_globs` as `.cargo/mutants.toml` would carry it."""
        (self.root / ".cargo").mkdir(exist_ok=True)
        listed = ", ".join(f'"{glob}"' for glob in globs)
        (self.root / ".cargo" / "mutants.toml").write_text(
            f"# surface-floor: 1\nexamine_globs = [{listed}]\n", encoding="utf-8"
        )

    def rotation(self, *rest: str) -> subprocess.CompletedProcess[str]:
        return run("--root", str(self.root), *rest)

    def test_each_crate_the_globs_touch_is_one_slot_in_glob_order(self) -> None:
        """Two globs into one crate are one week, not two; order is first mention."""
        self.globs(
            "crates/render/src/plan/**/*.rs",
            "crates/core/src/**/*.rs",
            "crates/render/src/audio/mix.rs",
            "crates/providers/src/synth/**/*.rs",
        )
        done = self.rotation("list")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(done.stdout.split(), ["x-render", "x-core", "x-providers"])

    def test_the_week_walks_every_slot_and_wraps(self) -> None:
        self.globs("crates/core/src/**/*.rs", "crates/render/src/**/*.rs")
        picked = [self.rotation("pick", "--week", str(w)).stdout.strip() for w in range(4)]
        self.assertEqual(picked, ["x-core", "x-render", "x-core", "x-render"])

    def test_a_new_glob_is_a_new_week_with_nothing_else_edited(self) -> None:
        """The #431 failure: the surface grew and the rotation did not."""
        self.globs("crates/core/src/**/*.rs")
        self.assertEqual(self.rotation("list").stdout.split(), ["x-core"])
        self.globs("crates/core/src/**/*.rs", "crates/providers/src/synth/**/*.rs")
        self.assertEqual(self.rotation("list").stdout.split(), ["x-core", "x-providers"])

    def test_a_named_crate_on_the_surface_is_taken_as_asked(self) -> None:
        self.globs("crates/core/src/**/*.rs", "crates/render/src/**/*.rs")
        done = self.rotation("pick", "x-render", "--week", "0")
        self.assertEqual(done.stdout.strip(), "x-render")

    def test_a_named_crate_off_the_surface_is_refused(self) -> None:
        """Sweeping a crate the config does not mutate would sweep nothing."""
        self.globs("crates/core/src/**/*.rs")
        done = self.rotation("pick", "x-render")
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("x-render", done.stderr)

    def test_a_glob_outside_crates_is_refused_rather_than_skipped(self) -> None:
        self.globs("crates/core/src/**/*.rs", "tools/lint/src/**/*.rs")
        done = self.rotation("list")
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("tools/lint", done.stderr)

    def test_a_glob_into_a_directory_with_no_manifest_is_refused(self) -> None:
        self.globs("crates/gone/src/**/*.rs")
        done = self.rotation("list")
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("crates/gone", done.stderr)

    def test_no_globs_is_refused_rather_than_an_empty_rotation(self) -> None:
        self.globs()
        self.assertNotEqual(self.rotation("list").returncode, 0)


class TheRealWorkflow(unittest.TestCase):
    """This repo's config and the one list the workflow still has to write out."""

    def test_the_hand_pickable_crates_are_exactly_the_rotation(self) -> None:
        """`workflow_dispatch` choices are static YAML, so they are held here."""
        derived = run("list").stdout.split()
        text = WORKFLOW.read_text(encoding="utf-8")
        block = re.search(r"^\s+options:\n((?:\s+(?:- \S+|#.*)\n)+)", text, re.MULTILINE)
        self.assertIsNotNone(block, "no `options:` list under the crate input")
        options = re.findall(r"- (\S+)", block.group(1)) if block else []
        self.assertEqual(options, ["rotation", *derived])

    def test_the_real_surface_has_a_slot_for_providers(self) -> None:
        """#428's subtree, the one the hand-written list missed."""
        self.assertIn("scorsese-providers", run("list").stdout.split())


if __name__ == "__main__":
    unittest.main()
