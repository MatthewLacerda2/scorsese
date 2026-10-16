#!/usr/bin/env python3
"""Every tool CI installs is the version cloud sessions install.

A gate's tool is installed twice: by a workflow step for CI, and by
`.claude/hooks/session-start.sh` for a cloud coder's `make gates`. When the two
differ, a gate can be green in the session and red in CI, or the reverse:
`terraform fmt`'s output has changed between releases, and CI's terraform used
to float to the latest one while the hook pinned 1.16.6 (#1370). Each place
said "bump both together"; this holds them to it.

`PINS`, in `.github/scripts/pins.py`, is the table to extend.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SCRIPTS = ROOT / ".github/scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from pins import HOOK, WORKFLOWS, disagreements, hook_pins  # noqa: E402

# The hook's pins as the cases below assume them.
HOOK_PINS = {"TERRAFORM": "1.16.6", "DENY": "0.20.2", "NEXTEST": "0.9.146", "MUTANTS": "27.1.0"}


class ToolPins(unittest.TestCase):
    def test_ci_installs_what_the_hook_installs(self) -> None:
        hook = hook_pins((ROOT / HOOK).read_text(encoding="utf-8"))
        workflows = {
            f"{WORKFLOWS}/{p.name}": p.read_text(encoding="utf-8")
            for p in sorted((ROOT / WORKFLOWS).glob("*.yml"))
        }
        self.assertEqual(disagreements(hook, workflows), [])

    def test_a_floating_or_wrong_version_is_named(self) -> None:
        workflow = """jobs:
  infra:
    steps:
      - uses: hashicorp/setup-terraform@v3
        with:
          terraform_wrapper: false
      - name: Install cargo-deny
        uses: taiki-e/install-action@v2
        with:
          tool: cargo-deny@0.19.0
      - name: Install cargo-nextest
        uses: taiki-e/install-action@nextest
      - uses: taiki-e/install-action@v2
        with:
          tool: cargo-mutants@27.0.0
"""
        self.assertEqual(
            disagreements(HOOK_PINS, {"ci.yml": workflow}),
            [
                "ci.yml installs terraform with no version; the hook pins 1.16.6",
                "ci.yml installs cargo-deny 0.19.0; the hook pins 0.20.2",
                "ci.yml installs cargo-nextest with no version; the hook pins 0.9.146",
                "ci.yml installs cargo-mutants 27.0.0; the hook pins 27.1.0",
            ],
        )

    def test_agreeing_pins_pass_and_a_missing_one_is_named(self) -> None:
        workflow = """jobs:
  infra:
    steps:
      - uses: hashicorp/setup-terraform@v3
        with:
          terraform_version: "1.16.6"
      - uses: taiki-e/install-action@v2
        with:
          tool: cargo-deny@0.20.2,cargo-mutants@27.1.0
      - uses: taiki-e/install-action@v2
        with:
          tool: cargo-nextest@0.9.146
      - run: cargo nextest run && cargo mutants
"""
        hook = {k: v for k, v in HOOK_PINS.items() if k != "DENY"}
        self.assertEqual(disagreements(HOOK_PINS, {"ci.yml": workflow}), [])
        self.assertEqual(
            disagreements(hook, {"ci.yml": workflow}),
            [f"{HOOK} does not pin cargo-deny (DENY=)"],
        )

    def test_a_step_ends_where_the_next_begins(self) -> None:
        workflow = """jobs:
  a:
    steps:
      - uses: hashicorp/setup-terraform@v3
      - name: Other
        with:
          terraform_version: 1.16.6
  b:
    runs-on: x
"""
        self.assertEqual(
            disagreements(HOOK_PINS, {"ci.yml": workflow}),
            [
                "ci.yml installs terraform with no version; the hook pins 1.16.6",
                "no workflow installs cargo-deny: is PINS's pattern for it stale?",
                "no workflow installs cargo-nextest: is PINS's pattern for it stale?",
                "no workflow installs cargo-mutants: is PINS's pattern for it stale?",
            ],
        )


if __name__ == "__main__":
    unittest.main()
