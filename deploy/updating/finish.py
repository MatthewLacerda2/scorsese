"""`deploy/update`'s last steps, once the new build is up: purge Cloudflare's
landing page, check the addresses answer, and say what changed.

`Finish` is a part of `cli.Update`, split off by concern; it uses the run's
host, settings and what the run learnt (`before`, `after`, the dump).
"""

from __future__ import annotations

from typing import List

from . import stack
from .host import Refusal

MIGRATIONS = "crates/server/migrations"


class Finish:
    """Steps 7 to 9 of `cli.Update`."""

    def purge(self) -> None:
        self.step(7, "drop Cloudflare's copy of the landing page")
        missing = [name for name in ("CLOUDFLARE_API_TOKEN", "SCORSESE_PUBLIC_ORIGIN")
                   if not self.env.get(name)]
        if missing:
            self.say(f"no {' or '.join(missing)} in the settings, so no Cloudflare cache to purge")
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
        count = self.git("rev-list", "--count", f"{before}..{after}")
        if not count.ok:
            raise Refusal(f"`git rev-list {before[:12]}..{after[:12]}` failed:\n"
                          + count.err.strip())
        self.say(f"commits: {before[:12]}..{after[:12]} ({count.out.strip()})")
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

