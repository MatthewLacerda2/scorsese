#!/usr/bin/env python3
"""After SCORSESE_DATA moves, the spool volume is made again before `up`, which
would otherwise ask, and without a terminal keep the old folder.
"""

from __future__ import annotations

import unittest

from .scripted import APPLIED, DUMPED, GIT, IDLE, PS, Result, Scripted, update

MOVED = (["volume", "inspect"],
         Result(0, '{"device":"/old/scor/cache/captures","o":"bind","type":"none"}\n'))
REAL = [(["pull"], Result(0)), APPLIED, IDLE, (["backup backup"], DUMPED),
        (["ps --all"], Result(0, PS))]


def ran(host, snippet):
    return [i for i, c in enumerate(host.ran) if snippet in c]


class Moved(unittest.TestCase):
    def test_the_volume_is_made_again_before_up(self):
        host = Scripted([MOVED] + GIT + REAL)
        code, out, err = update(host)
        self.assertEqual(code, 0, err)
        self.assertIn("points at /old/scor/cache/captures, not /srv/scor/cache/captures", out)
        rm, vol, up = (ran(host, s) for s in ("rm --stop --force capture-launcher",
                                              "volume rm scorsese_capture-spool", "up --detach"))
        self.assertTrue(rm and vol and up, host.ran)
        self.assertLess(rm[0], vol[0])
        self.assertLess(vol[0], up[0])

    def test_a_dry_run_says_so_and_removes_nothing(self):
        host = Scripted([MOVED] + GIT + [APPLIED, IDLE, (["ps --all"], Result(0, PS))])
        code, out, err = update(host, "--dry-run")
        self.assertEqual(code, 0, err)
        self.assertIn("would run: docker volume rm scorsese_capture-spool", out)
        self.assertFalse(ran(host, "volume rm") or ran(host, "rm --stop"))

    def test_a_failed_removal_stops_before_up_and_says_where_things_are(self):
        failed = (["volume rm"], Result(1, err="volume is in use"))
        host = Scripted([MOVED, failed] + GIT + REAL)
        code, _, err = update(host)
        self.assertEqual(code, 1)
        self.assertIn("so the stack was not restarted", err)
        self.assertIn("volume is in use", err)
        self.assertIn("built and `latest`", err)
        self.assertIn("capture-launcher may be stopped or removed", err)
        self.assertIn("the other running containers are still dddd's", err)
        self.assertNotIn("some may already be", err)
        self.assertNotIn("docker compose up", err)
        self.assertFalse(ran(host, "up --detach"))


class Unmoved(unittest.TestCase):
    def test_a_volume_on_the_right_folder_is_left_alone(self):
        host = Scripted(GIT + REAL)
        code, _, err = update(host)
        self.assertEqual(code, 0, err)
        self.assertFalse(ran(host, "volume rm") or ran(host, "rm --stop"))

    def test_no_volume_yet_is_left_to_up(self):
        missing = (["volume", "inspect"], Result(1, err="no such volume"))
        host = Scripted([missing] + GIT + REAL)
        code, _, err = update(host)
        self.assertEqual(code, 0, err)
        self.assertFalse(ran(host, "volume rm"))

    def test_a_config_without_the_volume_refuses(self):
        bare = (["config", "--format json"], Result(0, '{"name": "scorsese", "volumes": {}}'))
        code, _, err = update(Scripted([bare] + GIT + REAL))
        self.assertEqual(code, 1)
        self.assertIn("does not describe the `capture-spool` volume", err)

    def test_a_config_it_cannot_read_refuses_before_any_retag(self):
        broken = (["config", "--format json"], Result(1, err="yaml: line 3: bad"))
        host = Scripted([MOVED, broken] + GIT + REAL)
        code, _, err = update(host)
        self.assertEqual(code, 1)
        self.assertIn("`docker compose config` failed", err)
        self.assertIn("with `latest` left as it was", err)
        touched = ran(host, "docker tag") + ran(host, "rm --stop") + ran(host, "up --detach")
        self.assertFalse(touched, host.ran)


if __name__ == "__main__":
    unittest.main()
