#!/usr/bin/env python3
"""Every workflow job that runs the workspace's tests has the pinned Chromium (#1744).

`html` clips are captured with the chrome-headless-shell `tools/chromium/pin`
names, and the tests that capture a page fail without it rather than skip. A
mutation shard once lacked it: its baseline failed on one `scorsese-mcp` test
and the run measured nothing. So a job that runs the workspace's tests (nextest,
`cargo test`, llvm-cov, a mutation run, the regression check) restores the
browser's cache and exports it, both before the first test step, the way
`ci.yml`'s `check` does, from a step that fails when the download does.
Chrome cannot start under an address-space cap, so a test step that caps
memory caps it softly and runs the browser through `tools/chromium/uncapped`,
which lifts the cap for it alone. Another workspace's tests (`--manifest-path`)
and `cargo mutants --list` run no test and need nothing.
"""

from __future__ import annotations

import re
import unittest

from test_apt_cache import WORKFLOWS, job, steps
from test_ci_ok import jobs

CACHE = "path: ~/.cache/scorsese/chromium"
KEY = "key: chromium-${{ hashFiles('tools/chromium/pin') }}"
FETCH = "chrome=$(tools/chromium/fetch)"
EXPORT = 'echo "SCORSESE_CHROME=$chrome" >> "$GITHUB_ENV"'
UNCAPPED = 'export SCORSESE_CHROME="$PWD/tools/chromium/uncapped"'
NO_SANDBOX = 'echo "SCORSESE_CHROME_NO_SANDBOX=1" >> "$GITHUB_ENV"'
RUNS_TESTS = re.compile(
    r"cargo (?:nextest run|test|llvm-cov nextest)\b(?![^\n]*--manifest-path)"
    r"|cargo mutants\b(?![^\n]*--(?:list|version))"
    r"|regression\.py run\b"
)


def tests(step: str) -> bool:
    """Whether a step runs the workspace's tests."""
    body = "\n".join(line for line in step.splitlines() if not line.strip().startswith("#"))
    return bool(RUNS_TESTS.search(body))


def problems(block: str) -> list[str]:
    """What a job gets wrong about the browser; nothing if it runs no workspace test."""
    every = steps(block)
    first = next((i for i, step in enumerate(every) if tests(step)), None)
    if first is None:
        return []
    cache = next((i for i, s in enumerate(every) if "actions/cache@" in s and CACHE in s and KEY in s), None)
    export = next((i for i, s in enumerate(every) if f"{FETCH}\n          {EXPORT}" in s), None)
    found = []
    if cache is None or cache > first:
        found.append("the Chromium cache is not restored before the first test step")
    if export is None or export > first:
        found.append("`tools/chromium/fetch` is not exported as SCORSESE_CHROME before the first test step")
    elif NO_SANDBOX not in every[export]:
        found.append("SCORSESE_CHROME_NO_SANDBOX=1 is not exported with SCORSESE_CHROME")
    for step in filter(tests, every):
        if re.search(r"ulimit -v\b", step):
            found.append("a hard `ulimit -v` stops Chrome starting: cap with `ulimit -Sv`")
        elif "ulimit -Sv" in step and UNCAPPED not in step:
            found.append("a capped test step does not run the browser through `tools/chromium/uncapped`")
    return found


CACHED = f"""      - name: Chromium cache
        uses: actions/cache@v6
        with:
          {CACHE}
          {KEY}
"""
INSTALL = f"""      - name: Install Chromium
        run: |
          {FETCH}
          {EXPORT}
          {NO_SANDBOX}
"""
TEST = "      - run: cargo nextest run --workspace --locked\n"


class Shape(unittest.TestCase):
    def test_a_job_with_the_browser_passes(self):
        self.assertEqual(problems(job(CACHED, INSTALL, TEST)), [])

    def test_a_job_that_runs_no_test_passes(self):
        for run in (
            "cargo mutants --list --json $ARGS > planned.json",
            "cargo test --manifest-path tools/lint/Cargo.toml --locked",
            "cargo clippy --workspace",
        ):
            with self.subTest(run=run):
                self.assertEqual(problems(job(f"      - run: {run}\n")), [])

    def test_a_test_step_named_only_in_a_comment_passes(self):
        self.assertEqual(problems(job("      - run: |\n          # cargo mutants -p x\n          true\n")), [])

    def test_a_mutation_shard_without_the_browser_is_flagged(self):
        shard = '      - run: |\n          cargo mutants $ARGS --shard "$SHARD/$SHARDS"\n'
        self.assertEqual(len(problems(job(shard))), 2)

    def test_a_browser_set_up_after_the_tests_is_flagged(self):
        got = problems(job(TEST, CACHED, INSTALL))
        self.assertEqual(len(got), 2)

    def test_a_cache_keyed_on_anything_but_the_pin_is_flagged(self):
        got = problems(job(CACHED.replace("tools/chromium/pin", "Cargo.lock"), INSTALL, TEST))
        self.assertEqual(got, ["the Chromium cache is not restored before the first test step"])

    def test_an_export_that_hides_a_failed_download_is_flagged(self):
        hiding = INSTALL.replace(f"{FETCH}\n          {EXPORT}", 'echo "SCORSESE_CHROME=$(tools/chromium/fetch)" >> "$GITHUB_ENV"')
        got = problems(job(CACHED, hiding, TEST))
        self.assertEqual(got, ["`tools/chromium/fetch` is not exported as SCORSESE_CHROME before the first test step"])

    def test_a_capped_test_step_must_lift_the_cap_for_the_browser(self):
        mutate = "      - run: |\n          ulimit -{}v 6291456\n{}          cargo mutants $ARGS\n"
        for flag, export, want in (
            ("S", f"          {UNCAPPED}\n", []),
            ("S", "", ["a capped test step does not run the browser through `tools/chromium/uncapped`"]),
            ("", f"          {UNCAPPED}\n", ["a hard `ulimit -v` stops Chrome starting: cap with `ulimit -Sv`"]),
        ):
            with self.subTest(flag=flag, export=export):
                self.assertEqual(problems(job(CACHED, INSTALL, mutate.format(flag, export))), want)

    def test_a_sandboxed_capture_is_flagged(self):
        got = problems(job(CACHED, INSTALL.replace(f"          {NO_SANDBOX}\n", ""), TEST))
        self.assertEqual(got, ["SCORSESE_CHROME_NO_SANDBOX=1 is not exported with SCORSESE_CHROME"])


class Workflow(unittest.TestCase):
    def test_every_job_that_runs_tests_has_the_browser(self):
        seen = []
        for path in sorted([*WORKFLOWS.glob("*.yml"), *WORKFLOWS.glob("*.yaml")]):
            text = path.read_text(encoding="utf-8")
            if "\njobs:\n" not in text:
                continue
            for name, block in jobs(text).items():
                if not any(map(tests, steps(block))):
                    continue
                seen.append(f"{path.stem}/{name}")
                with self.subTest(job=f"{path.name}: {name}"):
                    self.assertEqual(problems(block), [])
        for name in ("ci/check", "ci/regression", "coverage/coverage", "mutants-on-request/shard", "mutation-sweep/shard"):
            self.assertIn(name, seen)


if __name__ == "__main__":
    unittest.main()
