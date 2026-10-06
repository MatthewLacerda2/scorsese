#!/usr/bin/env python3
"""The human checks a batch left behind come back as one list, and only those.

`human-checks.py` reads merged pull request descriptions (#821). The fixture is
four real ones, recorded from #727, #749, #787 and #796, which between them use
four of the heading spellings; the hand-written bodies cover what the recorded
ones happen not to: a ticked item, a checklist under some other heading, a
heading in another case and level, and an example inside a code fence. Nothing
here talks to `gh`.
"""

from __future__ import annotations

import importlib.util
import json
import unittest
from datetime import datetime, timezone
from pathlib import Path
from unittest import mock

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("human_checks", HERE.parent / "human-checks.py")
checks = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(checks)

RECORDED = json.loads((HERE / "fixtures" / "human-checks-recorded.json").read_text())

MIXED = """## Test plan
- [ ] not a human check: a plan under an unrelated heading

### human CHECKS (taste — never a merge hold)
- [x] done already
- [ ] Listen to the cue on real speakers;
  it should not clip.
  - [ ] a nested check
- [ ] Look at the card in dark mode.

```markdown
- [ ] an example inside a fence
```

#### A sub-heading still inside the section
- [ ] under the sub-heading

## Gates
- [ ] a gate list is not a human check
"""


def pull(number: int, body: str, merged: str | None = "2026-10-06T00:00:00Z", updated=None):
    return {
        "number": number,
        "title": f"PR {number}",
        "html_url": f"https://github.com/o/r/pull/{number}",
        "body": body,
        "merged_at": merged,
        "updated_at": updated or merged or "2026-10-06T00:00:00Z",
    }


class Sections(unittest.TestCase):
    def test_every_recorded_spelling_is_read_and_nothing_else(self):
        found = {p["number"]: checks.human_checks(p["body"]) for p in RECORDED}
        # Each recorded body's only unticked items are its human checks.
        for p in RECORDED:
            with self.subTest(pr=p["number"]):
                self.assertEqual(len(found[p["number"]]), p["body"].count("- [ ]"))
        self.assertTrue(found[787][0].startswith("**Delete the throwaway branch `772-measure`**"))
        self.assertTrue(found[749][2].startswith("Watch whether Gemini 3.8 Flash"))

    def test_ticked_unrelated_and_fenced_items_are_left_out(self):
        self.assertEqual(
            checks.human_checks(MIXED),
            [
                "Listen to the cue on real speakers; it should not clip.",
                "a nested check",
                "Look at the card in dark mode.",
                "under the sub-heading",
            ],
        )

    def test_a_body_without_a_section_or_at_all_has_none(self):
        self.assertEqual(checks.human_checks("## Test plan\n- [ ] run it\n"), [])
        self.assertEqual(checks.human_checks(None), [])
        self.assertEqual(checks.human_checks("#594 is an issue, not a heading\n- [ ] x"), [])


class Fetching(unittest.TestCase):
    SINCE = datetime(2026, 10, 5, 12, tzinfo=timezone.utc)

    def test_only_merges_since_count_and_paging_stops_at_older_updates(self):
        full = [pull(n, "") for n in range(200, 200 + checks.PAGE)]
        second = [
            pull(3, "", merged=None, updated="2026-10-06T00:00:00Z"),  # closed, not merged
            pull(2, "", merged="2026-10-04T00:00:00Z", updated="2026-10-05T13:00:00Z"),
            pull(1, "", merged="2026-10-01T00:00:00Z"),
        ]
        asked = []

        def gh(*args):
            asked.append(args[1])
            return [full, second, []][len(asked) - 1]

        merged = checks.merged_since(self.SINCE, gh)
        self.assertEqual([p["number"] for p in merged], list(range(200, 200 + checks.PAGE)))
        self.assertEqual(len(asked), 2)
        self.assertIn("state=closed", asked[0])
        self.assertIn("page=2", asked[1])

    def test_a_bare_date_is_midnight_utc(self):
        self.assertEqual(checks.moment("2026-10-05"), datetime(2026, 10, 5, tzinfo=timezone.utc))
        self.assertEqual(
            checks.moment("2026-10-05T15:00:00-03:00"),
            datetime(2026, 10, 5, 18, tzinfo=timezone.utc),
        )


class Report(unittest.TestCase):
    def test_a_heading_per_pull_request_with_checks_and_a_total(self):
        text = checks.report([pull(5, MIXED), pull(6, "nothing here")], "2026-10-05")
        self.assertIn("### [#5](https://github.com/o/r/pull/5): PR 5", text)
        self.assertNotIn("#6", text)
        self.assertIn("- [ ] Look at the card in dark mode.", text)
        self.assertTrue(text.endswith(
            "4 unticked human check(s) in 1 of the 2 pull request(s) merged since 2026-10-05."
        ))

    def test_nothing_found_is_one_line(self):
        text = checks.report([pull(6, "")], "2026-10-05")
        self.assertEqual(text.count("\n"), 0)
        self.assertIn("No unticked human checks", text)

    def test_an_outage_is_unreachable_never_an_empty_list(self):
        outage = checks.mergeable.Unreachable("network: GitHub unreachable")
        with mock.patch.object(checks, "merged_since", side_effect=outage), \
            mock.patch("builtins.print") as printed:
            status = checks.main(["--since", "2026-10-05"])
        self.assertEqual(status, checks.mergeable.UNREACHABLE_STATUS)
        self.assertIn("network", printed.call_args.args[0])


if __name__ == "__main__":
    unittest.main()
