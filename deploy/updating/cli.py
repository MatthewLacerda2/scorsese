"""`deploy/update`: the eight steps of docs/web.md's *Updating*, in order.

Every precondition is checked before anything changes, and a step that fails
stops the run with a sentence saying what happened and what is safe: nothing
after a failed step runs. The server migrates before it serves, so this
never runs migrations; it waits for the health that proves they ran.
"""

from __future__ import annotations

import argparse
import datetime
import sys
from pathlib import Path
from typing import List, Optional, Sequence

from . import stack
from .host import Host, Refusal, Result, read_env

DEPLOY = Path(__file__).resolve().parent.parent
MIGRATIONS = "crates/server/migrations"


def parse(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="deploy/update",
        description="Update the web app from main: pull, dump, build, wait for health, "
        "purge Cloudflare's landing page, check, and say what changed (docs/web.md, Updating).",
    )
    parser.add_argument("--dry-run", action="store_true",
                        help="check everything and print the steps, changing nothing: "
                        "fetches instead of pulling, and builds, dumps and purges nothing")
    parser.add_argument("--wait", action="store_true",
                        help="while jobs or assistant turns are running, wait for them to "
                        "finish instead of refusing")
    parser.add_argument("--timeout", type=int, default=600, metavar="SECONDS",
                        help="how long the services have to become healthy after the "
                        "build (default 600)")
    parser.add_argument("--env-file", type=Path, metavar="PATH",
                        help="the stack's settings (default: DEPLOY_ENV, else deploy/.env)")
    return parser.parse_args(list(argv))


class Update:
    """One run: the options, the machine, and what it learns on the way."""

    POLL = 30  # seconds between looks at the running work, under --wait
    HEALTH_POLL = 5

    def __init__(self, opts: argparse.Namespace, host: Host, deploy: Path = DEPLOY):
        self.opts, self.host, self.deploy = opts, host, deploy
        self.root = deploy.parent
        self.env_file = (opts.env_file or Path(host.env("DEPLOY_ENV") or deploy / ".env")).resolve()

    def say(self, line: str = "") -> None:
        print(line, flush=True)

    def step(self, n: int, what: str) -> None:
        self.say(f"\n== {n}. {what}{' (dry run)' if self.opts.dry_run else ''}")

    def git(self, *args: str) -> Result:
        return self.host.run(["git", *args], cwd=self.root)

    def compose(self, args: Sequence[str]) -> Result:
        return self.host.run(self.compose_args(args), cwd=self.deploy)

    def compose_args(self, args: Sequence[str]) -> List[str]:
        return ["docker", "compose", "--env-file", str(self.env_file), *args]

    def run(self) -> None:
        self.preconditions()
        before, after = self.pull()
        self.drain()
        applied = stack.applied_migrations(self.compose)
        dump = self.dump()
        started = self.build()
        self.healthy()
        self.purge()
        self.check()
        self.summary(before, after, applied, dump, started)

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

    def pull(self) -> tuple:
        self.step(1, "pull main")
        before = self.git("rev-parse", "HEAD").out.strip()
        if self.opts.dry_run:
            fetched = self.git("fetch", "origin", "main")
            if not fetched.ok:
                raise Refusal(f"`git fetch origin main` failed:\n{fetched.err.strip()}")
            after = self.git("rev-parse", "origin/main").out.strip()
            if not self.git("merge-base", "--is-ancestor", before, after).ok:
                raise Refusal("this checkout's main has commits origin/main does not, so "
                              "`git pull --ff-only` would refuse; put it back on origin/main")
        else:
            pulled = self.git("pull", "--ff-only", "origin", "main")
            if not pulled.ok:
                raise Refusal("`git pull --ff-only origin main` failed, so nothing was changed:\n"
                              + (pulled.err or pulled.out).strip())
            after = self.git("rev-parse", "HEAD").out.strip()
        log = self.git("log", "--oneline", f"{before}..{after}").out.strip()
        self.say(log or f"already at {after[:12]}: rebuilding what is there")
        return before, after

    def drain(self) -> None:
        self.step(2, "no render, job or assistant turn running")
        while True:
            busy = stack.running_work(self.compose)
            if not busy:
                self.say("nothing running")
                return
            listing = "\n".join(f"  {line}" for line in busy)
            if self.opts.dry_run:
                self.say(f"running now, so a real run would "
                         f"{'wait' if self.opts.wait else 'refuse'}:\n{listing}")
                return
            if not self.opts.wait:
                raise Refusal("work is running, and a restart would cut it off; run again once "
                              f"it finishes, or with --wait to wait for it:\n{listing}")
            self.say(f"waiting {self.POLL}s for:\n{listing}")
            self.host.sleep(self.POLL)

    def dump(self) -> str:
        self.step(3, "a dump of the database, the way back")
        if self.opts.dry_run:
            self.say("would run: " + " ".join(self.compose_args(["exec", "-T", "backup", "backup"])))
            return ""
        done = self.compose(["exec", "-T", "backup", "backup"])
        self.say(done.out.rstrip())
        name = stack.dump_name(done.out)
        if not done.ok or not name:
            raise Refusal("the backup failed, so there would be no way back; nothing was "
                          f"changed:\n{(done.err or done.out).strip()}")
        name = self.on_host(name)
        self.say(f"dump: {name}")
        return name

    def on_host(self, path: str) -> str:
        """A path inside the backup container (`/data/...`) as the host sees it."""
        data = self.env.get("SCORSESE_DATA", "")
        return data.rstrip("/") + path[len("/data"):] if data and path.startswith("/data/") else path

    def build_jobs(self) -> str:
        """`CARGO_BUILD_JOBS` from the shell, else half the cores: users' renders share them."""
        return self.host.env("CARGO_BUILD_JOBS") or str(max(1, self.host.cores() // 2))

    def build(self) -> str:
        self.step(4, "build and start")
        jobs = self.build_jobs()
        args = self.compose_args(["up", "--detach", "--build"])
        started = datetime.datetime.fromtimestamp(self.host.now(), datetime.timezone.utc)
        if self.opts.dry_run:
            self.say(f"would run: CARGO_BUILD_JOBS={jobs} " + " ".join(args))
        elif self.host.stream(args, cwd=self.deploy, env={"CARGO_BUILD_JOBS": jobs}) != 0:
            raise Refusal("the build or start failed (above); containers whose build failed "
                          "were left as they were")
        return started.strftime("%Y-%m-%dT%H:%M:%SZ")

    def healthy(self) -> None:
        self.step(5, "every service healthy")
        deadline = self.host.now() + self.opts.timeout
        while True:
            ps = self.compose(["ps", "--all", "--format", "json"])
            entries = stack.services(ps.out) if ps.ok else []
            waiting = stack.not_ready(entries) if entries else ["no service is up"]
            if not waiting:
                self.say("healthy: " + ", ".join(sorted(e.get("Service", "?") for e in entries)))
                return
            if self.opts.dry_run:
                self.say("not healthy now: " + ", ".join(waiting))
                return
            if self.host.now() >= deadline:
                self.say(self.compose(["logs", "server", "--tail", "50", "--no-color"]).out)
                raise Refusal(f"not healthy after {self.opts.timeout}s: {', '.join(waiting)}; "
                              "the server's last lines are above, and Cloudflare was not purged")
            self.host.sleep(self.HEALTH_POLL)

    def purge(self) -> None:
        self.step(6, "drop Cloudflare's copy of the landing page")
        if not self.env.get("CLOUDFLARE_API_TOKEN"):
            self.say("no CLOUDFLARE_API_TOKEN in the settings, so no Cloudflare cache to purge")
            return
        script = str(self.deploy / "purge-landing")
        if self.opts.dry_run:
            self.say(f"would run: {script}")
        elif self.host.stream([script], cwd=self.deploy, env={"DEPLOY_ENV": str(self.env_file)}):
            raise Refusal("purging Cloudflare's landing page failed (above): until it expires, "
                          "within the hour, a first visit draws a blank page")

    def addresses(self) -> List[str]:
        bind = self.env.get("SCORSESE_WEB_BIND") or "127.0.0.1"
        bind = "127.0.0.1" if bind in ("0.0.0.0", "::") else bind
        bases = [f"http://{bind}:{self.env.get('SCORSESE_WEB_PORT') or '8088'}"]
        if self.env.get("SCORSESE_PUBLIC_ORIGIN"):
            bases.append(self.env["SCORSESE_PUBLIC_ORIGIN"].rstrip("/"))
        return [base + path for base in bases for path in ("/", "/api/health")]

    def check(self) -> None:
        self.step(7, "the local and public addresses answer")
        wrong = []
        for url in self.addresses():
            answer = self.host.get(url)
            self.say(f"{answer}  {url}")
            if answer != "200":
                wrong.append(url)
        if wrong and not self.opts.dry_run:
            raise Refusal("updated, but these do not answer 200: " + ", ".join(wrong))

    def summary(self, before: str, after: str, applied: dict, dump: str, started: str) -> None:
        self.step(8, "what changed")
        count = self.git("rev-list", "--count", f"{before}..{after}").out.strip() or "0"
        self.say(f"commits: {before[:12]}..{after[:12]} ({count})")
        if self.opts.dry_run:
            listed = self.git("ls-tree", "--name-only", f"{after}:{MIGRATIONS}").out.split()
            new = stack.migration_files(listed)
            label = "SQL migrations it would apply"
        else:
            new = stack.applied_migrations(self.compose)
            label = "SQL migrations applied"
        fresh = [f"{v} {new[v]}" for v in sorted(new, key=int) if v not in applied]
        self.say(f"{label}: {', '.join(fresh) if fresh else 'none'}")
        if not self.opts.dry_run:
            log = self.compose(["logs", "server", "--since", started, "--no-color"]).out
            projects, templates = stack.stored_migrations(log)
            self.say(f"stored projects migrated: {projects}, templates: {templates}")
            self.say(f"the way back: check out {before[:12]}, rebuild, and restore {dump} "
                     "(docs/web.md, Restoring)")


def main(argv: Sequence[str], host: Optional[Host] = None, deploy: Path = DEPLOY) -> int:
    opts = parse(argv)
    try:
        Update(opts, host or Host(), deploy).run()
    except Refusal as refusal:
        print(f"\nupdate: {refusal}", file=sys.stderr)
        return 1
    return 0
