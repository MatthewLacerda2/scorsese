#!/usr/bin/env python3
"""A job outside `ci.yml` restores `check`'s apt archives and never saves them (#1517).

Its packages are a subset of `check`'s, so a save of its own would only spend
cache space. `test_apt_cache` holds every workflow to this shape; these are
the cases it must pass and flag.
"""

from __future__ import annotations

import unittest

from test_apt_cache import ARCHIVES, CHECK_FALLBACK, CHECK_KEY, INSTALL, RESTORE, SAVE, job, problems

READ = f"""      - name: Apt
        id: apt
        run: tools/ci-apt ~/.cache/apt
      - uses: actions/cache/restore@v6
        with:
          {ARCHIVES}
          {CHECK_KEY}
          {CHECK_FALLBACK}"""


class ReadOnly(unittest.TestCase):
    def test_a_job_that_restores_checks_archives_passes(self):
        self.assertEqual(problems(job(READ, INSTALL), saves=False), [])

    def test_a_signal_job_with_no_cache_is_flagged(self):
        got = problems(job(INSTALL), saves=False)
        self.assertEqual(
            got,
            [
                "`tools/ci-apt` does not run before the first install",
                "the archives are not restored between `tools/ci-apt` and the first install",
            ],
        )

    def test_a_signal_job_restoring_its_own_key_is_flagged(self):
        got = problems(job(RESTORE, INSTALL), saves=False)
        self.assertEqual(got, ["the archives are not restored between `tools/ci-apt` and the first install"])

    def test_a_restore_that_cannot_hit_is_flagged(self):
        restore = "the archives are not restored between `tools/ci-apt` and the first install"
        for wrong in (READ.replace("'tools/ci-postgres'", "'other'"), READ.replace(CHECK_FALLBACK, "\n")):
            with self.subTest(wrong=wrong):
                self.assertEqual(problems(job(wrong, INSTALL), saves=False), [restore])

    def test_a_signal_job_that_saves_is_flagged(self):
        got = problems(job(READ, INSTALL, SAVE), saves=False)
        self.assertEqual(got, ["the archives are saved outside `ci.yml`; restore `check`'s read-only"])

    def test_a_signal_install_with_no_time_limit_is_flagged(self):
        got = problems(job(READ, INSTALL.replace("        timeout-minutes: 5\n", "")), saves=False)
        self.assertEqual(got, ["no time limit on: - name: Install ffmpeg"])


if __name__ == "__main__":
    unittest.main()
