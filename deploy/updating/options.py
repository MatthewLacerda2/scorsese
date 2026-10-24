"""`deploy/update`'s flags."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Sequence


def parse(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="deploy/update",
        description="Update the web app from main: pull, build, dump, restart, wait for health, "
        "purge Cloudflare's landing page, check, and say what changed (docs/web.md, Updating).",
    )
    parser.add_argument("--dry-run", action="store_true",
                        help="check everything and print the steps, changing nothing: "
                        "fetches instead of pulling, and builds, dumps and purges nothing")
    parser.add_argument("--wait", action="store_true",
                        help="while jobs or assistant turns are running, wait for them to "
                        "finish instead of refusing")
    parser.add_argument("--wait-timeout", type=int, default=7200, metavar="SECONDS",
                        help="under --wait, how long to wait for running work before "
                        "giving up (default 7200, two hours)")
    parser.add_argument("--timeout", type=int, default=600, metavar="SECONDS",
                        help="how long the services have to become healthy after the "
                        "restart (default 600)")
    parser.add_argument("--from", dest="running", metavar="COMMIT",
                        help="the commit the stack runs, when its server image does not "
                        "say (one built before deploy/update labelled its images)")
    parser.add_argument("--env-file", type=Path, metavar="PATH",
                        help="the stack's settings (default: DEPLOY_ENV, else deploy/.env)")
    return parser.parse_args(list(argv))
