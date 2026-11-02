#!/usr/bin/env python3
"""The backup keeps nothing longer than the privacy policy allows, and its net is bounded (#1657).

`deploy/backup/backup` is run for real against a local rclone remote: a fake
`pg_dump` and a `date` that answers whatever day the test says it is, and the
script's `/data` pointed at a temporary folder. What it holds:

- the policy's days are the script's: `POLICY_DAYS` is what the privacy page
  promises, and a keep past it (less one backup interval) refuses to start;
- a library file deleted the hour after a backup is gone from the remote by
  the thirtieth day, and still there on the twenty-eighth;
- `library-replaced` past its size bound loses its oldest folders first.

Needs `rclone` and GNU `date`; skipped without rclone except in CI, where a
missing one is a failure, not a pass.
"""

from __future__ import annotations

import os
import re
import shutil
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "deploy" / "backup" / "backup"
PRIVACY = ROOT / "web" / "src" / "legal" / "en" / "privacy.ts"
DAY = 86400
HOUR = 3600
# 2026-01-01T04:00:00Z, the first night of every test.
START = 1767240000

RCLONE = shutil.which("rclone")
DATE = shutil.which("date")


def policy_days(text: str) -> set[int]:
    """Every number of days the privacy page gives a backup."""
    return {int(n) for n in re.findall(r"backups?\b[^.\"]*?\b(\d+) days", text)}


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
        self._tool(
            bin_ / "pg_dump",
            # pg_dump --format custom --file <path>
            'while [ $# -gt 1 ]; do shift; done; echo dump > "$1"\n',
        )
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

    def run_backup(self, now: int, **env: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [str(self.script)],
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

    def test_the_policy_is_the_privacy_pages(self):
        days = re.search(r"^POLICY_DAYS=(\d+)$", SCRIPT.read_text(encoding="utf-8"), re.M)
        self.assertIsNotNone(days, "deploy/backup/backup sets no POLICY_DAYS")
        self.assertEqual(policy_days(PRIVACY.read_text(encoding="utf-8")), {int(days.group(1))})

    def test_a_keep_past_the_policy_refuses_to_start(self):
        # 30 days plus up to a day before the next run would see it: 31.
        done = self.run_backup(START, SCORSESE_BACKUP_KEEP_DAYS="30")
        self.assertNotEqual(done.returncode, 0, done.stdout)
        self.assertIn("SCORSESE_BACKUP_KEEP_DAYS", done.stderr)
        self.assertFalse((self.data / "backups").exists(), "it started before refusing")

    def test_a_keep_within_the_policy_starts(self):
        self.backup(START, SCORSESE_BACKUP_KEEP_DAYS="29", SCORSESE_BACKUP_EVERY_HOURS="24")
        self.backup(START + DAY, SCORSESE_BACKUP_KEEP_DAYS="29", SCORSESE_BACKUP_EVERY_HOURS="12")

    def test_a_deleted_file_is_gone_by_day_30(self):
        (self.library / "member").mkdir()
        (self.library / "member" / "clip.mp4").write_text("footage", encoding="utf-8")
        self.backup(START)
        # Deleted the hour after that night's backup: the thirty days start here.
        deleted = START + HOUR
        (self.library / "member" / "clip.mp4").unlink()
        night = 1
        while START + night * DAY < deleted + 30 * DAY:
            self.backup(START + night * DAY)
            if night == 28:
                self.assertTrue(
                    any(p.endswith("member/clip.mp4") for p in self.replaced()),
                    "the safety net let go of it early",
                )
            night += 1
        self.assertEqual(self.replaced(), [], "a deleted file outlived the policy's 30 days")
        self.assertFalse((self.remote / "library" / "member" / "clip.mp4").exists())

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


class PolicyDays(unittest.TestCase):
    def test_it_reads_the_backup_sentences(self):
        text = (
            '"in a backup that is redone periodically and kept for 30 days.",'
            '"one that keeps you signed in for up to 30 days",'
            '"backups that still hold your data expire within 31 days."'
        )
        self.assertEqual(policy_days(text), {30, 31})


if __name__ == "__main__":
    unittest.main()
