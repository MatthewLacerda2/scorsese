"""What `deploy/update`'s tests share: a scripted host and a scratch deploy folder.

The host answers the commands the steps send and records them, so a test can
also say what was *not* run.
"""

from __future__ import annotations

import contextlib
import io
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "deploy"))

from updating import cli, stack  # noqa: E402,F401
from updating.host import Host, Result  # noqa: E402

ENV = "SCORSESE_WEB_BIND=0.0.0.0\nSCORSESE_WEB_PORT=8088\nSCORSESE_DATA=/srv/scor\n" \
      "SCORSESE_PUBLIC_ORIGIN='https://example.org'\nCLOUDFLARE_API_TOKEN=t\n"
PS = '{"Service":"server","State":"running","Health":"healthy"}\n' \
     '{"Service":"tunnel","State":"running","Health":""}\n'


class Scripted(Host):
    """Answers by the first rule whose snippets all appear in the command."""

    def __init__(self, rules, real_git=False, files=None, fail=()):
        self.rules, self.real_git, self.ran = rules, real_git, []
        self.files, self.fail = dict(files or {}), fail

    def run(self, args, cwd=None):
        self.ran.append(" ".join(args))
        if self.real_git and args[0] == "git":
            return super().run(args, cwd)
        line = " ".join(args)
        for words, result in self.rules:
            if all(word in line for word in words):
                # A list answers in turn, its last answer for good.
                return result.pop(0) if isinstance(result, list) and len(result) > 1 else \
                    result[0] if isinstance(result, list) else result
        return Result(0)

    def stream(self, args, cwd=None, env=None):
        """Streamed commands succeed, except those holding a snippet in `fail`."""
        line = " ".join(args)
        self.ran.append(line)
        return 1 if any(word in line for word in self.fail) else 0

    def read(self, path):
        return self.files.get(str(path))

    def write(self, path, text):
        self.files[str(path)] = text

    def get(self, url):
        return "200"

    def env(self, name):
        return None

    def cores(self):
        return 8

    def sleep(self, seconds):
        self.ran.append(f"sleep {seconds}")


def scratch(env=ENV):
    """A deploy folder in a fresh directory, with its settings unless `env` is None."""
    deploy = Path(tempfile.mkdtemp()) / "deploy"
    deploy.mkdir()
    if env is not None:
        (deploy / ".env").write_text(env)
    return deploy


def update(host, *argv, env=ENV, deploy=None):
    """Runs the script: exit code, stdout, stderr."""
    deploy = deploy or scratch(env)
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        code = cli.main(list(argv), host, deploy)
    return code, out.getvalue(), err.getvalue()


GIT = [(["--abbrev-ref"], Result(0, "main\n")), (["origin/main"], Result(0, "bbbb\n")),
       (["rev-parse", "HEAD"], Result(0, "aaaa\n")), (["--oneline"], Result(0, "bbbb A fix\n")),
       (["--count"], Result(0, "1\n")),
       (["ls-tree"], Result(0, "0001_accounts.sql\n0043_new_thing.sql\nREADME.md\n"))]
APPLIED = (["_sqlx_migrations"], Result(0, "1\taccounts\n"))
IDLE = (["chat_turns"], Result(0, ""))
RECORD = "/srv/scor/deployed-commit"
DUMPED = Result(0, "backup: database dumped to /data/backups/db/s-1.dump\n")
