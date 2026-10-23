"""`deploy/update`'s steps (#1509), split from the launcher so they can be tested.

- `host` is everything that touches the machine: commands, HTTP, files, the clock.
  The tests swap it for a scripted one, which is how the refusals and the dry
  run are proven without Docker (`.github/scripts/tests/deploying/`).
- `stack` asks the running stack questions and reads its answers: what work is
  running, which services are healthy, which migrations are applied.
- `cli` is the steps in order, and the flags.

Python 3.9 or later, stdlib only: it runs on the machine that hosts the web
app, not in a gate's container.
"""
