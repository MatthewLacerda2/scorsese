#!/usr/bin/env python3
"""A dry run changes nothing; a real run goes in order and says what changed.
"""

from __future__ import annotations

import unittest

from .scripted import APPLIED, GIT, IDLE, PS, Result, Scripted, update


class DryRun(unittest.TestCase):
    def test_it_prints_every_step_and_changes_nothing(self):
        host = Scripted(GIT + [APPLIED, IDLE, (["--format json"], Result(0, PS))])
        code, out, err = update(host, "--dry-run")
        self.assertEqual(code, 0, err)
        for n in range(1, 9):
            self.assertIn(f"\n== {n}. ", out)
        self.assertIn("bbbb A fix", out)
        self.assertIn("would run: CARGO_BUILD_JOBS=4 docker compose", out)
        self.assertIn("SQL migrations it would apply: 43 new thing", out)
        self.assertIn("200  http://127.0.0.1:8088/api/health", out)
        self.assertIn("200  https://example.org/", out)
        for never in ("pull", " up ", "backup backup", "purge-landing"):
            self.assertFalse([c for c in host.ran if never in c + " "], never)

    def test_running_work_is_reported_not_refused(self):
        busy = (["chat_turns"], Result(0, "job\trender\trunning\t2\nturn\tassistant\trunning\t1\n"))
        code, out, _ = update(Scripted(GIT + [APPLIED, busy]), "--dry-run")
        self.assertEqual(code, 0)
        self.assertIn("a real run would refuse:\n  2 render jobs running\n  1 assistant turn running", out)


class RealRun(unittest.TestCase):
    def test_waits_then_dumps_builds_purges_and_says_what_changed(self):
        busy = Result(0, "turn\tassistant\trunning\t1\n")
        dumped = Result(0, "backup: database dumped to /data/backups/db/s-1.dump\n")
        logs = Result(0, "server-1 | migrated 2 stored projects to this build's format\n")
        host = Scripted(GIT + [(["pull"], Result(0)), (["chat_turns"], [busy, Result(0)]),
                               (["_sqlx_migrations"], [APPLIED[1], Result(0, "1\ta\n43\tnew thing\n")]),
                               (["backup backup"], dumped), (["--format json"], Result(0, PS)),
                               (["logs server --since"], logs)])
        code, out, err = update(host, "--wait")
        self.assertEqual(code, 0, err)
        order = [next(i for i, c in enumerate(host.ran) if step in c)
                 for step in ("pull", "sleep", "backup backup", "up --detach", "purge-landing")]
        self.assertEqual(order, sorted(order))
        self.assertIn("SQL migrations applied: 43 new thing", out)
        self.assertIn("stored projects migrated: 2, templates: 0", out)
        self.assertIn("restore /srv/scor/backups/db/s-1.dump", out)


if __name__ == "__main__":
    unittest.main()
