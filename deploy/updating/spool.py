"""`deploy/update`'s answer to Compose's one question: the spool volume, again.

`Spool` is a part of `cli.Update`, split off by concern; it uses the run's
host, options and `compose`.
"""

from __future__ import annotations

from . import stack
from .host import Refusal


class Spool:
    """The `capture-spool` volume, made again in step 5 when it points elsewhere."""

    def moved_spool(self) -> str:
        """The spool volume's name when SCORSESE_DATA has moved since it was made, else "".

        Read-only, so it runs before `latest` moves and a refusal here leaves
        everything as it was. `up` would ask first, and run without a terminal it
        keeps the old folder, so every page capture times out into a slug card
        (docs/web.md, Updating).
        """
        config = self.compose(["config", "--format", "json"])
        if not config.ok:
            raise Refusal(f"`docker compose config` failed:\n{config.err.strip()}")
        name, device = stack.spool(config.out)
        made = self.host.run(["docker", "volume", "inspect", "--format", "{{json .Options}}", name])
        was = stack.volume_device(made.out) if made.ok else ""
        if not made.ok or was == device:
            return ""
        self.say(f"SCORSESE_DATA has moved: the volume {name} points at {was or 'nothing'}, "
                 f"not {device}, so it is made again (it holds nothing of its own)")
        return name

    def respool(self, name: str) -> None:
        """Remove capture-launcher and the volume `name`, so `up` makes both again.

        The volume holds nothing of its own; only capture-launcher mounts it.
        """
        if not name:
            return
        if not self.opts.dry_run:
            self.stage = "respooling"
        steps = (self.compose_args(["rm", "--stop", "--force", "capture-launcher"]),
                 ["docker", "volume", "rm", name])
        for args in steps:
            if self.opts.dry_run:
                self.say("would run: " + " ".join(args))
                continue
            done = self.host.run(args, cwd=self.deploy)
            if not done.ok:
                raise Refusal(f"`{' '.join(args)}` failed, so the stack was not restarted:\n"
                              + (done.err or done.out).strip())
