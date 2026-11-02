#!/usr/bin/env python3
"""The backup keeps nothing longer than the privacy policy allows, and its net is bounded (#1657).

`deploy/backup/backup` is run for real against a local rclone remote: a fake
`pg_dump` and a `date` that answers whatever day the test says it is, and the
script's `/data` pointed at a temporary folder. What it holds:

- a keep past the policy's `POLICY_DAYS` (less one backup interval), or of 0,
  refuses to start (`test_backup_policy.py` holds the days to the privacy page);
- a library file deleted the hour after a backup is gone from the remote by
  the thirtieth day, and still there three days before;
- a folder of replaced files a failed sync left is never written into, since
  a second move of the same path would overwrite the first;
- `library-replaced` past its size bound loses its oldest folders first, never
  the one the run filled; and no local dump outlives the keep.

Needs `rclone` and GNU `date`; skipped without rclone except in CI, where a
missing one is a failure, not a pass.
"""

from __future__ import annotations

import os
import shutil
import stat
import subprocess
import tempfile
import time
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "deploy" / "backup" / "backup"
DAY = 86400
HOUR = 3600
# 2026-01-01T04:00:00Z, the first night of every test.
START = 1767240000

RCLONE = shutil.which("rclone")
DATE = shutil.which("date")



@unittest.skipUnless(RCLONE or os.environ.get("CI"), "rclone is not installed")
class Backup(unittest.TestCase):
    def setUp(self) -> None:
        self.assertIsNotNone(RCLONE, "CI must install rclone for the backup's test")
        self.tmp = Path(tempfile.mkdtemp(prefix="scorsese-backup-"))
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        self.data = self.tmp / "data"
        self.remote = self.tmp / "remote"
        self.library = self.data / "library"
        self.library.mkdir(parents=True)
        bin_ = self.tmp / "bin"
        bin_.mkdir()
        # The script's only absolute paths are under /data.
        text = SCRIPT.read_text(encoding="utf-8")
        self.assertIn("/data/", text)
        self.script = self.tmp / "backup"
        self.script.write_text(text.replace("/data/", f"{self.data}/"), encoding="utf-8")
        self.script.chmod(0o755)
        # pg_dump --format custom --file <path>
        self._tool(bin_ / "pg_dump", 'while [ $# -gt 1 ]; do shift; done; echo dump > "$1"\n')
        # Asked for "now" (no -d, no -r), it answers the test's clock.
        self._tool(
            bin_ / "date",
            "for a; do case $a in -d|-r|-d*|-r*) exec "
            f'{DATE} "$@";; esac; done\n'
            f'exec {DATE} -d "@$BACKUP_TEST_NOW" "$@"\n',
        )
        (self.tmp / "rclone.conf").write_text("", encoding="utf-8")
        self.env = {
            "PATH": f"{bin_}{os.pathsep}{os.environ['PATH']}",
            "RCLONE_CONFIG": str(self.tmp / "rclone.conf"),
            "SCORSESE_BACKUP_REMOTE": str(self.remote),
            "SCORSESE_BACKUP_EVERY_HOURS": "24",
            "SCORSESE_BACKUP_KEEP_DAYS": "29",
        }

    @staticmethod
    def _tool(path: Path, body: str) -> None:
        path.write_text("#!/bin/sh\n" + body, encoding="utf-8")
        path.chmod(path.stat().st_mode | stat.S_IXUSR)

    def run_backup(self, now: int, *args: str, **env: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [str(self.script), *args],
            env={**self.env, **env, "BACKUP_TEST_NOW": str(now)},
            capture_output=True,
            text=True,
            check=False,
        )

    def backup(self, now: int, **env: str) -> None:
        done = self.run_backup(now, **env)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)

    def replaced(self) -> list[str]:
        """Every file under library-replaced, as <folder>/<path>."""
        root = self.remote / "library-replaced"
        return sorted(str(p.relative_to(root)) for p in root.rglob("*") if p.is_file())

    def test_a_keep_past_the_policy_refuses_to_start(self):
        # 30 days plus up to a day before the next run would see it: 31.
        done = self.run_backup(START, SCORSESE_BACKUP_KEEP_DAYS="30")
        self.assertNotEqual(done.returncode, 0, done.stdout)
        self.assertIn("SCORSESE_BACKUP_KEEP_DAYS", done.stderr)
        self.assertFalse((self.data / "backups").exists(), "it started before refusing")
        self.assertNotEqual(self.run_backup(START, SCORSESE_BACKUP_KEEP_DAYS="0").returncode, 0)
        # backup-loop's check at start refuses it too.
        self.assertNotEqual(self.run_backup(START, "--check", SCORSESE_BACKUP_KEEP_DAYS="30").returncode, 0)

    def test_the_check_alone_backs_nothing_up(self):
        done = self.run_backup(START, "--check")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertFalse((self.data / "backups").exists())

    def test_a_keep_within_the_policy_starts(self):
        self.backup(START, SCORSESE_BACKUP_KEEP_DAYS="29", SCORSESE_BACKUP_EVERY_HOURS="24")
        self.backup(START + DAY, SCORSESE_BACKUP_KEEP_DAYS="29", SCORSESE_BACKUP_EVERY_HOURS="12")

    def test_a_deleted_file_is_gone_by_day_30(self):
        # A backup every three days, kept the most that allows: 27 days.
        every = {"SCORSESE_BACKUP_EVERY_HOURS": "72", "SCORSESE_BACKUP_KEEP_DAYS": "27"}
        (self.library / "member").mkdir()
        (self.library / "member" / "clip.mp4").write_text("footage", encoding="utf-8")
        self.backup(START, **every)
        # Deleted the hour after that backup: the thirty days start here.
        deleted = START + HOUR
        (self.library / "member" / "clip.mp4").unlink()
        day = 3
        while START + day * DAY < deleted + 30 * DAY:
            self.backup(START + day * DAY, **every)
            if day == 27:
                self.assertTrue(
                    any(p.endswith("member/clip.mp4") for p in self.replaced()),
                    "the safety net let go of it early",
                )
            day += 3
        self.assertEqual(self.replaced(), [], "a deleted file outlived the policy's 30 days")
        self.assertFalse((self.remote / "library" / "member" / "clip.mp4").exists())

    def test_a_folder_a_failed_sync_left_is_not_written_into(self):
        clip = self.library / "clip.bin"
        clip.write_bytes(b"1")
        self.backup(START)
        # The next night's sync moved the first copy aside, then failed.
        left = self.remote / "library-replaced" / "20260101T040000Z"
        left.mkdir(parents=True)
        (left / "clip.bin").write_bytes(b"0")
        clip.write_bytes(b"2")
        self.backup(START + 2 * DAY)
        self.assertEqual((left / "clip.bin").read_bytes(), b"0")
        self.assertEqual(self.replaced(), ["20260101T035959Z/clip.bin", "20260101T040000Z/clip.bin"])

    def test_replaced_files_past_the_bound_purge_the_oldest_first(self):
        clip = self.library / "clip.bin"
        clip.write_bytes(b"0" * 400_000)
        self.backup(START)
        # Three nights each replace a 400 kB file: 1.2 MB against a 1 MB bound.
        for night in (1, 2, 3):
            clip.write_bytes(bytes([night]) * 400_000)
            self.backup(START + night * DAY, SCORSESE_BACKUP_REPLACED_MAX_MB="1")
        kept = self.replaced()
        self.assertEqual(len(kept), 2, kept)
        contents = [(self.remote / "library-replaced" / p).read_bytes()[:1] for p in kept]
        # The copies replaced on the second and third nights: those of the first and second.
        self.assertEqual(contents, [bytes([1]), bytes([2])])
        # A night that alone replaces more than the bound keeps what it replaced.
        clip.write_bytes(b"4" * 1_500_000)
        self.backup(START + 4 * DAY, SCORSESE_BACKUP_REPLACED_MAX_MB="1")
        clip.write_bytes(b"5")
        self.backup(START + 5 * DAY, SCORSESE_BACKUP_REPLACED_MAX_MB="1")
        kept = self.replaced()
        self.assertEqual(len(kept), 1, kept)
        self.assertEqual((self.remote / "library-replaced" / kept[0]).stat().st_size, 1_500_000)

    def test_a_local_dump_past_the_keep_is_deleted(self):
        old = self.data / "backups" / "db" / "scorsese-20251101T040000Z.dump"
        old.parent.mkdir(parents=True)
        old.write_text("dump", encoding="utf-8")
        then = time.time() - 30 * DAY
        os.utime(old, (then, then))
        self.backup(START)
        self.assertFalse(old.exists(), "a local dump outlived the keep")


if __name__ == "__main__":
    unittest.main()
