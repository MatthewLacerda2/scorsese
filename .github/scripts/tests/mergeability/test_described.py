"""`make described`: the description checks the merge path applies, asked
before readying (#1701).

The three shapes the 2026-11-02 batch was handed back for are the cases it
must refuse: a closing keyword aimed at a pull request (#1687), a copied review
record whose heading is not its first line (#1690), and a review at `medium`
where the paths ask `high` (#1685). And the queue must ask the same function,
so the two cannot drift.
"""

from __future__ import annotations

import importlib.util
import io
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[2]


def load(name: str, file: str):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / file)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


described = load("described", "described.py")
mergeable = load("mergeable", "mergeable.py")

SHA = "c23b7c4f0000000000000000000000000000000a"
REPO = "MatthewLacerda2/scorsese"
MONEY = "crates/providers/src/spending/ledger.rs"
GOOD = "Closes #1701\n\nThe change.\n\n**Gates**: `make gates` green on abc1234.\n"
REVIEW_HIGH = "## Independent review — high\n\nNothing found."


def rest_pull(body: str = GOOD, branch: str = "1701-pre-ready-checks", draft: bool = True) -> dict:
    """`repos/{repo}/pulls/N` as GitHub's REST API answers it."""
    return {"number": 1702, "draft": draft, "body": body,
            "head": {"sha": SHA, "ref": branch}, "base": {"ref": "main"},
            "user": {"login": "MatthewLacerda2"}}


def github(pull: dict, files: list[str] = (), comments: list[str] = (),
           issues: dict[int, dict] | None = None):
    """A fake `gh api` over REST, answering only what the checks ask."""
    issues = {1701: {"number": 1701, "state": "open", "labels": []}, **(issues or {})}

    def gh(_api: str, path: str):
        if path == f"repos/{REPO}/pulls/1702":
            return pull
        if path.startswith(f"repos/{REPO}/pulls/1702/files"):
            return [{"filename": f, "status": "modified"} for f in files]
        if path.startswith(f"repos/{REPO}/issues/1702/comments"):
            return [{"body": c} for c in comments]
        number = int(path.rsplit("/", 1)[1])
        return issues[number]
    return gh


def ask(pull: dict, **facts) -> tuple[bool, list[str]]:
    gh = github(pull, **facts)
    shaped = described.gather(gh, REPO, described.pulls.view(gh, REPO, 1702))
    return described.check(shaped)


class Refuses(unittest.TestCase):
    def test_a_keyword_closing_a_pull_request(self):
        body = GOOD + "\n## For a human\n- [ ] close #1683 once this merges\n"
        ok, lines = ask(rest_pull(body), issues={1683: {"number": 1683, "state": "open",
                                                        "pull_request": {}, "labels": []}})
        self.assertFalse(ok, lines)
        self.assertIn("#1683, which is a pull request", lines[0])

    def test_a_review_record_whose_heading_is_not_its_first_line(self):
        copied = "Copied from #1654:\n\n" + REVIEW_HIGH
        ok, lines = ask(rest_pull(), files=[MONEY], comments=[copied])
        self.assertFalse(ok, lines)
        self.assertIn("no independent review at `high`", lines[0])

    def test_a_review_below_the_level_the_paths_ask(self):
        ok, lines = ask(rest_pull(), files=[MONEY],
                        comments=["## Independent review — medium\n\nfine"])
        self.assertFalse(ok, lines)
        self.assertIn("ran at `medium`", lines[0])

    def test_a_branch_whose_description_does_not_close_its_issue(self):
        ok, lines = ask(rest_pull("The change.\n\nGates: green"))
        self.assertFalse(ok, lines)
        self.assertIn("does not close #1701", lines[0])


class Passes(unittest.TestCase):
    def test_a_draft_whose_description_says_what_the_queue_asks(self):
        ok, lines = ask(rest_pull(), files=[MONEY], comments=[REVIEW_HIGH])
        self.assertTrue(ok, lines)
        self.assertFalse(any("Warning" in line for line in lines), lines)

    def test_a_missing_gates_line_is_a_warning_not_a_refusal(self):
        ok, lines = ask(rest_pull("Closes #1701\n\nThe change."))
        self.assertTrue(ok, lines)
        self.assertTrue(any("Gates" in line for line in lines), lines)


class OneFunction(unittest.TestCase):
    """The queue and `make described` ask the same question."""

    def test_mergeable_refuses_what_described_refuses_before_any_run(self):
        p = {"isDraft": False, "headRefOid": SHA, "number": 1702,
             "headRefName": "1701-pre-ready-checks", "body": GOOD,
             "author": {"login": "MatthewLacerda2"},
             "reviewed": {"files": [{"filename": MONEY}], "comments": []}}
        ok, lines = mergeable.judge(p, [], {})
        self.assertFalse(ok)
        self.assertEqual(lines, described.refusal(p))

    def test_the_queue_imports_it(self):
        text = (SCRIPTS / "merge-queue.py").read_text(encoding="utf-8")
        self.assertIn("described.gather(", text)
        self.assertIn("described.refusal(", text)


class Offline(Exception):
    pass


class Command(unittest.TestCase):
    def run_main(self, pull: dict, **facts) -> tuple[int, str]:
        out = io.StringIO()
        with mock.patch.object(described, "github",
                                  return_value=(github(pull, **facts), Offline, 3)), \
                mock.patch.object(described.pulls, "repository", return_value=REPO), \
                redirect_stdout(out), redirect_stderr(out):
            code = described.main(["1702"])
        return code, out.getvalue()

    def test_it_prints_the_queues_refusal_and_exits_one(self):
        code, out = self.run_main(rest_pull(), files=[MONEY])
        self.assertEqual(code, 1, out)
        self.assertIn("described: #1702: it changes money", out)

    def test_it_exits_zero_on_a_description_the_queue_takes(self):
        code, out = self.run_main(rest_pull())
        self.assertEqual(code, 0, out)

    def test_it_asks_for_a_number(self):
        with redirect_stderr(io.StringIO()):
            self.assertEqual(described.main([]), 2)


if __name__ == "__main__":
    unittest.main()
