#!/usr/bin/env python3
"""Every session's browser is its own: `tools/playwright-mcp` never shares one.

Each Claude session starts the script, so each gets its own server and browser
as long as nothing makes them one: no `--port` (one server for every session),
no `--shared-browser-context`, the profile in memory, and an output folder of
the session's own. These run the real script in a scratch copy of the tree,
with a stand-in `node` that records what it was asked to run.
"""

from __future__ import annotations

import json
import os
import shutil
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

# Records its arguments, one per line, then sleeps so two runs overlap.
STAND_IN = """#!/bin/sh
out="$RECORD/$$"
for arg in "$@"; do printf '%s\\n' "$arg"; done > "$out"
sleep 0.3
"""


class PlaywrightMcp(unittest.TestCase):
    def setUp(self) -> None:
        self.dir = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.dir)
        tree = self.dir / "tree"
        (tree / "tools").mkdir(parents=True)
        shutil.copy(ROOT / "tools/playwright-mcp", tree / "tools/playwright-mcp")
        cli = tree / "web/node_modules/@playwright/test/cli.js"
        cli.parent.mkdir(parents=True)
        cli.write_text("")
        shutil.copy(ROOT / "web/playwright-mcp.json", tree / "web/playwright-mcp.json")
        self.tree = tree
        bin_dir = self.dir / "bin"
        bin_dir.mkdir()
        node = bin_dir / "node"
        node.write_text(STAND_IN)
        node.chmod(node.stat().st_mode | stat.S_IXUSR)
        self.record = self.dir / "record"
        self.record.mkdir()
        self.env = {
            **os.environ,
            "PATH": f"{bin_dir}:{os.environ['PATH']}",
            "RECORD": str(self.record),
            "E2E_CHROMIUM": "/bin/true",
        }

    def start_two(self) -> list[list[str]]:
        script = str(self.tree / "tools/playwright-mcp")
        runs = [
            subprocess.Popen([script], env=self.env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
            for _ in range(2)
        ]
        for run in runs:
            _, err = run.communicate(timeout=30)
            self.assertEqual(run.returncode, 0, err)
        return [path.read_text().splitlines() for path in sorted(self.record.iterdir())]

    def test_two_sessions_write_to_folders_of_their_own(self) -> None:
        recorded = self.start_two()
        self.assertEqual(len(recorded), 2)
        outputs = [args[args.index("--output-dir") + 1] for args in recorded]
        self.assertNotEqual(outputs[0], outputs[1])
        for output in outputs:
            self.assertEqual(Path(output).parent, self.tree / ".playwright-mcp")
            self.assertTrue(Path(output).is_dir())

    def test_no_session_is_handed_a_shared_server_or_context(self) -> None:
        for args in self.start_two():
            self.assertEqual(args[0], str(self.tree / "web/node_modules/@playwright/test/cli.js"))
            self.assertIn("mcp", args)
            self.assertNotIn("--port", args)
            self.assertNotIn("--shared-browser-context", args)

    def test_the_profile_stays_in_memory(self) -> None:
        config = json.loads((ROOT / "web/playwright-mcp.json").read_text())
        self.assertIs(config["browser"]["isolated"], True)
        self.assertNotIn("userDataDir", config["browser"])
        self.assertNotIn("server", config)


if __name__ == "__main__":
    unittest.main()
