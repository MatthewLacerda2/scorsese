#!/usr/bin/env python3
"""Every wait has a deadline, work that starts during the dump is caught, and a
failure of any kind past the pull says where things stand.
"""

from __future__ import annotations

import unittest

from .scripted import APPLIED, BUSY, DUMPED, GIT, PS, Result, Scripted, cli, update

PULLED = GIT + [(["pull"], Result(0)), APPLIED]
HEALTHY = (["--format json"], Result(0, PS))


def ups(host):
    return [c for c in host.ran if "up --detach" in c]


class DuringTheDump(unittest.TestCase):
    def test_work_started_during_the_dump_refuses_naming_the_dump(self):
        host = Scripted(PULLED + [(["chat_turns"], [Result(0), BUSY]), (["backup backup"], DUMPED)])
        code, _, err = update(host)
        self.assertEqual(code, 1)
        self.assertIn("work started while the dump was taken", err)
        self.assertIn("the dump /srv/scor/backups/db/s-1.dump was taken", err)
        self.assertFalse(ups(host))

    def test_under_wait_it_drains_again_and_takes_a_fresh_dump(self):
        host = Scripted(PULLED + [(["chat_turns"], [Result(0), BUSY, Result(0), Result(0)]),
                                  (["backup backup"], DUMPED), HEALTHY])
        code, _, err = update(host, "--wait")
        self.assertEqual(code, 0, err)
        dumps = [i for i, c in enumerate(host.ran) if "backup backup" in c]
        self.assertEqual(len(dumps), 2)
        self.assertLess(dumps[-1], host.ran.index(ups(host)[0]))


class Deadlines(unittest.TestCase):
    def test_wait_gives_up_at_its_deadline(self):
        host = Scripted(PULLED + [(["chat_turns"], BUSY)])
        code, _, err = update(host, "--wait", "--wait-timeout", "0")
        self.assertEqual(code, 1)
        self.assertIn("still running after waiting 0s (--wait-timeout)", err)
        self.assertFalse([c for c in host.ran if "backup" in c])

    def test_the_dump_has_a_timeout(self):
        hung = Result(124, "", "", timed_out=True)
        host = Scripted(PULLED + [(["chat_turns"], Result(0)), (["backup backup"], hung)])
        code, _, err = update(host)
        self.assertEqual(code, 1)
        self.assertIn("the backup had not finished after 30 minutes", err)
        dump = next(c for c in host.ran if "backup backup" in c)
        self.assertEqual(host.timeouts[dump], cli.Update.DUMP_TIMEOUT)
        self.assertFalse(ups(host))


class Restart(unittest.TestCase):
    def test_a_failed_up_prints_the_server_log_and_the_way_back(self):
        host = Scripted(PULLED + [(["chat_turns"], Result(0)), (["backup backup"], DUMPED),
                                  (["logs server --tail"], Result(0, "server-1 | panicked\n"))],
                        fail=("up --detach",))
        code, out, err = update(host)
        self.assertEqual(code, 1)
        self.assertIn("server-1 | panicked", out)
        self.assertIn("check out dddd, rebuild, and restore /srv/scor/backups/db/s-1.dump", err)

    def test_an_orphan_container_does_not_fail_a_healthy_stack(self):
        orphan = PS + '{"Service":"old-thing","State":"exited","Health":""}\n'
        host = Scripted(PULLED + [(["chat_turns"], Result(0)), (["backup backup"], DUMPED),
                                  (["--format json"], Result(0, orphan))])
        code, out, err = update(host, "--timeout", "0")
        self.assertEqual(code, 0, err)
        self.assertIn("healthy: server, tunnel", out)

    def test_an_unexpected_error_says_where_things_stand(self):
        host = Scripted(PULLED + [(["chat_turns"], Result(0)), (["backup backup"], DUMPED),
                                  (["--format json"], Result(0, "not json"))])
        code, _, err = update(host)
        self.assertEqual(code, 1)
        self.assertIn("Traceback", err)
        self.assertIn("something unexpected went wrong", err)
        self.assertIn("check out dddd", err)


if __name__ == "__main__":
    unittest.main()
