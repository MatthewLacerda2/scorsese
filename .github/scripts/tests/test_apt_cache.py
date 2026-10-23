#!/usr/bin/env python3
"""Every CI job that installs packages takes them from its cached archives (#1515).

The runner's Ubuntu mirror once served at 78 kB/s and cut a green job at its
time limit. `tools/ci-apt` points apt at an archive directory the job caches,
and this holds each job of `ci.yml` that installs anything to that path: the
archives set up and restored before the first install, a time limit on every
install, and a save from `main` only after the last one, and only when every
install succeeded: a partial set saved under the key would be an exact hit for
every later run, which then saves nothing until the image moves. An install
step added above the restore, or a job that installs with no cache, fails here.
"""

from __future__ import annotations

import re
import unittest

from test_ci_ok import ROOT, WORKFLOW, jobs

ARCHIVES = "path: ~/.cache/apt/*.deb"
KEY = "key: apt-${{ github.job }}-${{ steps.apt.outputs.image }}-"
MAIN_ONLY = "!cancelled() && github.ref == 'refs/heads/main' && steps.apt-cache.outputs.cache-hit != 'true'"
SAVE_KEY = "key: ${{ steps.apt-cache.outputs.cache-primary-key }}"
INSTALLS = re.compile(r"\bapt(-get)?\s+(-\S+\s+)*install\b|tools/ci-postgres$|--with-deps|install-deps", re.M)


def steps(block: str) -> list[str]:
    """A job's steps, each as its text."""
    found: list[list[str]] = []
    inside = False
    for line in block.splitlines():
        if re.match(r"^    steps:\s*$", line):
            inside = True
        elif inside and line.startswith("      - "):
            found.append([line])
        elif inside and found and (line.startswith("        ") or not line.strip()):
            found[-1].append(line)
        elif inside and re.match(r"^    \S", line):
            inside = False
    return ["\n".join(step) for step in found]


def installs(step: str) -> bool:
    return bool(INSTALLS.search(step))


def ident(step: str) -> str | None:
    match = re.search(r"^        id: (\S+)$", step, re.M)
    return match.group(1) if match else None


def problems(block: str) -> list[str]:
    """What a job's steps get wrong about the cached path; nothing if it installs nothing."""
    every = steps(block)
    at = [i for i, step in enumerate(every) if installs(step)]
    if not at:
        return []
    first, last = at[0], at[-1]

    def index(test) -> int | None:
        return next((i for i, step in enumerate(every) if test(step)), None)

    found = []
    setup = index(lambda s: "run: tools/ci-apt ~/.cache/apt" in s and "id: apt\n" in s)
    restore = index(lambda s: "actions/cache/restore@" in s and ARCHIVES in s and KEY in s and "id: apt-cache\n" in s)
    save = index(lambda s: "actions/cache/save@" in s and ARCHIVES in s and MAIN_ONLY in s and SAVE_KEY in s)
    if setup is None or setup > first:
        found.append("`tools/ci-apt` does not run before the first install")
    if restore is None or restore > first or (setup is not None and restore < setup):
        found.append("the archives are not restored between `tools/ci-apt` and the first install")
    if save is None or save < last:
        found.append("the archives are not saved from `main` after the last install")
    for i in at:
        name = every[i].splitlines()[0].strip()
        if "timeout-minutes:" not in every[i]:
            found.append(f"no time limit on: {name}")
        if save is not None and f"steps.{ident(every[i])}.outcome == 'success'" not in every[save]:
            found.append(f"the save does not wait on: {name}")
    return found


RESTORE = f"""      - name: Apt
        id: apt
        run: tools/ci-apt ~/.cache/apt
      - id: apt-cache
        uses: actions/cache/restore@v6
        with:
          {ARCHIVES}
          {KEY}x
"""
INSTALL = """      - name: Install ffmpeg
        id: ffmpeg
        timeout-minutes: 5
        run: sudo apt-get install -y ffmpeg
"""
SAVE = f"""      - if: ${{{{ {MAIN_ONLY} && steps.ffmpeg.outcome == 'success' }}}}
        uses: actions/cache/save@v6
        with:
          {ARCHIVES}
          {SAVE_KEY}
"""


def job(*parts: str) -> str:
    return "    runs-on: ubuntu-24.04\n    steps:\n" + "".join(parts)


class Shape(unittest.TestCase):
    def test_a_cached_job_passes(self):
        self.assertEqual(problems(job(RESTORE, INSTALL, SAVE)), [])

    def test_a_job_that_installs_nothing_passes(self):
        self.assertEqual(problems(job("      - run: cargo test\n")), [])

    def test_a_job_with_no_cache_is_flagged(self):
        self.assertEqual(len(problems(job(INSTALL))), 3)

    def test_an_install_above_the_restore_is_flagged(self):
        got = problems(job(INSTALL, RESTORE, SAVE))
        self.assertIn("`tools/ci-apt` does not run before the first install", got)

    def test_a_save_before_the_last_install_is_flagged(self):
        got = problems(job(RESTORE, SAVE, INSTALL))
        self.assertEqual(got, ["the archives are not saved from `main` after the last install"])

    def test_an_install_with_no_time_limit_is_flagged(self):
        got = problems(job(RESTORE, INSTALL.replace("        timeout-minutes: 5\n", ""), SAVE))
        self.assertEqual(got, ["no time limit on: - name: Install ffmpeg"])

    def test_a_save_that_does_not_wait_on_an_install_is_flagged(self):
        got = problems(job(RESTORE, INSTALL, SAVE.replace(" && steps.ffmpeg.outcome == 'success'", "")))
        self.assertEqual(got, ["the save does not wait on: - name: Install ffmpeg"])

    def test_other_spellings_of_an_install_are_seen(self):
        for run in ("sudo apt install -y foo", "sudo apt-get -y install foo", "bunx playwright install-deps"):
            with self.subTest(run=run):
                self.assertTrue(installs(f"      - run: {run}\n"))
        self.assertFalse(installs("      - run: sudo apt-get autoclean\n"))

    def test_a_pull_request_saves_nothing(self):
        got = problems(job(RESTORE, INSTALL, SAVE.replace(MAIN_ONLY, "!cancelled()")))
        self.assertEqual(got, ["the archives are not saved from `main` after the last install"])


class Workflow(unittest.TestCase):
    def test_every_job_that_installs_is_cached(self):
        found = jobs(WORKFLOW)
        cached = [name for name, block in found.items() if steps(block) and any(map(installs, steps(block)))]
        self.assertIn("check", cached)
        self.assertIn("app", cached)
        for name in cached:
            with self.subTest(job=name):
                self.assertEqual(problems(found[name]), [])

    def test_the_script_points_apt_at_the_archives_and_retries(self):
        script = (ROOT / "tools" / "ci-apt").read_text(encoding="utf-8")
        for line in ('Dir::Cache::Archives "$dir/";', "Keep-Downloaded-Packages", "Acquire::Retries"):
            with self.subTest(line=line):
                self.assertIn(line, script)
        self.assertIn('echo "image=', script)


if __name__ == "__main__":
    unittest.main()
