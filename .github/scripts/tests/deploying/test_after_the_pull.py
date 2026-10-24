#!/usr/bin/env python3
"""After the pull, `deploy/update` keeps the stack serving as long as it can,
and every failure says what is running and the way back.
"""

from __future__ import annotations

import unittest

from .scripted import APPLIED, BUSY, DUMPED, GIT, IDLE, PS, Result, Scripted, update

PULLED = GIT + [(["pull"], Result(0)), APPLIED, IDLE]


class Order(unittest.TestCase):
    def test_the_build_comes_before_the_drain_the_dump_and_the_restart(self):
        host = Scripted(PULLED + [(["backup backup"], DUMPED), (["--format json"], Result(0, PS))])
        code, _, err = update(host)
        self.assertEqual(code, 0, err)
        index = {step: next(i for i, c in enumerate(host.ran) if step in c)
                 for step in (".env build", "chat_turns", "backup backup", "up --detach")}
        self.assertEqual(sorted(index, key=index.get), list(index))
        up = next(c for c in host.ran if "up --detach" in c)
        self.assertNotIn("--build", up)

    def test_a_failed_build_stops_before_the_drain_and_the_dump(self):
        host = Scripted(PULLED, fail=(".env build",))
        code, _, err = update(host)
        self.assertEqual(code, 1)
        self.assertIn("the build failed", err)
        self.assertIn("checkout is at aaaa and some images may already be built from it as `:aaaa`", err)
        self.assertNotIn("untouched", err)
        self.assertNotIn("nothing was changed", err)
        self.assertFalse([c for c in host.ran if "chat_turns" in c or "backup" in c or " up " in c])


class Before(unittest.TestCase):
    def test_a_refusal_after_the_pull_names_the_running_image_not_the_checkout(self):
        host = Scripted(GIT + [(["pull"], Result(0)), (["chat_turns"], BUSY)])
        code, out, err = update(host)
        self.assertEqual(code, 1)
        self.assertIn("the stack runs dddd: if this update goes wrong", out)
        self.assertIn("checkout is at aaaa and its images are built as `:aaaa`", err)
        self.assertIn("running containers are still dddd's", err)
        self.assertIn("SCORSESE_REVISION=aaaa", " ".join(f"{k}={v}" for k, v in host.envs[0].items()))


class WayBack(unittest.TestCase):
    def test_a_health_timeout_names_the_commit_and_the_dump(self):
        sick = (["--format json"], Result(0, '{"Service":"server","State":"running","Health":"unhealthy"}'))
        host = Scripted(PULLED + [(["backup backup"], DUMPED), sick])
        code, _, err = update(host, "--timeout", "0")
        self.assertEqual(code, 1)
        self.assertIn("server (unhealthy)", err)
        self.assertIn("check out dddd, rebuild, and restore /srv/scor/backups/db/s-1.dump", err)

    def test_an_off_machine_copy_failure_names_the_local_dump(self):
        copy = Result(1, DUMPED.out, "rclone: couldn't connect to offsite")
        host = Scripted(PULLED + [(["backup backup"], copy)])
        code, _, err = update(host)
        self.assertEqual(code, 1)
        self.assertIn("the dump was written to /srv/scor/backups/db/s-1.dump, but the backup "
                      "script failed after it", err)
        self.assertIn("couldn't connect", err)
        self.assertNotIn("no way back", err)
        self.assertFalse([c for c in host.ran if " up " in c])


if __name__ == "__main__":
    unittest.main()
