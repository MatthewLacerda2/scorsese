#!/usr/bin/env python3
"""`.github/labels.json` is every label the repository's machinery names.

The *Sync labels* workflow creates what that file lists, and GitHub quietly
ignores a label a repository does not have: Dependabot opens its pull request
unlabelled, and `make queue` never sees one waiting. So every label something
relies on must be in the file (#1226):

- what `dependabot.yml` puts on its pull requests;
- `merge-watch.py`'s go-ahead (`QUEUE_LABEL`) and its ranks (`PRIORITY`);
- `closing.py`'s stage labels (`STAGES`);
- every label a skill or a `CLAUDE.md` tells an agent to put on (`` `x` label``,
  ``labelled `x` ``).

Read as text, not imported: `merge-watch.py` loads the merge path when imported.
"""

from __future__ import annotations

import ast
import json
import os
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SCRIPTS = ROOT / ".github/scripts"
# The forms prose names a label in; `architecture`-label work is one too.
SKIP = {"target", "node_modules", "dist"}
PROSE = re.compile(r"`([a-z][a-z-]*)`[- ]labels?\b|\blabel(?:led|s)? \**`([a-z][a-z-]*)`")


def assigned(source: str, name: str) -> object:
    """The literal a module assigns to `name` at its top level."""
    for node in ast.parse(source).body:
        if isinstance(node, ast.Assign) and any(
            isinstance(t, ast.Name) and t.id == name for t in node.targets
        ):
            return ast.literal_eval(node.value)
    raise KeyError(name)


def in_prose(text: str) -> set[str]:
    return {a or b for a, b in PROSE.findall(text)}


def in_dependabot(text: str) -> set[str]:
    return {
        label
        for found in re.findall(r"^\s*labels:\s*\[([^\]]*)\]", text, re.M)
        for label in re.findall(r"[\"']([^\"']+)[\"']", found)
    }


def claude_mds() -> list[Path]:
    """Every `CLAUDE.md`, without walking a build directory's millions of files."""
    found = []
    for directory, subdirectories, files in os.walk(ROOT):
        subdirectories[:] = [d for d in subdirectories if d not in SKIP and not d.startswith(".")]
        found += [Path(directory) / f for f in files if f == "CLAUDE.md"]
    return sorted(found)


def named() -> dict[str, set[str]]:
    """Each place, and the labels it names."""
    watch = (SCRIPTS / "merge-watch.py").read_text(encoding="utf-8")
    places = {
        ".github/dependabot.yml": in_dependabot((ROOT / ".github/dependabot.yml").read_text(encoding="utf-8")),
        "merge-watch.py": {assigned(watch, "QUEUE_LABEL"), *assigned(watch, "PRIORITY")},
        "closing.py": set(assigned((SCRIPTS / "closing.py").read_text(encoding="utf-8"), "STAGES")),
    }
    for path in [*ROOT.glob(".claude/skills/**/*.md"), *claude_mds()]:
        places[str(path.relative_to(ROOT))] = in_prose(path.read_text(encoding="utf-8"))
    return places


def problems(places: dict[str, set[str]], defined: set[str]) -> list[str]:
    return [
        f"{place} names the label `{label}`, which .github/labels.json does not define"
        for place, labels in sorted(places.items())
        for label in sorted(labels - defined)
    ]


class Labels(unittest.TestCase):
    def test_every_label_named_is_defined(self) -> None:
        defined = {l["name"] for l in json.loads((ROOT / ".github/labels.json").read_text(encoding="utf-8"))}
        self.assertEqual(problems(named(), defined), [])

    def test_prose_and_dependabot_forms_are_read(self) -> None:
        text = "carries the `agent` label; `architecture`-label work; labelled **`queue`**; label `minor`"
        self.assertEqual(in_prose(text), {"agent", "architecture", "queue", "minor"})
        self.assertEqual(in_dependabot('    labels: ["queue", "dependencies"]\n'), {"queue", "dependencies"})
        self.assertEqual(assigned('PRIORITY = {"bug": 2}\nX = 1\n', "PRIORITY"), {"bug": 2})

    def test_an_undefined_label_is_named(self) -> None:
        self.assertEqual(
            problems({"a.md": {"agent", "idea"}, "b.py": {"queue"}}, {"agent", "queue"}),
            ["a.md names the label `idea`, which .github/labels.json does not define"],
        )


if __name__ == "__main__":
    unittest.main()
