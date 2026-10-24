#!/usr/bin/env python3
"""Every workflow job that installs packages takes them from cached archives (#1515, #1517).

The runner's Ubuntu mirror once served at 78 kB/s and cut a green job at its
time limit. `tools/ci-apt` points apt at an archive directory a job caches,
and this holds each job of every workflow that installs anything to that path:
the archives set up and restored before the first install, and a time limit on
every install. A job of `ci.yml` owns its archives and also saves them from
`main`, only after the last install and only when every install succeeded: a
partial set saved under the key would be an exact hit for every later run,
which then saves nothing until the image moves. A job of any other workflow
installs a subset of `check`'s packages, so it restores `check`'s archives and
never saves: a save would spend the cache space the Rust caches need. An
install step added above the restore, or a job that installs with no cache,
fails here.
"""

from __future__ import annotations

import re
import unittest

from test_ci_ok import ROOT, jobs

WORKFLOWS = ROOT / ".github" / "workflows"

ARCHIVES = "path: ~/.cache/apt/*.deb"
KEY = "key: apt-${{ github.job }}-${{ steps.apt.outputs.image }}-"
CHECK_KEY = "key: apt-check-${{ steps.apt.outputs.image }}-${{ hashFiles('tools/ci-apt', 'tools/ci-postgres') }}"
CHECK_FALLBACK = "restore-keys: |\n            apt-check-${{ steps.apt.outputs.image }}-\n"
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


def problems(block: str, saves: bool = True) -> list[str]:
    """What a job's steps get wrong about the cached path; nothing if it installs nothing.

    `saves` is whether the job owns its archives (`ci.yml`) or only reads `check`'s.
    """
    every = steps(block)
    at = [i for i, step in enumerate(every) if installs(step)]
    if not at:
        return []
    first, last = at[0], at[-1]

    def index(test) -> int | None:
        return next((i for i, step in enumerate(every) if test(step)), None)

    found = []
    setup = index(lambda s: "run: tools/ci-apt ~/.cache/apt" in s and "id: apt\n" in s)
    if saves:
        restore = index(lambda s: "actions/cache/restore@" in s and ARCHIVES in s and KEY in s and "id: apt-cache\n" in s)
        save = index(lambda s: "actions/cache/save@" in s and ARCHIVES in s and MAIN_ONLY in s and SAVE_KEY in s)
    else:
        restore = index(
            lambda s: "actions/cache/restore@" in s and ARCHIVES in s and f"{CHECK_KEY}\n" in s and CHECK_FALLBACK.rstrip() in s
        )
        save = None
    if setup is None or setup > first:
        found.append("`tools/ci-apt` does not run before the first install")
    if restore is None or restore > first or (setup is not None and restore < setup):
        found.append("the archives are not restored between `tools/ci-apt` and the first install")
    if saves and (save is None or save < last):
        found.append("the archives are not saved from `main` after the last install")
    if not saves and any("actions/cache/save@" in s and ARCHIVES in s for s in every):
        found.append("the archives are saved outside `ci.yml`; restore `check`'s read-only")
    for i in at:
        name = every[i].splitlines()[0].strip()
        if "timeout-minutes:" not in every[i]:
            found.append(f"no time limit on: {name}")
        if saves and save is not None and f"steps.{ident(every[i])}.outcome == 'success'" not in every[save]:
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
        cached = []
        for path in sorted([*WORKFLOWS.glob("*.yml"), *WORKFLOWS.glob("*.yaml")]):
            text = path.read_text(encoding="utf-8")
            if "\njobs:\n" not in text:
                continue
            for name, block in jobs(text).items():
                if not any(map(installs, steps(block))):
                    continue
                cached.append(f"{path.stem}/{name}")
                with self.subTest(job=f"{path.name}: {name}"):
                    self.assertEqual(problems(block, saves=path.name == "ci.yml"), [])
        for name in ("ci/check", "ci/app", "coverage/coverage", "mutants-on-request/shard", "mutation-sweep/shard"):
            self.assertIn(name, cached)

    def test_the_read_only_key_is_the_one_check_saves_under(self):
        check = jobs((WORKFLOWS / "ci.yml").read_text(encoding="utf-8"))["check"]
        self.assertIn(CHECK_KEY.replace("apt-check-", "apt-${{ github.job }}-") + "\n", check)

    def test_the_script_points_apt_at_the_archives_and_retries(self):
        script = (ROOT / "tools" / "ci-apt").read_text(encoding="utf-8")
        for line in ('Dir::Cache::Archives "$dir/";', "Keep-Downloaded-Packages", "Acquire::Retries"):
            with self.subTest(line=line):
                self.assertIn(line, script)
        self.assertIn('echo "image=', script)


if __name__ == "__main__":
    unittest.main()
