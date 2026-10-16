"""A re-blessed reference is named with a reason, and a pin move re-blesses
every page (#1217).

`reblessed.py` is pure but for `evidence`, and `mergeable.judge` reads its
answer from `pull["reblessed"]`, so all of it is tested by import.
"""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[2] / "mergeable.py"
_spec = importlib.util.spec_from_file_location("mergeable", SCRIPT)
mergeable = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(mergeable)
reblessed = mergeable.reblessed

SHA = "c23b7c4f0000000000000000000000000000000a"
GREEN = [{"name": "CI", "head_sha": SHA, "id": 1, "status": "completed",
          "conclusion": "success", "html_url": "https://example.invalid/1"}]
JOBS = {1: [{"name": "fmt + clippy + test", "conclusion": "success"}]}
PAGES = ["page_alpha", "page_motion"]


def golden(name: str, status: str = "modified", frame: str = "frame-0000.png") -> dict:
    return {"filename": f"crates/golden/fixtures/{name}/expected/{frame}",
            "status": status, "changes": 0}


def pull(body: str, files: list[dict], pages: list[str] = ()) -> dict:
    return {"isDraft": False, "headRefOid": SHA, "number": 9, "headRefName": "main",
            "body": body, "reblessed": {"files": files, "pages": list(pages)}}


class Modified(unittest.TestCase):
    def test_a_modified_reference_without_a_reason_is_handed_back(self):
        ok, lines = mergeable.judge(pull("Gates: green", [golden("alpha")]), GREEN, JOBS)
        self.assertFalse(ok)
        self.assertIn("`alpha`", lines[0])

    def test_a_modified_reference_named_with_a_reason_passes(self):
        body = "## Re-blessed\n- `alpha`: the new dither moves each gradient.\n\nGates: ok"
        ok, lines = mergeable.judge(pull(body, [golden("alpha")]), GREEN, JOBS)
        self.assertTrue(ok, lines)

    def test_a_single_line_counts_and_a_snapshot_is_a_reference(self):
        snap = {"filename": "app/tests/snapshots/a_whole_edit.png", "status": "modified"}
        body = "**Re-blessed**: `a_whole_edit` — the bar is two pixels taller now."
        self.assertEqual(reblessed.refusal(pull(body, [snap])["reblessed"], body), [])

    def test_a_name_without_a_reason_is_not_enough(self):
        body = "## Re-blessed\n- `alpha`, `blend`\n"
        lines = reblessed.refusal({"files": [golden("alpha"), golden("blend")]}, body)
        self.assertIn("`alpha`, `blend`", lines[0])

    def test_a_name_outside_the_marked_lines_is_not_enough(self):
        body = "`alpha` changed because the new dither moves each gradient."
        self.assertTrue(reblessed.refusal({"files": [golden("alpha")]}, body))

    def test_a_longer_name_does_not_cover_its_prefix(self):
        body = "Re-blessed: `a_whole_edit_in_light` for the taller bar, on purpose."
        snap = {"filename": "app/tests/snapshots/a_whole_edit.png", "status": "modified"}
        self.assertIn("`a_whole_edit`", reblessed.refusal({"files": [snap]}, body)[0])

    def test_an_added_or_removed_reference_needs_nothing(self):
        files = [golden("dashes", "added"), golden("old", "removed"),
                 {"filename": "crates/golden/fixtures/alpha/expected/decoder.txt",
                  "status": "modified"}]
        self.assertEqual(reblessed.refusal({"files": files}, ""), [])

    def test_a_rename_counts_only_when_the_image_changed(self):
        moved = golden("beta", "renamed")
        self.assertEqual(reblessed.modified([moved]), [])
        self.assertEqual(reblessed.modified([moved | {"changes": 4}]), ["beta"])


class Pin(unittest.TestCase):
    PIN = {"filename": "tools/chromium/pin", "status": "modified"}

    def test_a_pin_move_with_every_page_rebless_named_once_passes(self):
        files = [self.PIN, golden("page_alpha"), golden("page_motion")]
        body = "## Re-blessed\n- `page_*`: the Chromium pin moved to 155.0.1.2.\n"
        self.assertEqual(reblessed.refusal({"files": files, "pages": PAGES}, body), [])

    def test_a_pin_move_missing_a_page_is_handed_back(self):
        files = [self.PIN, golden("page_alpha")]
        body = "Re-blessed: `page_*`, the Chromium pin moved to 155.0.1.2."
        lines = reblessed.refusal({"files": files, "pages": PAGES}, body)
        self.assertIn("leaves `page_motion`", lines[0])

    def test_a_page_added_under_the_new_pin_counts(self):
        files = [self.PIN, golden("page_alpha"), golden("page_motion", "added")]
        body = "Re-blessed: `page_alpha` because the Chromium pin moved."
        self.assertEqual(reblessed.refusal({"files": files, "pages": PAGES}, body), [])


class Evidence(unittest.TestCase):
    def test_it_pages_through_files_and_lists_pages_only_for_a_pin_move(self):
        asked = []

        def gh(*args):
            asked.append(args[1])
            if "contents" in args[1]:
                return [{"name": "page_alpha", "type": "dir"},
                        {"name": "alpha", "type": "dir"}]
            if args[1].endswith("&page=1"):
                return [golden(f"f{n}") for n in range(100)]
            return [{"filename": "tools/chromium/pin", "status": "modified"}]

        facts = reblessed.evidence(gh, "o/r", {"number": 9, "headRefOid": SHA})
        self.assertEqual(len(facts["files"]), 101)
        self.assertEqual(facts["pages"], ["page_alpha"])
        self.assertEqual(len(asked), 3)
        self.assertIn(f"ref={SHA}", asked[2])


if __name__ == "__main__":
    unittest.main()
