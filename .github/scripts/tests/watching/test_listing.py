#!/usr/bin/env python3
"""What the watch asks GitHub, over REST (#1724): the open pull requests carrying
the go-ahead, and the labels of the issues they close.
"""

from __future__ import annotations

import importlib.util
import json
import subprocess
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("merge_queue", SCRIPTS / "merge-queue.py")
queue = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(queue)


def pr(number: int, closes=()) -> dict:
    """One open pull request labelled `queue`, as REST lists it."""
    return {"number": number, "state": "open", "draft": False,
            "head": {"sha": f"{number:040x}", "ref": f"b{number}"}, "base": {"ref": "main"},
            "labels": [{"name": "queue"}], "body": "".join(f"Closes #{n}\n" for n in closes)}


class Listing(unittest.TestCase):
    def test_the_listing_asks_rest_and_keeps_the_labelled_pull_requests(self):
        unlabelled = {**pr(2, closes=[9]), "labels": []}
        done = subprocess.CompletedProcess([], 0, json.dumps([pr(1, closes=[9]), unlabelled]), "")
        with mock.patch.object(queue.mergeable.subprocess, "run", return_value=done) as ran:
            found = queue.watch.waiting("o/r", "queue")
        self.assertEqual(ran.call_args.args[0],
                         ["gh", "api", "repos/o/r/pulls?state=open&per_page=100&page=1"])
        self.assertEqual([(p["number"], p["closingIssuesReferences"]) for p in found],
                         [(1, [{"number": 9}])])

    def test_an_issue_github_does_not_know_has_no_labels_and_stops_nothing(self):
        # Ordering only: the closing check refuses the number when the PR is taken.
        def gh(api, path):
            if path.endswith("/99999"):
                raise queue.mergeable.Answered("mergeable: gh api: Not Found (HTTP 404)")
            return {"labels": [{"name": "bug"}]}

        pulls = [queue.mergeable.pulls.from_rest(pr(1, closes=[99999, 5]))]
        with mock.patch.object(queue.mergeable, "gh", side_effect=gh):
            self.assertEqual(queue.watch.issue_labels("o/r", pulls, {}), {5: ["bug"], 99999: []})


if __name__ == "__main__":
    unittest.main()
