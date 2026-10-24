"""How each runner's answer reads, and where a Cargo target builds (#1508).

Pure: nextest's exit codes, `bun test`'s output, the innermost workspace over
a scratch tree, what is laid over the base and the `app` output.
"""

from __future__ import annotations

import os
import shutil
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[2]
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

import regression  # noqa: E402
from regression import BROKEN, FAILED, PASSED, Target  # noqa: E402

MISSING = "TypeError: m.two is not a function. (In 'm.two()', 'm.two' is undefined)\n" \
    "      at <anonymous> (/w/a.test.ts:3:43)\n(fail) new api [1.23ms]\n"
UNDEFINED = "TypeError: undefined is not an object (evaluating '(void 0).x')\n(fail) undef [0.09ms]\n"
UNDECLARED = "ReferenceError: nope is not defined\n(fail) ref [0.11ms]\n"
WRONG = "error: expect(received).toBe(expected)\n\nExpected: 2\nReceived: 1\n\n(fail) real [1.90ms]\n"
SUMMARY = "\n 0 pass\n 1 fail\n"


class Runners(unittest.TestCase):
    def test_nextest_fails_on_100_alone(self) -> None:
        # 100 is a failing test, a slow-timeout kill included; 101 a build.
        self.assertEqual([regression.nextest_outcome(c) for c in (0, 100, 101, 4)],
                         [PASSED, FAILED, BROKEN, BROKEN])

    def test_bun_a_real_failure_fails(self) -> None:
        self.assertEqual(regression.bun_outcome(1, WRONG + SUMMARY), FAILED)
        self.assertEqual(regression.bun_outcome(1, MISSING + WRONG + SUMMARY), FAILED)

    def test_bun_failing_only_on_undeclared_names_is_broken(self) -> None:
        self.assertEqual(regression.bun_outcome(1, UNDECLARED + UNDECLARED + SUMMARY), BROKEN)

    def test_bun_type_errors_are_what_a_bug_throws_so_they_fail(self) -> None:
        for out in (MISSING, UNDEFINED, MISSING + UNDECLARED):
            with self.subTest(out=out):
                self.assertEqual(regression.bun_outcome(1, out + SUMMARY), FAILED)

    def test_bun_with_no_failing_test_is_broken_and_a_clean_run_passes(self) -> None:
        self.assertEqual(regression.bun_outcome(1, "SyntaxError: Export named 'x' not found\n"), BROKEN)
        self.assertEqual(regression.bun_outcome(0, "(pass) t\n"), PASSED)


class Workspace(unittest.TestCase):
    def test_a_package_builds_in_its_innermost_workspace(self) -> None:
        root = Path(tempfile.mkdtemp(prefix="regression-ws-"))
        self.addCleanup(shutil.rmtree, root, True)
        for where, text in {".": "[workspace]\n", "crates/core": "[package]\n", "app": "[workspace]\n",
                            "app/crates/ui": "[package]\n", "tools/lint": "[package]\n[workspace]\n"}.items():
            (root / where).mkdir(parents=True, exist_ok=True)
            (root / where / "Cargo.toml").write_text(text)
        got = [regression.workspace(p, root) for p in ("crates/core", "app/crates/ui", "app", "tools/lint")]
        self.assertEqual(got, [".", "app", "app", "tools/lint"])


class Laid(unittest.TestCase):
    def test_fixtures_anywhere_are_laid_over_but_name_no_target(self) -> None:
        for path in ("crates/providers/fixtures/eleven/reply.json", "crates/golden/fixtures/a/project.json",
                     "web/src/fixtures/x.json", "crates/core/tests/time.rs"):
            with self.subTest(path=path):
                self.assertTrue(regression.laid(path))
        self.assertFalse(regression.laid("crates/core/src/fixtures.rs"))
        self.assertIsNone(regression.target("crates/providers/fixtures/eleven/reply.json", lambda _p: True))

    def test_the_app_output_says_whether_an_app_target_runs(self) -> None:
        root = Path(tempfile.mkdtemp(prefix="regression-out-"))
        self.addCleanup(shutil.rmtree, root, True)
        (root / "app").mkdir()
        (root / "app" / "Cargo.toml").write_text("[workspace]\n")
        out = root / "out"
        with mock.patch.dict(os.environ, {"GITHUB_OUTPUT": str(out)}), \
                mock.patch.object(Path, "cwd", return_value=root):
            regression.write_output(True, [Target("cargo", "app", "panels")])
            regression.write_output(True, [Target("cargo", "crates/core", "time")])
        self.assertEqual(out.read_text(), "run=true\napp=true\nrun=true\napp=false\n")


if __name__ == "__main__":
    unittest.main()
