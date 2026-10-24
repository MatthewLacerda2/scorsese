#!/usr/bin/env python3
"""The new images wait under the commit's tag until the restart, an interrupted
run still says where things stand, and health and the purge read what is set.
"""

from __future__ import annotations

import os
import re
import signal
import unittest

from .scripted import APPLIED, BUSY, DUMPED, ENV, GIT, IDLE, PS, ROOT, Result, Scripted, stack, update

PULLED = GIT + [(["pull"], Result(0)), APPLIED]
HEALTHY = (["--format json"], Result(0, PS))


def tagging(host):
    return [c for c in host.ran if c.startswith("docker tag") or c.startswith("docker image rm")]


class Tags(unittest.TestCase):
    def test_a_refusal_after_the_build_never_retags(self):
        host = Scripted(PULLED + [(["chat_turns"], BUSY)])
        code, _, err = update(host)
        self.assertEqual(code, 1)
        self.assertEqual(host.envs[0][stack.TAG], "aaaa")
        self.assertFalse(tagging(host))
        self.assertIn("with `latest` left as it was", err)

    def test_the_restart_moves_latest_to_every_built_image_first(self):
        host = Scripted(PULLED + [IDLE, (["backup backup"], DUMPED), HEALTHY])
        code, _, err = update(host)
        self.assertEqual(code, 0, err)
        up = next(i for i, c in enumerate(host.ran) if "up --detach" in c)
        for image in stack.BUILT:
            tag = host.ran.index(f"docker tag {image}:aaaa {image}:latest")
            self.assertLess(tag, host.ran.index(f"docker image rm {image}:aaaa"))
            self.assertLess(host.ran.index(f"docker image rm {image}:aaaa"), up)
        self.assertEqual([e for e in host.envs if stack.TAG in e], [host.envs[0]])

    def test_a_failed_retag_does_not_restart(self):
        host = Scripted(PULLED + [IDLE, (["backup backup"], DUMPED),
                                  (["docker tag"], Result(1, "", "no such image"))])
        code, _, err = update(host)
        self.assertEqual(code, 1)
        self.assertIn("so the stack was not restarted", err)
        self.assertIn("some may already be `latest`", err)
        self.assertFalse([c for c in host.ran if "up --detach" in c])

    def test_compose_tags_every_built_image_by_the_variable(self):
        compose = (ROOT / "deploy" / "compose.yaml").read_text()
        tagged = re.findall(r"image: (scorsese-[a-z-]+):\$\{" + stack.TAG + r":-latest\}", compose)
        self.assertEqual(sorted(tagged), sorted(stack.BUILT))
        self.assertEqual(len(re.findall(r"^    build:", compose, re.M)), len(stack.BUILT))


class Interrupted(Scripted):
    """A build stopped by `signum`, sent to this process, or Ctrl-C when None."""

    def __init__(self, rules, signum=None):
        super().__init__(rules)
        self.signum = signum

    def stream(self, args, cwd=None, env=None):
        if " build" in " ".join(args):
            if self.signum is None:
                raise KeyboardInterrupt
            os.kill(os.getpid(), self.signum)
        return super().stream(args, cwd, env)


class Signals(unittest.TestCase):
    def test_ctrl_c_sigterm_and_sighup_say_where_things_stand(self):
        for signum in (None, signal.SIGTERM, signal.SIGHUP):
            with self.subTest(signum=signum):
                before = signal.getsignal(signal.SIGTERM)
                host = Interrupted(PULLED, signum)
                code, _, err = update(host)
                self.assertEqual(code, 1)
                self.assertIn("interrupted\nthe checkout is at aaaa and some images may", err)
                self.assertFalse([c for c in host.ran if "backup" in c or "up --detach" in c])
                self.assertEqual(signal.getsignal(signal.SIGTERM), before)


class Settings(unittest.TestCase):
    def test_a_declared_service_with_no_container_is_not_healthy(self):
        self.assertEqual(stack.not_ready(stack.services(PS), ["server", "tunnel", "web"]),
                         ["web (no container)"])
        host = Scripted([(["config", "--services"], Result(0, "server\ntunnel\nweb\n"))]
                        + PULLED + [IDLE, (["backup backup"], DUMPED), HEALTHY])
        code, _, err = update(host, "--timeout", "0")
        self.assertEqual(code, 1)
        self.assertIn("not healthy after 0s: web (no container)", err)

    def test_no_public_origin_skips_the_purge(self):
        env = ENV.replace("SCORSESE_PUBLIC_ORIGIN='https://example.org'\n", "")
        host = Scripted(PULLED + [IDLE, (["backup backup"], DUMPED), HEALTHY])
        code, out, err = update(host, env=env)
        self.assertEqual(code, 0, err)
        self.assertIn("no SCORSESE_PUBLIC_ORIGIN in the settings", out)
        self.assertFalse([c for c in host.ran if "purge-landing" in c])


if __name__ == "__main__":
    unittest.main()
