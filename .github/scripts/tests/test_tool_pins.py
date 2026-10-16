#!/usr/bin/env python3
"""Every tool CI installs is the version cloud sessions install.

A gate's tool is installed twice: by a workflow step for CI, and by
`.claude/hooks/session-start.sh` for a cloud coder's `make gates`. When the two
differ, a gate can be green in the session and red in CI, or the reverse:
`terraform fmt`'s output has changed between releases, and CI's terraform used
to float to the latest one while the hook pinned 1.16.6 (#1370). Each place
said "bump both together"; this holds them to it.

`PINS` is the table to extend (#1226 adds the other tools): the hook variable
that pins a tool, how to recognise a workflow step that installs it, and where
that step names its version. A step that installs the tool and names no version
floats, and is flagged like a wrong version.
"""

from __future__ import annotations

import re
import unittest
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
HOOK = ".claude/hooks/session-start.sh"
WORKFLOWS = ".github/workflows"


@dataclass(frozen=True)
class Pin:
    tool: str
    hook_variable: str
    installs: re.Pattern[str]
    version: re.Pattern[str]


PINS = (
    Pin(
        "terraform",
        "TERRAFORM",
        re.compile(r"uses:\s*hashicorp/setup-terraform@"),
        re.compile(r"terraform_version:\s*[\"']?([^\"'\s]+)"),
    ),
    Pin(
        "cargo-deny",
        "DENY",
        re.compile(r"\bcargo-deny(?:@|\s*$)", re.MULTILINE),
        re.compile(r"\bcargo-deny@([^\s,\"']+)"),
    ),
)


def hook_pins(text: str) -> dict[str, str]:
    """The hook's `NAME=value` assignments at the start of a line."""
    return {m.group(1): m.group(2) for m in re.finditer(r"^([A-Z_0-9]+)=(\S+)$", text, re.MULTILINE)}


def steps(text: str) -> list[str]:
    """A workflow's steps, each the text of one `- ` item under a `steps:` key.

    Text, not YAML: the stdlib has no YAML parser, and a step is a `- ` line
    plus every line indented deeper than its dash.
    """
    found: list[str] = []
    lines = text.splitlines()
    for i, line in enumerate(lines):
        if not re.match(r"^\s*steps:\s*$", line):
            continue
        under = len(line) - len(line.lstrip())
        current: list[str] | None = None
        dash = -1
        for later in lines[i + 1 :]:
            indent = len(later) - len(later.lstrip())
            if later.strip() and indent <= under:
                break
            if later.lstrip().startswith("- ") and (current is None or indent <= dash):
                if current is not None:
                    found.append("\n".join(current))
                current, dash = [later], indent
            elif current is not None:
                current.append(later)
        if current is not None:
            found.append("\n".join(current))
    return found


def disagreements(hook: dict[str, str], workflows: dict[str, str]) -> list[str]:
    """What is wrong, one line per problem: an unpinned tool, a floating or wrong step."""
    problems: list[str] = []
    for pin in PINS:
        want = hook.get(pin.hook_variable)
        if want is None:
            problems.append(f"{HOOK} does not pin {pin.tool} ({pin.hook_variable}=)")
            continue
        seen = 0
        for path, text in sorted(workflows.items()):
            for step in steps(text):
                if not pin.installs.search(step):
                    continue
                seen += 1
                got = pin.version.search(step)
                if got is None:
                    problems.append(f"{path} installs {pin.tool} with no version; the hook pins {want}")
                elif got.group(1) != want:
                    problems.append(f"{path} installs {pin.tool} {got.group(1)}; the hook pins {want}")
        if not seen:
            problems.append(f"no workflow installs {pin.tool}: is PINS's pattern for it stale?")
    return problems


class ToolPins(unittest.TestCase):
    def test_ci_installs_what_the_hook_installs(self) -> None:
        hook = hook_pins((ROOT / HOOK).read_text(encoding="utf-8"))
        workflows = {
            f"{WORKFLOWS}/{p.name}": p.read_text(encoding="utf-8")
            for p in sorted((ROOT / WORKFLOWS).glob("*.yml"))
        }
        self.assertEqual(disagreements(hook, workflows), [])

    def test_a_floating_or_wrong_version_is_named(self) -> None:
        hook = {"TERRAFORM": "1.16.6", "DENY": "0.20.2"}
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
"""
        self.assertEqual(
            disagreements(hook, {"ci.yml": workflow}),
            [
                "ci.yml installs terraform with no version; the hook pins 1.16.6",
                "ci.yml installs cargo-deny 0.19.0; the hook pins 0.20.2",
            ],
        )

    def test_agreeing_pins_pass_and_a_missing_one_is_named(self) -> None:
        workflow = """jobs:
  infra:
    steps:
      - uses: hashicorp/setup-terraform@v3
        with:
          terraform_version: "1.16.6"
      - run: make infra
"""
        self.assertEqual(
            disagreements({"TERRAFORM": "1.16.6"}, {"ci.yml": workflow}),
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
            disagreements({"TERRAFORM": "1.16.6", "DENY": "0.20.2"}, {"ci.yml": workflow}),
            [
                "ci.yml installs terraform with no version; the hook pins 1.16.6",
                "no workflow installs cargo-deny: is PINS's pattern for it stale?",
            ],
        )


if __name__ == "__main__":
    unittest.main()
