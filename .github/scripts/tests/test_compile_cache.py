#!/usr/bin/env python3
"""`make compile-cache` writes the worktrees' sccache config only when sccache is there.

`make setup` runs it (#1475, ported from MatthewLacerda2/rusty#492). A machine
with sccache gets a cargo config naming it by absolute path, since cargo did
not find a bare `sccache` from a config file; a machine without it, which is CI
and every cloud session, must get nothing written, or its builds would fail on
a wrapper that is not installed. Both cases run the real target, with `PATH`
narrowed to a directory the test controls so the host's own sccache, if it has
one, decides nothing.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def compile_cache(bin_dir: Path, config: Path) -> subprocess.CompletedProcess:
    make = shutil.which("make")
    assert make, "make is not on PATH"
    return subprocess.run(
        [make, "-s", "--no-print-directory", "compile-cache", f"CACHE_CONFIG={config}"],
        cwd=ROOT,
        env={**os.environ, "PATH": str(bin_dir)},
        capture_output=True,
        text=True,
        check=True,
    )


@unittest.skipUnless(shutil.which("make"), "make is not installed")
class CompileCache(unittest.TestCase):
    def setUp(self) -> None:
        self.dir = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.dir)
        self.bin = self.dir / "bin"
        self.bin.mkdir()
        mkdir = shutil.which("mkdir")
        assert mkdir
        (self.bin / "mkdir").symlink_to(mkdir)
        self.config = self.dir / "worktrees/.cargo/config.toml"

    def test_without_sccache_nothing_is_written(self) -> None:
        run = compile_cache(self.bin, self.config)
        self.assertIn("compile-cache: off", run.stdout)
        self.assertFalse(self.config.parent.exists())

    def test_with_sccache_the_wrapper_is_its_absolute_path(self) -> None:
        sccache = self.bin / "sccache"
        sccache.write_text("#!/bin/sh\n")
        sccache.chmod(0o755)
        run = compile_cache(self.bin, self.config)
        self.assertIn("compile-cache: on", run.stdout)
        written = tomllib.loads(self.config.read_text(encoding="utf-8"))
        self.assertEqual(written["build"]["rustc-wrapper"], str(sccache))
        self.assertEqual(written["env"]["SCCACHE_CACHE_SIZE"], "2G")


if __name__ == "__main__":
    unittest.main()
