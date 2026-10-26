"""What a bug fix owes and which targets its changed tests name (#1508).

Pure: `is_test`, `target`, `bugs`, `excused`, `owed` and `verdict` over plain
values, the repository's layout faked by an `exists` over a set of paths.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[2]
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

import regression  # noqa: E402
from regression import BROKEN, FAILED, PASSED, Target  # noqa: E402

TREE = {
    "crates/core/Cargo.toml", "crates/core/tests/time.rs",
    "crates/server/Cargo.toml", "crates/server/tests/suite/main.rs",
    "crates/server/tests/suite/api/mod.rs", "crates/server/tests/suite/api/projects.rs",
    "crates/server/tests/suite/logs.rs", "crates/server/tests/suite/common/mod.rs",
    "crates/golden/Cargo.toml", "crates/golden/fixtures/alpha/project.json",
    "app/Cargo.toml", "app/tests/panels/main.rs", "app/tests/panels/fixture.rs",
    "web/package.json", "web/src/timeline/Track.test.tsx",
    ".github/scripts/tests/test_closing.py", ".github/scripts/tests/fixtures/a.json",
}


def exists(path: str) -> bool:
    return path in TREE


def bug(*labels: str, **fields: object) -> dict:
    return {"state": "open", "labels": [{"name": n} for n in labels], **fields}


class Classify(unittest.TestCase):
    def test_tests_as_the_size_gate_reads_them_and_the_golden_fixtures(self) -> None:
        for path in sorted(TREE - {"crates/core/Cargo.toml", "crates/server/Cargo.toml",
                                   "crates/golden/Cargo.toml", "app/Cargo.toml",
                                   "web/package.json"}):
            with self.subTest(path=path):
                self.assertTrue(regression.is_test(path))
        for path in ("crates/core/src/time.rs", "web/src/timeline/Track.tsx",
                     ".github/scripts/closing.py", "docs/testing.md", "crates/golden/src/lib.rs"):
            with self.subTest(path=path):
                self.assertFalse(regression.is_test(path))

    def test_each_file_names_the_target_its_runner_runs(self) -> None:
        self.assertEqual(regression.targets(sorted(TREE), exists), sorted([
            Target("cargo", "crates/core", "time"),
            Target("cargo", "crates/server", "suite", "api"),
            Target("cargo", "crates/server", "suite", "logs"),
            Target("cargo", "crates/golden", "suite"),
            Target("cargo", "app", "panels"),
            Target("bun", "web", "src/timeline/Track.test.tsx"),
            Target("unittest", "", ".github/scripts/tests/test_closing.py"),
        ]))

    def test_a_helper_or_a_fixture_names_none(self) -> None:
        for path in ("crates/server/tests/suite/common/mod.rs", "crates/server/tests/suite/main.rs",
                     ".github/scripts/tests/fixtures/a.json"):
            with self.subTest(path=path):
                self.assertIsNone(regression.target(path, exists))

    def test_a_binary_other_than_the_suite_runs_whole(self) -> None:
        self.assertEqual(regression.target("app/tests/panels/fixture.rs", exists),
                         Target("cargo", "app", "panels"))

    def test_a_deleted_test_names_none(self) -> None:
        self.assertEqual(regression.targets(["crates/core/tests/gone.rs"], exists), [])


class Owed(unittest.TestCase):
    def test_only_an_open_issue_labelled_bug_is_a_bug(self) -> None:
        closes = {1: bug("bug", "agent"), 2: bug("feature"), 3: bug("bug", pull_request={})}
        self.assertEqual(regression.bugs(closes), [1])

    def test_the_exception_needs_a_reason(self) -> None:
        body = "Closes #4\n\nNo regression test on base: only a real window shows the stutter.\n"
        self.assertEqual(regression.excused(body), "only a real window shows the stutter.")
        self.assertIsNone(regression.excused("No regression test on base: none"))
        self.assertIsNone(regression.excused("Closes #4"))

    def test_a_bug_fix_with_no_target_and_no_exception_is_refused(self) -> None:
        lines = regression.owed([4], "Closes #4", [])
        self.assertIn("#4, labelled `bug`", lines[0])
        self.assertIn("No regression test on base: <why>", lines[-1])

    def test_a_target_an_exception_or_no_bug_owes_nothing_here(self) -> None:
        found = [Target("cargo", "crates/core", "time")]
        self.assertEqual(regression.owed([4], "Closes #4", found), [])
        self.assertEqual(regression.owed([4], "No regression test on base: taste, by ear only.", []), [])
        self.assertEqual(regression.owed([], "Closes #4", []), [])


class Verdict(unittest.TestCase):
    CORE, API = Target("cargo", "crates/core", "time"), Target("cargo", "crates/server", "api")

    def test_one_target_failing_on_the_base_passes(self) -> None:
        self.assertEqual(regression.verdict([4], {self.CORE: PASSED, self.API: FAILED}), [])

    def test_passing_or_broken_on_the_base_is_refused_by_target(self) -> None:
        lines = regression.verdict([4], {self.CORE: PASSED, self.API: BROKEN})
        self.assertIn("nothing shows #4 fixed", lines[0])
        self.assertIn("`crates/core` test target `time` passes on the base", lines[1])
        self.assertIn("`api` cannot be judged on the base", lines[2])
        self.assertIn("No regression test on base", lines[-1])


if __name__ == "__main__":
    unittest.main()
