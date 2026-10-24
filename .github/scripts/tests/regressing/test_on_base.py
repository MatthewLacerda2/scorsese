"""A bug fix's test laid over a real base, and the job's two steps (#1508).

A throwaway repository: the base has a bug in `calc.py`, the head fixes it, and
a Python test file is the regression test, so `on_base` runs the real runner
over a real worktree. Then `main`'s `plan` and `run` with GitHub's answer
faked, which is the job's whole decision.
"""

from __future__ import annotations

import io
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[2]
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

import regression  # noqa: E402
from regression import BROKEN, FAILED, PASSED, Target  # noqa: E402

TEST = "tests/test_calc.py"
BUGGY = "def double(x):\n    return x + 2\n"
FIXED = "def double(x):\n    return x * 2\n\n\ndef triple(x):\n    return x * 3\n"
HEADER = "import sys, unittest\nfrom pathlib import Path\n" \
         "sys.path.insert(0, str(Path(__file__).resolve().parents[1]))\n"
CATCHES = HEADER + "from calc import double\n\n\nclass T(unittest.TestCase):\n" \
    "    def test_double(self):\n        self.assertEqual(double(5), 10)\n"
MISSES = CATCHES.replace("double(5), 10", "double(2), 4")
NEW_API = CATCHES.replace("import double", "import double, triple")


def git(root: Path, *argv: str) -> None:
    subprocess.run(("git", "-c", "user.name=t", "-c", "user.email=t@t", *argv), cwd=root,
                   check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def repo(case: unittest.TestCase, test: str, base: dict | None = None, head: dict | None = None) -> Path:
    """A repository whose `HEAD^1` is the bug and whose `HEAD` fixes it with
    `test`, each end with any other files given as `{path: text}`."""
    root = Path(tempfile.mkdtemp(prefix="regression-test-"))
    case.addCleanup(shutil.rmtree, root, True)
    git(root, "init", "-q")
    for path, text in {"calc.py": BUGGY, **(base or {})}.items():
        (root / path).parent.mkdir(parents=True, exist_ok=True)
        (root / path).write_text(text)
    git(root, "add", ".")
    git(root, "commit", "-qm", "base")
    for path, text in {"calc.py": FIXED, TEST: test, **(head or {})}.items():
        (root / path).parent.mkdir(parents=True, exist_ok=True)
        (root / path).write_text(text)
    git(root, "add", ".")
    git(root, "commit", "-qm", "fix")
    return root


def quietly(call, *args):
    with redirect_stdout(io.StringIO()):
        return call(*args)


def outcome(case: unittest.TestCase, test: str, base: dict | None = None, head: dict | None = None) -> str:
    """`test`'s outcome on the base, laid over it with the head's other files."""
    root, target = repo(case, test, base, head), Target("unittest", "", TEST)
    found = regression.targets(regression.changed(root, "HEAD^1"), lambda p: (root / p).exists())
    case.assertEqual(found, [target])
    return quietly(regression.on_base, root, "HEAD^1", found, dict(os.environ))[target]


class OnBase(unittest.TestCase):
    def outcome(self, test: str) -> str:
        return outcome(self, test)

    def test_a_test_that_catches_the_bug_fails_on_the_base(self) -> None:
        self.assertEqual(self.outcome(CATCHES), FAILED)

    def test_a_test_that_misses_it_passes_there(self) -> None:
        self.assertEqual(self.outcome(MISSES), PASSED)

    def test_a_test_calling_what_the_fix_added_is_broken_there(self) -> None:
        self.assertEqual(self.outcome(NEW_API), BROKEN)


class Main(unittest.TestCase):
    def main(self, root: Path, step: str, body: str, labels: tuple[str, ...]) -> tuple[int, str]:
        env = {"GITHUB_EVENT_NAME": "pull_request", "PR_NUMBER": "9", "REPO": "o/r",
               "GITHUB_TOKEN": "x", "BASE": "HEAD^1", "GITHUB_OUTPUT": str(root / "out")}
        issues = {"repos/o/r/issues/9": {"body": body, "pull_request": {}},
                  "repos/o/r/issues/7": {"state": "open", "labels": [{"name": n} for n in labels]}}
        with mock.patch.dict(os.environ, env), mock.patch.object(Path, "cwd", return_value=root), \
                mock.patch.object(regression, "labelled", return_value=lambda _api, path: issues[path]):
            status = quietly(regression.main, [step])
        out = root / "out"
        return status, out.read_text() if out.exists() else ""

    def test_a_bug_fix_whose_test_fails_on_the_base_passes(self) -> None:
        root = repo(self, CATCHES)
        self.assertEqual(self.main(root, "plan", "Closes #7", ("bug",)), (0, "run=true\napp=false\n"))
        self.assertEqual(self.main(root, "run", "Closes #7", ("bug",))[0], 0)

    def test_a_bug_fix_whose_test_passes_on_the_base_is_refused(self) -> None:
        self.assertEqual(self.main(repo(self, MISSES), "run", "Closes #7", ("bug",))[0], 1)

    def test_the_written_exception_owes_nothing(self) -> None:
        body = "Closes #7\nNo regression test on base: only real speakers show the click.\n"
        self.assertEqual(self.main(repo(self, MISSES), "plan", body, ("bug",)), (0, "run=false\napp=false\n"))

    def test_a_fix_that_is_not_a_bug_owes_nothing(self) -> None:
        self.assertEqual(self.main(repo(self, MISSES), "plan", "Closes #7", ("feature",)), (0, "run=false\napp=false\n"))

    def test_a_bug_fix_changing_no_test_is_refused_at_plan(self) -> None:
        root = repo(self, CATCHES)
        git(root, "rm", "-q", TEST)
        git(root, "commit", "-qm", "no test", "--amend")
        self.assertEqual(self.main(root, "plan", "Closes #7", ("bug",)), (1, ""))


if __name__ == "__main__":
    unittest.main()
