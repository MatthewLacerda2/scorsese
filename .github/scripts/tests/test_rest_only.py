#!/usr/bin/env python3
"""The merge path asks GitHub through REST only (#1724).

A cloud container's proxy answers every GraphQL call with a 403, and `gh pr
…`, `gh issue view --json`, `gh repo view --json` and `gh run view --json` are
GraphQL underneath, or need it to find the repository. So every `gh` the merge
path runs, as a command list or through `mergeable.gh`, is `gh api` and not
`gh api graphql`.
"""

from __future__ import annotations

import ast
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
MERGE_PATH = [
    "merge-queue.py", "merge-watch.py", "mergeable.py", "described.py", "closing.py",
    "reblessed.py", "reviewed.py", "protocol.py", "pulls.py",
    *sorted(str(p.relative_to(SCRIPTS)) for p in (SCRIPTS / "merge_queue").glob("*.py")),
]


def text(node: ast.AST) -> str | None:
    return node.value if isinstance(node, ast.Constant) and isinstance(node.value, str) else None


def not_rest(source: str) -> list[str]:
    """Each `gh` call in `source` that is not REST (`api`, never `api graphql`), by line."""
    found = []
    for node in ast.walk(ast.parse(source)):
        if isinstance(node, ast.List) and len(node.elts) > 1 and text(node.elts[0]) == "gh":
            words = [text(e) for e in node.elts[1:3]]
        elif isinstance(node, ast.Call) and node.args and (
            getattr(node.func, "id", None) == "gh" or getattr(node.func, "attr", None) == "gh"
        ):
            words = [text(a) for a in node.args[:2]]
        else:
            continue
        if words[0] is not None and words[0] != "api":
            found.append((node.lineno, words[0]))
        elif words[0] == "api" and words[1:] == ["graphql"]:
            found.append((node.lineno, "api graphql"))
    return [f"line {line}: gh {sub}" for line, sub in sorted(found)]


class RestOnly(unittest.TestCase):
    def test_it_flags_a_graphql_command(self):
        source = ('run(["gh", "pr", "merge", n])\nmergeable.gh("issue", "view", n)\ngh("repo", "view")\n'
                  'run(["gh", "api", "graphql", "-f", q])\nmergeable.gh("api", "graphql")\n')
        self.assertEqual(not_rest(source), ["line 1: gh pr", "line 2: gh issue", "line 3: gh repo",
                                            "line 4: gh api graphql", "line 5: gh api graphql"])

    def test_it_passes_rest(self):
        self.assertEqual(not_rest('run(["gh", "api", "-X", "PUT", p])\nmergeable.gh("api", p)\n'), [])

    def test_the_merge_path_asks_rest_only(self):
        for name in MERGE_PATH:
            with self.subTest(script=name):
                found = not_rest((SCRIPTS / name).read_text(encoding="utf-8"))
                self.assertEqual(found, [], f"{name} runs `gh` other than `gh api`, which a"
                                 " cloud container's proxy refuses (GraphQL): ask the REST"
                                 " endpoint through `gh api repos/…` and map it in `pulls.py`.")


if __name__ == "__main__":
    unittest.main()
