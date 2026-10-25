#!/usr/bin/env python3
"""The web stack's logs survive a deploy, and the healthcheck stays out of them.

Every service in `deploy/compose.yaml` logs to journald under the tag
`scorsese-<service>` (#1526): Docker's default `json-file` belongs to the
container, so each deploy, which recreates `server` and `web`, wiped the
history. And `web`'s healthcheck asks nginx for a path whose `location` turns
the access log off, so a line every 30 seconds does not drown the visitors.
"""

from __future__ import annotations

import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
COMPOSE = ROOT / "deploy/compose.yaml"
NGINX = ROOT / "deploy/nginx.conf"

SERVICE = re.compile(r"^  ([a-z][a-z0-9-]*):\s*$")
SECTION = re.compile(r"^\S")


def services(compose: str) -> dict[str, str]:
    """Each service under `services:`, with the text of its block."""
    blocks: dict[str, list[str]] = {}
    inside = False
    current = None
    for line in compose.splitlines():
        if SECTION.match(line):
            inside = line.startswith("services:")
            current = None
            continue
        if not inside:
            continue
        named = SERVICE.match(line)
        if named:
            current = named.group(1)
            blocks[current] = []
        elif current:
            blocks[current].append(line)
    return {name: "\n".join(lines) for name, lines in blocks.items()}


def logging_problems(compose: str) -> list[str]:
    """One line per service that would not log to journald under its tag."""
    problems = []
    if not re.search(r"^x-journald: &journald\n  driver: journald$", compose, re.MULTILINE):
        problems.append("compose.yaml has no `x-journald: &journald` anchor with `driver: journald`")
    for name, block in services(compose).items():
        logging = re.search(r"^    logging:\n((?:      .*\n?)+)", block + "\n", re.MULTILINE)
        if not logging:
            problems.append(f"service `{name}` has no `logging:`: add `<<: *journald` and its tag")
            continue
        body = logging.group(1)
        if "<<: *journald" not in body:
            problems.append(f"service `{name}`'s logging does not merge `*journald`")
        if not re.search(rf"^        tag: scorsese-{re.escape(name)}$", body, re.MULTILINE):
            problems.append(f"service `{name}`'s logging is not tagged `scorsese-{name}`")
    return problems


def healthcheck_path(compose: str) -> str | None:
    """The path `web`'s healthcheck asks nginx for."""
    block = services(compose).get("web", "")
    found = re.search(r'"http://127\.0\.0\.1(/[^"]*)"', block)
    return found.group(1) if found else None


def unlogged(nginx: str, path: str) -> bool:
    """Whether an exact `location` for `path` turns nginx's access log off."""
    found = re.search(rf"location = {re.escape(path)} \{{([^}}]*)\}}", nginx)
    return bool(found) and re.search(r"^\s*access_log off;", found.group(1), re.MULTILINE) is not None


class Logging(unittest.TestCase):
    def test_every_service_logs_to_journald_under_its_tag(self) -> None:
        self.assertEqual(logging_problems(COMPOSE.read_text()), [])

    def test_the_web_healthcheck_is_not_in_the_access_log(self) -> None:
        path = healthcheck_path(COMPOSE.read_text())
        self.assertIsNotNone(path, "web's healthcheck asks nginx for no path")
        self.assertTrue(
            unlogged(NGINX.read_text(), path),
            f"nginx.conf has no `location = {path}` with `access_log off;`",
        )


CLEAN = """\
x-journald: &journald
  driver: journald

services:
  web:
    image: nginx
    logging:
      <<: *journald
      options:
        tag: scorsese-web
    healthcheck:
      test: ["CMD", "wget", "--spider", "http://127.0.0.1/healthz"]

volumes:
  data:
"""


class Checks(unittest.TestCase):
    def test_a_compliant_file_passes(self) -> None:
        self.assertEqual(logging_problems(CLEAN), [])
        self.assertEqual(healthcheck_path(CLEAN), "/healthz")

    def test_a_service_without_logging_is_named(self) -> None:
        compose = CLEAN + "\n".join(["", "services:", "  server:", "    image: x", ""])
        self.assertIn(
            "service `server` has no `logging:`: add `<<: *journald` and its tag",
            logging_problems(compose),
        )

    def test_a_wrong_tag_is_named(self) -> None:
        compose = CLEAN.replace("tag: scorsese-web", "tag: web")
        self.assertEqual(
            logging_problems(compose), ["service `web`'s logging is not tagged `scorsese-web`"]
        )

    def test_a_missing_anchor_is_named(self) -> None:
        compose = CLEAN.replace("  driver: journald\n", "  driver: json-file\n")
        self.assertIn("anchor", logging_problems(compose)[0])

    def test_the_healthcheck_location_must_turn_the_log_off(self) -> None:
        self.assertTrue(unlogged("location = /healthz {\n    access_log off;\n}", "/healthz"))
        self.assertFalse(unlogged("location = /healthz {\n    return 200;\n}", "/healthz"))
        self.assertFalse(unlogged("location / {\n    access_log off;\n}", "/healthz"))


if __name__ == "__main__":
    unittest.main()
