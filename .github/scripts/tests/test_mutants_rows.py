#!/usr/bin/env python3
"""A survivor's row says which mutation it was, even beside its neighbours.

The failure this pins is #616's. #596's report listed two survivors on
`crates/compositor/src/cpu.rs:152` as `replaced with *` and `replaced with %`;
the follow-up read both as mutations of the division, and they were `+`→`*` at
column 52 and `/`→`%` at column 60 — different operators, different
assertions. `fixtures/two-on-one-line.json` is those two records as cargo-mutants
27.1.0 wrote them (diffs dropped), beside a caught mutant of the same line.

Exercised as a subprocess, on the argv contract, for the reason
`test_mutants_summary.py` gives.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
SCRIPT = HERE.parent / "mutants-summary.py"
FIXTURE = HERE / "fixtures" / "two-on-one-line.json"


def report(path: Path) -> str:
    done = subprocess.run(
        [sys.executable, str(SCRIPT), str(path)], capture_output=True, text=True, check=True
    )
    return done.stdout


def rows(out: str) -> list[str]:
    """The survivor table's data rows, without its heading and separator."""
    return [line for line in out.splitlines() if line.startswith("| `")]


def survivor(name: str, genre: str, replacement: str, function: str = "evaluate") -> dict:
    """A mutant as cargo-mutants records it, located by its own `name`."""
    file, line, column, _ = name.split(":", 3)
    return {
        "summary": "MissedMutant",
        "scenario": {
            "Mutant": {
                "name": name,
                "file": file,
                "function": {"function_name": function},
                "span": {"start": {"line": int(line), "column": int(column)}},
                "replacement": replacement,
                "genre": genre,
            }
        },
    }


def render(*outcomes: dict) -> str:
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "outcomes.json"
        path.write_text(
            json.dumps(
                {
                    "total_mutants": len(outcomes),
                    "missed": len(outcomes),
                    "caught": 0,
                    "timeout": 0,
                    "unviable": 0,
                    "outcomes": list(outcomes),
                }
            )
        )
        return report(path)


class TwoOnOneLine(unittest.TestCase):
    def test_each_row_names_its_column_and_its_original_operator(self) -> None:
        self.assertEqual(
            rows(report(FIXTURE)),
            [
                "| `crates/compositor/src/cpu.rs:152:52` | `demultiply` | `+` → `*` |",
                "| `crates/compositor/src/cpu.rs:152:60` | `demultiply` | `/` → `%` |",
            ],
        )


class Shapes(unittest.TestCase):
    FILE = "crates/core/src/keyframe.rs"

    def mutation(self, what: str, genre: str, replacement: str, function: str = "evaluate") -> str:
        """The Mutation cell for one survivor whose name describes `what`."""
        name = f"{self.FILE}:61:9: {what} in {function}"
        [line] = rows(render(survivor(name, genre, replacement, function)))
        return line.rsplit(" | ", 1)[1].removesuffix(" |")

    def test_a_function_body_names_the_function_and_not_its_signature(self) -> None:
        self.assertEqual(
            self.mutation("replace silenced -> Vec<u32> with vec![]", "FnValue", "vec![]", "silenced"),
            "replace `silenced` body with `vec![]`",
        )

    def test_a_pipe_is_escaped_so_it_does_not_end_the_cell(self) -> None:
        self.assertEqual(self.mutation("replace || with &&", "BinaryOperator", "&&"), "`\\|\\|` → `&&`")

    def test_a_function_name_with_spaces_is_still_stripped_exactly(self) -> None:
        self.assertEqual(
            self.mutation("replace + with -", "BinaryOperator", "-", "Canvas<'a, 'f>::brushed"),
            "`+` → `-`",
        )

    def test_a_guard_and_an_arm_and_a_field_each_quote_the_code_they_touched(self) -> None:
        self.assertEqual(
            self.mutation("replace match guard *a == b with true", "MatchArmGuard", "true"),
            "match guard `*a == b` → `true`",
        )
        self.assertEqual(
            self.mutation("delete match arm path::OPACITY", "MatchArm", ""),
            "delete match arm `path::OPACITY`",
        )
        self.assertEqual(
            self.mutation("delete field media from struct Asset expression", "StructField", ""),
            "delete field `media` from `Asset { … }`",
        )

    def test_a_description_of_unknown_shape_is_printed_as_written(self) -> None:
        self.assertEqual(self.mutation("swap the arguments", "SomethingNew", "x"), "swap the arguments")

    def test_a_record_without_a_name_falls_back_to_the_replacement(self) -> None:
        mutant = survivor(f"{self.FILE}:61:9: x", "BinaryOperator", "<=")
        del mutant["scenario"]["Mutant"]["name"]
        self.assertIn("| replaced with `<=` |", render(mutant))


if __name__ == "__main__":
    unittest.main()
