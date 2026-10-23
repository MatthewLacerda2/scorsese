#!/usr/bin/env python3
"""How `deploy/update` reads Compose, the server's log, `backup` and its flags.
"""

from __future__ import annotations

import contextlib
import io
import unittest

from .scripted import PS, cli, stack


class Reading(unittest.TestCase):
    def test_compose_ps_in_either_shape(self):
        array = '[{"Service":"web","State":"running","Health":"starting"}]'
        self.assertEqual(stack.not_ready(stack.services(array)), ["web (starting)"])
        self.assertEqual(stack.not_ready(stack.services(PS)), [])
        exited = '{"Service":"server","State":"exited","Health":""}'
        self.assertEqual(stack.not_ready(stack.services(exited)), ["server (exited)"])

    def test_start_up_lines_and_the_dump(self):
        log = "server-1 | scorsese-server: migrated 3 stored projects to this build's format\n" \
              "server-1 | scorsese-server: migrated 1 stored templates to this build's format\n"
        self.assertEqual(stack.stored_migrations(log), (3, 1))
        self.assertEqual(stack.stored_migrations(""), (0, 0))
        out = "backup: database dumped to /data/backups/db/scorsese-20261023T1200Z.dump\n"
        self.assertEqual(stack.dump_name(out), "/data/backups/db/scorsese-20261023T1200Z.dump")

    def test_flags(self):
        opts = cli.parse(["--wait", "--timeout", "30"])
        self.assertEqual((opts.wait, opts.dry_run, opts.timeout), (True, False, 30))
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            cli.parse(["--timeout", "soon"])


if __name__ == "__main__":
    unittest.main()
