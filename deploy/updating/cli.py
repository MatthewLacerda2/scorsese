"""`deploy/update`: the steps of docs/web.md's *Updating*, in order.

The build comes first and the restart last: `docker compose build` takes the
whole cargo build while the old stack keeps serving, so the running-work check
and the dump are taken just before `up` recreates anything, not twenty minutes
earlier. A step that fails stops the run with a sentence saying what happened
and where the checkout, the images and the running stack each are, and nothing
after it runs. What the stack runs is read off the running server's image,
never the checkout. The server migrates before it serves, so this never runs
migrations; it waits for the health that proves they ran.
"""

from __future__ import annotations

import argparse
import datetime
import signal
import sys
import traceback
from pathlib import Path
from typing import List, Optional, Sequence

from . import stack
from .finish import Finish
from .spool import Spool
from .host import Host, Refusal, Result, read_env
from .options import parse

DEPLOY = Path(__file__).resolve().parent.parent


class Update(Finish, Spool):
    """One run: the options, the machine, and what it learns on the way."""

    POLL = 30  # seconds between looks at the running work, under --wait
    HEALTH_POLL = 5
    DUMP_TIMEOUT = 1800  # a pg_dump once hung for seven hours

    def __init__(self, opts: argparse.Namespace, host: Host, deploy: Path = DEPLOY):
        self.opts, self.host, self.deploy = opts, host, deploy
        self.root = deploy.parent
        self.env_file = (opts.env_file or Path(host.env("DEPLOY_ENV") or deploy / ".env")).resolve()
        self.before = self.after = self.dump_path = ""
        # How far the run got: "pulled", "building", "built", "retagging", "restarted".
        self.stage = ""

    def say(self, line: str = "") -> None:
        print(line, flush=True)

    def step(self, n: int, what: str) -> None:
        self.say(f"\n== {n}. {what}{' (dry run)' if self.opts.dry_run else ''}")

    def git(self, *args: str) -> Result:
        return self.host.run(["git", *args], cwd=self.root)

    def compose(self, args: Sequence[str], timeout: Optional[float] = None) -> Result:
        return self.host.run(self.compose_args(args), cwd=self.deploy, timeout=timeout)

    def compose_args(self, args: Sequence[str]) -> List[str]:
        return ["docker", "compose", "--env-file", str(self.env_file), *args]

    def run(self) -> None:
        self.preconditions()
        self.before = self.running()
        self.say(f"the stack runs {self.before[:12]}: if this update goes wrong, "
                 "that is the commit to go back to")
        self.after = self.pull()
        self.stage = "pulled"
        try:
            self.changes()
            self.build()
            applied = self.drain_and_dump()
            started = self.start()
            self.healthy()
            self.purge()
            self.check()
            self.summary(applied, started)
        except Refusal as refusal:
            raise Refusal(f"{refusal}\n{self.where()}") from None
        except KeyboardInterrupt:
            raise Refusal(f"interrupted\n{self.where()}") from None
        except Exception:
            traceback.print_exc()
            raise Refusal(f"something unexpected went wrong (the traceback is above)\n"
                          f"{self.where()}") from None

    def where(self) -> str:
        """Where things stand after a failure past the pull, and the way back once there is one."""
        if self.opts.dry_run:
            return "(dry run: nothing was pulled, built or restarted)"
        after, before = self.after[:12], self.before[:12]
        dump = f"; the dump {self.dump_path} was taken" if self.dump_path else ""
        if self.stage == "restarted":
            return (f"the stack was restarted on {after}; the way back: check out {before}, "
                    f"rebuild, and restore {self.dump_path} (docs/web.md, Restoring)")
        images = {
            "pulled": "the images are as they were",
            "building": f"some images may already be built from it as `:{self.tag()}`, "
                        "with `latest` left as it was",
            "built": f"its images are built as `:{self.tag()}`, with `latest` left as it was",
            "retagging": "its images are built and some may already be `latest` (page "
                         "captures may run the new server image)",
        }[self.stage]
        return (f"the checkout is at {after} and {images}, while the running containers "
                f"are still {before}'s{dump}")

    def preconditions(self) -> None:
        if not self.env_file.is_file():
            raise Refusal(f"there is no settings file at {self.env_file}; docs/web.md's "
                          "first-time setup makes it from deploy/.env.example")
        self.env = read_env(self.env_file)
        branch = self.git("rev-parse", "--abbrev-ref", "HEAD")
        if not branch.ok:
            raise Refusal(f"{self.root} is not a git checkout: {branch.err.strip()}")
        if branch.out.strip() != "main":
            raise Refusal(f"the checkout is on `{branch.out.strip()}`, not `main`; "
                          "only main is deployed, so `git switch main` first")
        dirty = self.git("status", "--porcelain").out.rstrip()
        if dirty:
            raise Refusal("the checkout has changes git has not committed, so the build would "
                          "not be what main says; commit, stash or remove them first:\n" + dirty)

    def running(self) -> str:
        """The commit the stack runs: its server image's label, else `--from`."""
        commit, said = self.opts.running, "--from"
        if not commit:
            commit, said = self.label(), "the running server image's label"
        known = self.git("rev-parse", "--verify", "--quiet", f"{commit}^{{commit}}")
        if not known.ok:
            raise Refusal(f"{commit}, from {said}, is not a commit this checkout has; "
                          "pass --from with the commit the stack was built from")
        return known.out.strip()

    def label(self) -> str:
        """The commit the running server's image was built from, as its label says."""
        container = self.compose(["ps", "--quiet", "server"])
        if not container.ok or not container.out.strip():
            raise Refusal("no server container is running, so nothing says which commit the "
                          "stack runs; pass --from with the commit it was built from")
        image = self.host.run(["docker", "inspect", "--format", "{{.Image}}",
                               container.out.split()[0]])
        label = self.host.run(["docker", "image", "inspect", "--format",
                               f'{{{{index .Config.Labels "{stack.REVISION}"}}}}',
                               image.out.strip()]) if image.ok else image
        commit = label.out.strip() if label.ok else ""
        if commit in ("", "<no value>"):
            raise Refusal("the running server image does not say which commit it was built "
                          "from (it predates deploy/update), so pass --from with that commit")
        return commit

    def pull(self) -> str:
        self.step(1, "pull main")
        if self.opts.dry_run:
            fetched = self.git("fetch", "origin", "main")
            if not fetched.ok:
                raise Refusal(f"`git fetch origin main` failed:\n{fetched.err.strip()}")
            after = self.git("rev-parse", "origin/main").out.strip()
            head = self.git("rev-parse", "HEAD").out.strip()
            if not self.git("merge-base", "--is-ancestor", head, after).ok:
                raise Refusal("this checkout's main has commits origin/main does not, so "
                              "`git pull --ff-only` would refuse; put it back on origin/main")
        else:
            pulled = self.git("pull", "--ff-only", "origin", "main")
            if not pulled.ok:
                raise Refusal("`git pull --ff-only origin main` failed, so nothing was changed:\n"
                              + (pulled.err or pulled.out).strip())
            after = self.git("rev-parse", "HEAD").out.strip()
        return after

    def changes(self) -> None:
        """The commits this update deploys, listed under the pull."""
        log = self.git("log", "--oneline", f"{self.before}..{self.after}")
        if not log.ok:
            raise Refusal(f"`git log {self.before[:12]}..{self.after[:12]}` failed, so what "
                          f"this update deploys is unknown:\n{log.err.strip()}")
        self.say(log.out.strip() or f"already at {self.after[:12]}: rebuilding what is there")

    def build_jobs(self) -> str:
        """`CARGO_BUILD_JOBS` from the shell, else half the cores: users' renders share them."""
        return self.host.env("CARGO_BUILD_JOBS") or str(max(1, self.host.cores() // 2))

    def tag(self) -> str:
        """The tag the new images are built under until the restart: the commit's."""
        return self.after[:12]

    def build(self) -> None:
        self.step(2, "build, while the running stack keeps serving")
        env = {"CARGO_BUILD_JOBS": self.build_jobs(), "SCORSESE_REVISION": self.after,
               stack.TAG: self.tag()}
        args = self.compose_args(["build"])
        if self.opts.dry_run:
            self.say("would run: " + " ".join(f"{k}={v}" for k, v in env.items()) + " "
                     + " ".join(args))
            return
        self.stage = "building"
        if self.host.stream(args, cwd=self.deploy, env=env) != 0:
            raise Refusal("the build failed (above), before anything was stopped or dumped")
        self.stage = "built"
        self.say(f"built as `:{self.tag()}`; the running stack and its `latest` images are "
                 "untouched until the restart")

    def drain_and_dump(self) -> dict:
        """Wait out running work and take the dump, again if work started during it."""
        deadline = self.host.now() + self.opts.wait_timeout
        while True:
            self.drain(deadline)
            applied = stack.applied_migrations(self.compose)
            self.dump()
            busy = "" if self.opts.dry_run else self.busy()
            if not busy:
                return applied
            if not self.opts.wait:
                raise Refusal("work started while the dump was taken, and a restart would cut "
                              "it off; run again once it finishes, or with --wait to wait for "
                              f"it:\n{busy}")
            self.say(f"work started while the dump was taken, so waiting again and taking a "
                     f"fresh dump:\n{busy}")

    def busy(self) -> str:
        """The running work, one indented line each, or nothing."""
        return "\n".join(f"  {line}" for line in stack.running_work(self.compose))

    def drain(self, deadline: float) -> None:
        self.step(3, "no render, job or assistant turn running")
        while True:
            busy = self.busy()
            if not busy:
                self.say("nothing running")
                return
            if self.opts.dry_run:
                self.say(f"running now, so a real run would "
                         f"{'wait' if self.opts.wait else 'refuse'}:\n{busy}")
                return
            if not self.opts.wait:
                raise Refusal("work is running, and a restart would cut it off; run again once "
                              f"it finishes, or with --wait to wait for it:\n{busy}")
            if self.host.now() >= deadline:
                raise Refusal(f"work was still running after waiting {self.opts.wait_timeout}s "
                              f"(--wait-timeout), so the update stops:\n{busy}")
            self.say(f"waiting {self.POLL}s for:\n{busy}")
            self.host.sleep(self.POLL)

    def dump(self) -> None:
        self.step(4, "a dump of the database, the way back")
        args = ["exec", "-T", "backup", "backup"]
        if self.opts.dry_run:
            self.say("would run: " + " ".join(self.compose_args(args)))
            return
        done = self.compose(args, timeout=self.DUMP_TIMEOUT)
        self.say(done.out.rstrip())
        name = stack.dump_name(done.out)
        if name:
            self.dump_path = self.on_host(name)
        why = (done.err or done.out).strip()
        if done.timed_out:
            raise Refusal(f"the backup had not finished after {self.DUMP_TIMEOUT // 60} minutes, "
                          "so the update stops; it may still be running in the `backup` "
                          f"container (`docker compose top backup`):\n{why}")
        if not name:
            raise Refusal(f"the backup failed before the dump was written, so there would be "
                          f"no way back:\n{why}")
        if not done.ok:
            raise Refusal(f"the dump was written to {self.dump_path}, but the backup script "
                          "failed after it (its output is above), so the update stops until "
                          f"the backup works (docs/web.md, Backups):\n{why}")
        self.say(f"dump: {self.dump_path}; with {self.before[:12]}, the way back")

    def on_host(self, path: str) -> str:
        """A path inside the backup container (`/data/...`) as the host sees it."""
        data = self.env.get("SCORSESE_DATA", "")
        return data.rstrip("/") + path[len("/data"):] if data and path.startswith("/data/") else path

    def start(self) -> str:
        self.step(5, "restart on the new build")
        args = self.compose_args(["up", "--detach"])
        started = datetime.datetime.fromtimestamp(self.host.now(), datetime.timezone.utc)
        if self.opts.dry_run:
            self.say(f"would move `latest` to each `:{self.tag()}` image")
            self.respool()
            self.say("would run: " + " ".join(args))
        else:
            self.retag()
            self.respool()
            self.stage = "restarted"
            if self.host.stream(args, cwd=self.deploy) != 0:
                self.server_logs()
                raise Refusal("starting the new build failed; Compose's output and the "
                              "server's last lines are above")
        return started.strftime("%Y-%m-%dT%H:%M:%SZ")

    def retag(self) -> None:
        """Move `latest` to the new images, and drop the commit's tag beside it."""
        self.stage = "retagging"
        for image in stack.BUILT:
            new = f"{image}:{self.tag()}"
            for args in (["docker", "tag", new, f"{image}:latest"], ["docker", "image", "rm", new]):
                done = self.host.run(args)
                if not done.ok:
                    raise Refusal(f"`{' '.join(args)}` failed, so the stack was not restarted:\n"
                                  + (done.err or done.out).strip())

    def server_logs(self) -> None:
        self.say(self.compose(["logs", "server", "--tail", "50", "--no-color"]).out)

    def healthy(self) -> None:
        self.step(6, "every service healthy")
        deadline = self.host.now() + self.opts.timeout
        names = self.compose(["config", "--services"])
        if not names.ok:
            raise Refusal(f"`docker compose config --services` failed:\n{names.err.strip()}")
        while True:
            ps = self.compose(["ps", "--all", "--format", "json"])
            declared = names.out.split()
            entries = stack.declared(stack.services(ps.out), declared) if ps.ok else []
            waiting = stack.not_ready(entries, declared) if entries else ["no service is up"]
            if not waiting:
                self.say("healthy: " + ", ".join(sorted(e.get("Service", "?") for e in entries)))
                return
            if self.opts.dry_run:
                self.say("not healthy now: " + ", ".join(waiting))
                return
            if self.host.now() >= deadline:
                self.server_logs()
                raise Refusal(f"not healthy after {self.opts.timeout}s: {', '.join(waiting)}; "
                              "the server's last lines are above, and Cloudflare was not purged")
            self.host.sleep(self.HEALTH_POLL)


def interrupt(signum: int, frame: object) -> None:
    """SIGTERM and SIGHUP stop a run the way Ctrl-C does, so it still says where things stand."""
    raise KeyboardInterrupt


def main(argv: Sequence[str], host: Optional[Host] = None, deploy: Path = DEPLOY) -> int:
    opts = parse(argv)
    caught = (signal.SIGTERM, signal.SIGHUP)
    previous = [signal.signal(signum, interrupt) for signum in caught]
    try:
        Update(opts, host or Host(), deploy).run()
    except Refusal as refusal:
        print(f"\nupdate: {refusal}", file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        print("\nupdate: interrupted before the build, so the images and the running stack "
              "are as they were (the checkout may already be pulled)", file=sys.stderr)
        return 1
    finally:
        for signum, handler in zip(caught, previous):
            signal.signal(signum, handler)
    return 0
