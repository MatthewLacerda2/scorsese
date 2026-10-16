"""The tool versions CI installs, against the ones the session hook pins.

A gate's tool is installed twice: by a workflow step for CI, and by
`.claude/hooks/session-start.sh` for a cloud coder's `make gates`, and the two
must agree (#1370, #1226). `PINS` is the table to extend: the hook variable
that pins a tool, how to recognise a workflow step that installs it, and where
that step names its version. A step that installs the tool and names no version
floats, and is flagged like a wrong version. `tests/test_tool_pins.py` runs it
over the real files.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

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
    # `taiki-e/install-action@nextest` is the action's floating shorthand for
    # the latest release, so it counts as installing with no version (#1226).
    Pin(
        "cargo-nextest",
        "NEXTEST",
        re.compile(r"install-action@(?:cargo-)?nextest\b|^\s*tool:.*(?<![\w-])(?:cargo-)?nextest\b", re.M),
        re.compile(r"^\s*tool:.*(?<![\w-])(?:cargo-)?nextest@([^\s,\"']+)", re.M),
    ),
    Pin(
        "cargo-mutants",
        "MUTANTS",
        re.compile(r"install-action@cargo-mutants\b|^\s*tool:.*\bcargo-mutants\b", re.M),
        re.compile(r"^\s*tool:.*\bcargo-mutants@([^\s,\"']+)", re.M),
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
