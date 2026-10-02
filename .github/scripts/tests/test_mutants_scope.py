#!/usr/bin/env python3
"""A scope narrows to exactly what it names, and says so when it does not.

The failure this is written against is the one `docs/mutation-testing.md`
records under *Running it*: cargo-mutants' own `--file` widens a run and its
`--re` lets every struct-field deletion on the surface through, so a run asked
about one file came back describing others. `mutants-scope.py` answers that by
turning paths into a whole-file `--in-diff`, and by checking the plan against
the scope rather than trusting it. Both halves are held here, on a throwaway
git repository so the globbing is git's own.
"""

from __future__ import annotations

import importlib.util
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

_spec = importlib.util.spec_from_file_location(
    "mutants_scope", Path(__file__).resolve().parents[1] / "mutants-scope.py"
)
assert _spec and _spec.loader
scope = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(scope)

KNOWN = {"scorsese-core", "scorsese-zimmer"}


def planned(*files: str, package: str = "scorsese-core") -> list[dict]:
    """`cargo mutants --list --json`, as far as the check reads it."""
    return [{"file": f, "package": package, "name": f"{f}:1:1: x"} for f in files]


class WhichSpelling(unittest.TestCase):
    def test_diff_is_the_branch(self) -> None:
        self.assertEqual(scope.kind("diff", KNOWN), "diff")

    def test_a_package_name_is_a_crate(self) -> None:
        self.assertEqual(scope.kind("scorsese-zimmer", KNOWN), "crate")

    def test_a_mistyped_crate_is_a_path_and_not_a_guess(self) -> None:
        self.assertEqual(scope.kind("scorsese-zimer", KNOWN), "paths")

    def test_several_words_are_paths_even_if_one_is_a_crate(self) -> None:
        self.assertEqual(scope.kind("scorsese-core crates/x.rs", KNOWN), "paths")

    def test_empty_is_refused(self) -> None:
        with self.assertRaises(SystemExit):
            scope.kind("  ", KNOWN)


class PathsBecomeAWholeFileDiff(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        root = Path(self.tmp.name)
        for path in ("a/src/one.rs", "a/src/deep/two.rs", "a/src/notes.md", "b/src/three.rs"):
            (root / path).parent.mkdir(parents=True, exist_ok=True)
            (root / path).write_text("fn f() {}\nfn g() {}\n")
        subprocess.run(["git", "init", "-q"], cwd=root, check=True)
        subprocess.run(["git", "add", "."], cwd=root, check=True)
        self.cwd = Path.cwd()
        os.chdir(root)

    def tearDown(self) -> None:
        os.chdir(self.cwd)
        self.tmp.cleanup()

    def test_double_star_crosses_directories_and_skips_non_rust(self) -> None:
        self.assertEqual(scope.matching(["a/src/**"]), ["a/src/deep/two.rs", "a/src/one.rs"])

    def test_single_star_does_not(self) -> None:
        self.assertEqual(scope.matching(["a/src/*.rs"]), ["a/src/one.rs"])

    def test_the_diff_adds_every_line_of_every_named_file(self) -> None:
        plan, diff = scope.resolve("a/src/one.rs b/src/three.rs", KNOWN, "origin/main", "s.diff")

        self.assertEqual(plan["args"], ["--in-diff", "s.diff"])
        self.assertEqual(plan["files"], ["a/src/one.rs", "b/src/three.rs"])
        self.assertIn("+++ b/a/src/one.rs", diff)
        self.assertIn("+++ b/b/src/three.rs", diff)
        self.assertEqual(diff.count("+fn g() {}"), 2)

    def test_a_glob_matching_nothing_is_refused_with_its_name(self) -> None:
        with self.assertRaises(SystemExit) as refused:
            scope.resolve("c/**", KNOWN, "origin/main", "s.diff")
        self.assertIn("c/**", str(refused.exception))

    def test_a_crate_needs_no_diff(self) -> None:
        plan, diff = scope.resolve("scorsese-core", KNOWN, "origin/main", "s.diff")
        self.assertEqual((plan["args"], diff), (["-p", "scorsese-core"], ""))


class TheCheck(unittest.TestCase):
    PATHS = {"kind": "paths", "scope": "a/**", "files": ["a/one.rs"], "said": None}

    def test_a_clean_plan_says_what_it_covers(self) -> None:
        line, go = scope.check(self.PATHS, planned("a/one.rs", "a/one.rs"))
        self.assertTrue(go)
        self.assertIn("2 mutations planned", line)
        self.assertNotIn("outside", line)

    def test_strays_are_counted_and_named(self) -> None:
        line, go = scope.check(self.PATHS, planned("a/one.rs", "z/far.rs"))
        self.assertTrue(go)
        self.assertIn("1 of them are from outside that scope (`z/far.rs`)", line)

    def test_a_crate_scope_checks_the_package(self) -> None:
        crate = {"kind": "crate", "scope": "scorsese-core", "files": None}
        stray = planned("x.rs", package="scorsese-zimmer")
        self.assertIn("outside", scope.check(crate, stray)[0])

    def test_nothing_named_on_the_surface_is_no_answer(self) -> None:
        line, go = scope.check(self.PATHS, [])
        self.assertFalse(go)
        self.assertIn("None of it is on the mutated surface", line)

    def test_a_diff_with_nothing_mutable_is_an_answer(self) -> None:
        diff = {"kind": "diff", "scope": "diff", "files": [], "said": "the branch"}
        line, go = scope.check(diff, [])
        self.assertTrue(go)
        self.assertIn("0 mutations planned", line)


if __name__ == "__main__":
    unittest.main()
