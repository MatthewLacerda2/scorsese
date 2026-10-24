#!/usr/bin/env python3
"""What the stack runs comes off its server image's label, or `--from`, and is
checked against the checkout; never taken from the checkout itself.
"""

from __future__ import annotations

import unittest

from .scripted import GIT, Result, Scripted, update


class Label(unittest.TestCase):
    def test_the_running_image_label_is_what_the_stack_runs(self):
        code, out, err = update(Scripted(GIT), "--dry-run")
        self.assertEqual(code, 0, err)
        self.assertIn("the stack runs dddd", out)
        self.assertIn("commits: dddd..bbbb", out)

    def test_an_image_without_a_label_is_refused_without_from(self):
        host = Scripted([(["image", "inspect"], Result(0, "<no value>\n"))] + GIT)
        code, _, err = update(host)
        self.assertEqual(code, 1)
        self.assertIn("pass --from", err)
        self.assertFalse([c for c in host.ran if "pull" in c or " build" in c])

    def test_from_stands_in_for_the_label(self):
        host = Scripted([(["image", "inspect"], Result(0, "<no value>\n")),
                         (["--verify", "eeee"], Result(0, "eeee\n"))] + GIT)
        code, out, err = update(host, "--dry-run", "--from", "eeee")
        self.assertEqual(code, 0, err)
        self.assertIn("the stack runs eeee", out)

    def test_an_unknown_commit_is_refused(self):
        host = Scripted([(["--verify"], Result(1))] + GIT)
        code, _, err = update(host, "--from", "zzzz")
        self.assertEqual(code, 1)
        self.assertIn("zzzz, from --from, is not a commit this checkout has", err)
        self.assertFalse([c for c in host.ran if "pull" in c])

    def test_no_running_server_is_refused(self):
        host = Scripted([(["ps", "--quiet", "server"], Result(0, ""))] + GIT)
        code, _, err = update(host)
        self.assertEqual(code, 1)
        self.assertIn("no server container is running", err)

    def test_a_failing_git_log_is_a_refusal_not_an_empty_range(self):
        host = Scripted([(["--oneline"], Result(128, "", "fatal: bad revision"))] + GIT
                        + [(["pull"], Result(0))])
        code, out, err = update(host)
        self.assertEqual(code, 1)
        self.assertIn("what this update deploys is unknown", err)
        self.assertNotIn("rebuilding what is there", out)
        self.assertFalse([c for c in host.ran if " build" in c])


if __name__ == "__main__":
    unittest.main()
