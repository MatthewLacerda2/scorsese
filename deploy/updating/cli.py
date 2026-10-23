"""`deploy/update`: the steps of docs/web.md's *Updating*, in order.

The build comes first and the restart last: `docker compose build` takes the
whole cargo build while the old stack keeps serving, so the running-work check
and the dump are taken just before `up` recreates anything, not twenty minutes
earlier. A step that fails stops the run with a sentence saying what happened
and what is running, and nothing after it runs. The server migrates before it
serves, so this never runs migrations; it waits for the health that proves
they ran.
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
# Under SCORSESE_DATA: the commit the stack was last updated to, so a run after
# a refused one still knows what is running, whatever the checkout says.
DEPLOYED = "deployed-commit"


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
    parser.add_argument("--timeout", type=int, default=600, metavar="SECONDS",
                        help="how long the services have to become healthy after the "
                        "restart (default 600)")
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
        self.before = self.after = self.dump_path = ""
        self.restarted = False

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
        self.before = self.deployed()
        self.say(f"the stack runs {self.before[:12]}: if this update goes wrong, "
                 "that is the commit to go back to")
        self.after = self.pull()
        try:
            self.build()
            self.drain()
            applied = stack.applied_migrations(self.compose)
            self.dump()
            started = self.start()
            self.healthy()
            self.purge()
            self.check()
        except Refusal as refusal:
            raise Refusal(f"{refusal}\n{self.where()}") from None
        self.summary(applied, started)

    def where(self) -> str:
        """What is running after a failure past the pull, and the way back once there is one."""
        if self.opts.dry_run:
            return "(dry run: nothing was pulled, built or restarted)"
        dump = f"; the dump {self.dump_path} was taken" if self.dump_path else ""
        if not self.restarted:
            return (f"the checkout is at {self.after[:12]}, while the stack still runs "
                    f"{self.before[:12]}, untouched{dump}")
        return (f"the stack was restarted on {self.after[:12]}; the way back: check out "
                f"{self.before[:12]}, rebuild, and restore {self.dump_path} (docs/web.md, Restoring)")

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

    def record(self) -> Optional[Path]:
        data = self.env.get("SCORSESE_DATA", "")
        return Path(data) / DEPLOYED if data else None

    def deployed(self) -> str:
        """The commit the stack runs: the last update's record, else the checkout's HEAD."""
        record = self.record()
        written = (self.host.read(record) or "").strip() if record else ""
        if written:
            return written
        head = self.git("rev-parse", "HEAD").out.strip()
        self.say(f"no record of the deployed commit at {record or '$SCORSESE_DATA/' + DEPLOYED}, "
                 f"so taking the checkout's HEAD, {head[:12]}, as what the stack runs")
        return head

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
        log = self.git("log", "--oneline", f"{self.before}..{after}").out.strip()
        self.say(log or f"already at {after[:12]}: rebuilding what is there")
        return after

    def build_jobs(self) -> str:
        """`CARGO_BUILD_JOBS` from the shell, else half the cores: users' renders share them."""
        return self.host.env("CARGO_BUILD_JOBS") or str(max(1, self.host.cores() // 2))

    def build(self) -> None:
        self.step(2, "build, while the running stack keeps serving")
        jobs = self.build_jobs()
        args = self.compose_args(["build"])
        if self.opts.dry_run:
            self.say(f"would run: CARGO_BUILD_JOBS={jobs} " + " ".join(args))
        elif self.host.stream(args, cwd=self.deploy, env={"CARGO_BUILD_JOBS": jobs}) != 0:
            raise Refusal("the build failed (above), before anything was stopped or dumped")

    def drain(self) -> None:
        self.step(3, "no render, job or assistant turn running")
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

    def dump(self) -> None:
        self.step(4, "a dump of the database, the way back")
        args = ["exec", "-T", "backup", "backup"]
        if self.opts.dry_run:
            self.say("would run: " + " ".join(self.compose_args(args)))
            return
        done = self.compose(args)
        self.say(done.out.rstrip())
        name = stack.dump_name(done.out)
        if name:
            self.dump_path = self.on_host(name)
        why = (done.err or done.out).strip()
        if not name:
            raise Refusal(f"the backup failed before the dump was written, so there would be "
                          f"no way back:\n{why}")
        if not done.ok:
            raise Refusal(f"the dump was written to {self.dump_path}, but copying it off the "
                          "machine failed, so the update stops until the backup works "
                          f"(docs/web.md, Backups):\n{why}")
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
            self.say("would run: " + " ".join(args))
        else:
            self.restarted = True
            if self.host.stream(args, cwd=self.deploy) != 0:
                raise Refusal("starting the new build failed (above)")
        return started.strftime("%Y-%m-%dT%H:%M:%SZ")

    def healthy(self) -> None:
        self.step(6, "every service healthy")
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
        self.step(7, "drop Cloudflare's copy of the landing page")
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
        self.step(8, "the local and public addresses answer")
        wrong = []
        for url in self.addresses():
            answer = self.host.get(url)
            self.say(f"{answer}  {url}")
            if answer != "200":
                wrong.append(url)
        if wrong and not self.opts.dry_run:
            raise Refusal("updated, but these do not answer 200: " + ", ".join(wrong))

    def summary(self, applied: dict, started: str) -> None:
        self.step(9, "what changed")
        before, after = self.before, self.after
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
        if self.opts.dry_run:
            return
        log = self.compose(["logs", "server", "--since", started, "--no-color"]).out
        projects, templates = stack.stored_migrations(log)
        self.say(f"stored projects migrated: {projects}, templates: {templates}")
        self.say(f"the way back: check out {before[:12]}, rebuild, and restore "
                 f"{self.dump_path} (docs/web.md, Restoring)")
        record = self.record()
        failed = self.host.write(record, after + "\n") if record else "SCORSESE_DATA is not set"
        if failed:
            self.say(f"could not record {after[:12]} as deployed ({failed}); the next run "
                     "will take the checkout's HEAD as what is running")


def main(argv: Sequence[str], host: Optional[Host] = None, deploy: Path = DEPLOY) -> int:
    opts = parse(argv)
    try:
        Update(opts, host or Host(), deploy).run()
    except Refusal as refusal:
        print(f"\nupdate: {refusal}", file=sys.stderr)
        return 1
    return 0
