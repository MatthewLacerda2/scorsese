"""Everything `deploy/update` does to the machine, behind one object.

The steps never call `subprocess`, `urllib` or the clock themselves; they ask
a `Host`. The tests hand them a scripted one instead.
"""

from __future__ import annotations

import os
import subprocess
import sys
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Mapping, Optional, Sequence


class Refusal(Exception):
    """A step that cannot go on, said in a sentence for the operator."""


@dataclass
class Result:
    """A finished command: its exit code and what it printed."""

    code: int
    out: str = ""
    err: str = ""

    @property
    def ok(self) -> bool:
        return self.code == 0


class Host:
    """The real machine."""

    def run(self, args: Sequence[str], cwd: Optional[Path] = None) -> Result:
        """Run a command and keep what it printed."""
        try:
            done = subprocess.run(list(args), cwd=cwd, capture_output=True, text=True)
        except FileNotFoundError:
            return Result(127, "", f"{args[0]} is not installed")
        return Result(done.returncode, done.stdout, done.stderr)

    def stream(
        self,
        args: Sequence[str],
        cwd: Optional[Path] = None,
        env: Optional[Mapping[str, str]] = None,
    ) -> int:
        """Run a command with its output on the terminal, for the long ones."""
        merged = dict(os.environ, **(env or {}))
        sys.stdout.flush()
        try:
            return subprocess.run(list(args), cwd=cwd, env=merged).returncode
        except FileNotFoundError:
            print(f"update: {args[0]} is not installed", file=sys.stderr)
            return 127

    def get(self, url: str) -> str:
        """An address's answer: its HTTP status, or why there was none."""
        request = urllib.request.Request(url, headers={"User-Agent": "scorsese-deploy-update"})
        try:
            with urllib.request.urlopen(request, timeout=10) as response:
                return str(response.status)
        except urllib.error.HTTPError as error:
            return str(error.code)
        except (urllib.error.URLError, OSError) as error:
            return f"no answer ({getattr(error, 'reason', error)})"

    def env(self, name: str) -> Optional[str]:
        return os.environ.get(name)

    def cores(self) -> int:
        return os.cpu_count() or 1

    def now(self) -> float:
        return time.time()

    def sleep(self, seconds: float) -> None:
        time.sleep(seconds)


def read_env(path: Path) -> dict:
    """`deploy/.env`'s values, read the way `purge-landing` reads them.

    The last assignment wins and one pair of surrounding quotes is dropped;
    there is no interpolation, which `.env.example` never uses.
    """
    values = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        name, value = line.split("=", 1)
        if len(value) >= 2 and value[0] == value[-1] and value[0] in "'\"":
            value = value[1:-1]
        values[name.strip()] = value
    return values
