#!/usr/bin/env python3
"""The tripwire fires when the surface collapses, and only then.

Written against #363/#365, where three `exclude_re` entries were regexes that
matched everything and the mutation signal reported *nothing to report* for ten
merged pull requests. The check that would have caught that is only worth
having if it is itself checked -- a tripwire nobody trips is indistinguishable
from one that is wired to nothing.

Exercised as its callers call it: as a subprocess, on its argv contract,
asserting on the exit status and on what lands on stdout and stderr.
"""

from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "mutation-surface.py"


def run(*argv: str) -> subprocess.CompletedProcess[str]:
    """The script, invoked as `make` and the workflow invoke it."""
    return subprocess.run(
        [sys.executable, str(SCRIPT), *argv],
        capture_output=True,
        text=True,
        check=False,
    )


class Surface(unittest.TestCase):
    """A mutant list and a floor, and what the script makes of the pair."""

    def setUp(self) -> None:
        self.scratch = tempfile.TemporaryDirectory()
        self.addCleanup(self.scratch.cleanup)
        self.dir = Path(self.scratch.name)

    def config(self, body: str) -> str:
        """A stand-in `.cargo/mutants.toml` holding `body`."""
        path = self.dir / "mutants.toml"
        path.write_text(body, encoding="utf-8")
        return str(path)

    def listing(self, mutants: int, trailing: str = "") -> str:
        """A stand-in `cargo mutants --list` capture of `mutants` lines."""
        path = self.dir / "list.txt"
        lines = [f"crates/core/src/lib.rs:{n}: replace foo -> bool with true" for n in range(mutants)]
        path.write_text("\n".join(lines) + trailing, encoding="utf-8")
        return str(path)

    def floor(self, n: int) -> str:
        """A config whose only content that matters is the floor."""
        return self.config(f"# a comment\n# surface-floor: {n}\nexamine_globs = []\n")

    def test_an_intact_surface_passes_and_says_both_numbers(self) -> None:
        done = run(self.listing(3875), self.floor(3000))
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertIn("3875", done.stdout)
        self.assertIn("3000", done.stdout)

    def test_a_collapsed_surface_fails_and_names_the_cause(self) -> None:
        done = run(self.listing(6), self.floor(3000))
        self.assertEqual(done.returncode, 1)
        self.assertIn("6", done.stderr)
        self.assertIn("3000", done.stderr)
        self.assertIn("exclude_re", done.stderr)

    def test_the_floor_is_a_floor_and_not_a_target(self) -> None:
        """Exactly at it passes; one under it does not. Growth never trips it."""
        self.assertEqual(run(self.listing(3000), self.floor(3000)).returncode, 0)
        self.assertEqual(run(self.listing(2999), self.floor(3000)).returncode, 1)
        self.assertEqual(run(self.listing(9000), self.floor(3000)).returncode, 0)

    def test_a_trailing_newline_is_not_a_mutant(self) -> None:
        """The floor is compared against mutants, not against bytes of output."""
        self.assertEqual(run(self.listing(3000, trailing="\n"), self.floor(3000)).returncode, 0)
        self.assertEqual(run(self.listing(2999, trailing="\n\n\n"), self.floor(3000)).returncode, 1)

    def test_a_config_with_no_floor_fails_rather_than_passing(self) -> None:
        """Deleting the line must not read as satisfying it."""
        done = run(self.listing(3875), self.config("examine_globs = []\n"))
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("surface-floor", done.stderr)

    def test_a_missing_list_fails_rather_than_counting_zero(self) -> None:
        """Counting a file that is not there as a collapse would be a lie."""
        done = run(str(self.dir / "absent.txt"), self.floor(3000))
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("absent.txt", done.stderr)

    def test_the_argv_contract_is_one_or_two_paths(self) -> None:
        done = run()
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("usage", done.stderr)


class TheRealConfig(unittest.TestCase):
    """The floor this repo actually runs against."""

    def test_the_committed_config_carries_a_floor(self) -> None:
        """Whatever the number is, `.cargo/mutants.toml` has to state one."""
        config = SCRIPT.resolve().parents[2] / ".cargo" / "mutants.toml"
        found = [
            line for line in config.read_text(encoding="utf-8").splitlines()
            if line.strip().startswith("# surface-floor:")
        ]
        self.assertEqual(len(found), 1, f"expected exactly one floor line, got {found}")
        self.assertGreater(int(found[0].split(":")[1]), 0)


if __name__ == "__main__":
    unittest.main()
