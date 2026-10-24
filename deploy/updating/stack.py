"""Questions put to the running stack, and how their answers are read.

Postgres is asked through its own container (`psql` as the `scorsese` role,
which owns every table and so reads past row-level security), so the host
needs no client and no password.
"""

from __future__ import annotations

import json
import re
from typing import Callable, Dict, List, Sequence, Tuple

from .host import Refusal, Result

# Work an update would cut off. A job `waiting` is counted with one `running`:
# the server would pick it up again after the restart, but it is somebody's
# render or shot mid-queue, and a quiet moment is the time to update. An
# assistant turn `asking` waits on its user, not on the server, so it is not.
RUNNING_WORK = (
    "SELECT 'job', kind, state, count(*) FROM jobs"
    " WHERE state IN ('waiting', 'running') GROUP BY kind, state"
    " UNION ALL "
    "SELECT 'turn', 'assistant', state, count(*) FROM chat_turns"
    " WHERE state = 'running' GROUP BY state"
    " ORDER BY 1, 2, 3"
)

# What sqlx has applied (crates/server/src/db).
APPLIED_MIGRATIONS = (
    "SELECT version, description FROM _sqlx_migrations WHERE success ORDER BY version"
)

# The label the server image carries its commit in (deploy/server.Dockerfile).
REVISION = "org.opencontainers.image.revision"

Compose = Callable[[Sequence[str]], Result]


def psql(compose: Compose, sql: str) -> List[List[str]]:
    """Rows of a query, each a list of its columns as text."""
    done = compose(
        ["exec", "-T", "postgres", "psql", "-U", "scorsese", "-d", "scorsese",
         "-v", "ON_ERROR_STOP=1", "-At", "-F", "\t", "-c", sql]
    )
    if not done.ok:
        raise Refusal(
            "could not ask Postgres (through the `postgres` container) what it holds;"
            f" is the stack up?\n{(done.err or done.out).strip()}"
        )
    return [line.split("\t") for line in done.out.splitlines() if line.strip()]


def running_work(compose: Compose) -> List[str]:
    """What is running, one line each: `2 render jobs running`."""
    lines = []
    for what, kind, state, count in psql(compose, RUNNING_WORK):
        noun = "assistant turns" if what == "turn" else f"{kind} jobs"
        if count == "1":
            noun = noun[:-1]
        lines.append(f"{count} {noun} {state}")
    return lines


def applied_migrations(compose: Compose) -> Dict[str, str]:
    """Every SQL migration applied, version to description."""
    return {row[0]: row[1] for row in psql(compose, APPLIED_MIGRATIONS) if len(row) == 2}


def migration_files(names: Sequence[str]) -> Dict[str, str]:
    """`crates/server/migrations` file names as version to description.

    sqlx reads `0012_some_thing.sql` as version 12, `some thing`.
    """
    found = {}
    for name in names:
        match = re.fullmatch(r"(?:.*/)?(\d+)_(.+)\.sql", name)
        if match:
            found[str(int(match.group(1)))] = match.group(2).replace("_", " ")
    return found


def services(ps_json: str) -> List[dict]:
    """`docker compose ps --format json`, as either shape Compose prints.

    Compose 2.21 and later print one object per line; older ones, an array.
    """
    text = ps_json.strip()
    if not text:
        return []
    if text.startswith("["):
        return json.loads(text)
    return [json.loads(line) for line in text.splitlines() if line.strip()]


def declared(entries: Sequence[dict], names: Sequence[str]) -> List[dict]:
    """The entries of services the compose file declares, without orphans.

    `ps --all` lists a container left from a service since removed; filtering by
    `config --services` needs no flag a Compose version might lack.
    """
    return [entry for entry in entries if entry.get("Service") in names]


def not_ready(entries: Sequence[dict]) -> List[str]:
    """Each service not yet up, with what it is instead: `server (starting)`.

    Up is running and, where it has a healthcheck, healthy; a tunnel has none.
    """
    waiting = []
    for entry in entries:
        name = entry.get("Service") or entry.get("Name") or "?"
        state = (entry.get("State") or "").lower()
        health = (entry.get("Health") or "").lower()
        if state != "running":
            waiting.append(f"{name} ({state or 'unknown'})")
        elif health not in ("", "healthy"):
            waiting.append(f"{name} ({health})")
    return sorted(waiting)


def stored_migrations(log: str) -> Tuple[int, int]:
    """Projects and templates the server migrated, from its start-up lines.

    It prints `migrated N stored projects` (and templates) only when N > 0
    (`crates/server/src/lib.rs`), so silence is zero.
    """
    counts = {"projects": 0, "templates": 0}
    for count, what in re.findall(r"migrated (\d+) stored (projects|templates)", log):
        counts[what] += int(count)
    return counts["projects"], counts["templates"]


def dump_name(backup_out: str) -> str:
    """The dump `backup` wrote, from its own line saying so."""
    match = re.search(r"database dumped to (\S+)", backup_out)
    return match.group(1) if match else ""
