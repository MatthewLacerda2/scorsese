"""`deploy/update`'s answer to Compose's one question: the spool volume, again.

`Spool` is a part of `cli.Update`, split off by concern; it uses the run's
host, options and `compose`.
"""

from __future__ import annotations

from . import stack
from .host import Refusal


class Spool:
    """The `capture-data` volume, made again in step 5 when it points elsewhere."""

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
        """Remove capture-launcher, the captures it left, and the volume `name`, so `up`
        makes the volume and the launcher again.

        The volume holds nothing of its own. The launcher mounts it, and so does each
        capture container it starts (a media file each), which outlives the launcher's
        removal: `docker run --rm` is the daemon's, not the client's.
        """
        if not name:
            return
        if not self.opts.dry_run:
            self.stage = "respooling"
        self.respool_step(self.compose_args(["rm", "--stop", "--force", "capture-launcher"]))
        listed = ["docker", "ps", "--quiet", "--all", f"--filter=label={stack.CAPTURE_LABEL}"]
        if self.opts.dry_run:
            self.say(f"would run: docker rm --force <every container labelled {stack.CAPTURE_LABEL}>")
        else:
            left = self.host.run(listed).out.split()
            if left:
                self.respool_step(["docker", "rm", "--force", *left])
        self.respool_step(["docker", "volume", "rm", name])

    def respool_step(self, args) -> None:
        """Run `args`, or say it would; a failure stops the update before `up`."""
        if self.opts.dry_run:
            self.say("would run: " + " ".join(args))
            return
        done = self.host.run(args, cwd=self.deploy)
        if not done.ok:
            raise Refusal(f"`{' '.join(args)}` failed, so the stack was not restarted:\n"
                          + (done.err or done.out).strip())
