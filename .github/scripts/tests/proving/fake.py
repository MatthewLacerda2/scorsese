"""A fake `gh api` for `proved_tree`'s tests: canned bodies keyed by path prefix,
so nothing reaches GitHub (#1758)."""

from __future__ import annotations

import sys
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[2]
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

import proved_tree  # noqa: E402

REPO, SHA, BEFORE, TREE = "o/r", "s" * 40, "b" * 40, "t" * 40


def job(name: str, step: str | None, conclusion: str = "success", step_concl: str = "success") -> dict:
    steps = [{"name": "Set up job", "conclusion": "success"}]
    if step:
        steps.append({"name": step, "conclusion": step_concl})
    return {"name": name, "conclusion": conclusion, "steps": steps}


def built_jobs(**over) -> list[dict]:
    """Every job `BUILT` names, each with its step run; `over` replaces by name."""
    jobs = {name: job(name, step) for name, step in proved_tree.BUILT.items()}
    jobs.update(over)
    return [j for j in jobs.values() if j is not None]


class Fake:
    """`gh api`: the answer for the first prefix a path starts with, else `None`."""

    def __init__(self, jobs=None, tree=TREE, files=("crates/core/src/lib.rs",), status="ahead", fail=(), tip=SHA,
                 later=None, base="main"):
        run = {"id": 7, "html_url": "http://run/7", "head_commit": {"tree_id": tree},
               "event": "pull_request", "pull_requests": [{"base": {"ref": base}}]}
        self.answers = {
            f"repos/{REPO}/compare/{SHA}...": {"status": "ahead", "files": [{"filename": f} for f in
                                                                           (files if later is None else later)]},
            f"repos/{REPO}/compare/": {"status": status, "files": [{"filename": f} for f in files]},
            f"repos/{REPO}/commits/main": {"sha": tip},
            f"repos/{REPO}/commits/{SHA}": {"commit": {"tree": {"sha": TREE}}},
            f"repos/{REPO}/actions/workflows/ci.yml/": {"workflow_runs": [run]},
            f"repos/{REPO}/actions/runs/7/jobs": {"jobs": built_jobs() if jobs is None else jobs},
        }
        self.fail = fail
        self.asked: list[str] = []

    def __call__(self, path: str):
        self.asked.append(path)
        for prefix, answer in self.answers.items():
            if path.startswith(prefix):
                return None if any(prefix.startswith(f) for f in self.fail) else answer
        return None


def verdict(fake: Fake, before: str = BEFORE):
    return proved_tree.verdict(fake, REPO, SHA, before)
