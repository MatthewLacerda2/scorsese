#!/usr/bin/env python3
"""The backup's `POLICY_DAYS` is what the privacy page promises (#1657).

`deploy/backup/backup` refuses a keep past `POLICY_DAYS`; this holds the number
to every language of the privacy policy (pt-BR is the binding text), so a
change to one is a change to the other.
"""

from __future__ import annotations

import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "deploy" / "backup" / "backup"
LEGAL = ROOT / "web" / "src" / "legal"
# How each language gives a backup its days.
BACKUP_DAYS = {
    "pt-BR": r"cópias? de segurança\b[^.\"]*?\b(\d+) dias",
    "en": r"backups?\b[^.\"]*?\b(\d+) days",
    "es": r"copias? de seguridad\b[^.\"]*?\b(\d+) días",
}


def policy_days(language: str, text: str) -> set[int]:
    """Every number of days the privacy page gives a backup."""
    return {int(n) for n in re.findall(BACKUP_DAYS[language], text)}


class PolicyDays(unittest.TestCase):
    def test_the_policy_is_the_privacy_pages(self):
        days = re.search(r"^POLICY_DAYS=(\d+)$", SCRIPT.read_text(encoding="utf-8"), re.M)
        self.assertIsNotNone(days, "deploy/backup/backup sets no POLICY_DAYS")
        for language in BACKUP_DAYS:
            text = (LEGAL / language / "privacy.ts").read_text(encoding="utf-8")
            self.assertEqual(policy_days(language, text), {int(days.group(1))}, language)

    def test_it_reads_the_backup_sentences(self):
        text = (
            '"in a backup that is redone periodically and kept for 30 days.",'
            '"one that keeps you signed in for up to 30 days",'
            '"backups that still hold your data expire within 31 days."'
        )
        self.assertEqual(policy_days("en", text), {30, 31})
        text = (
            '"numa cópia de segurança, refeita periodicamente e mantida por 30 dias.",'
            '"um que mantém você conectado por até 30 dias",'
            '"as cópias de segurança que ainda tiverem os seus dados expiram em até 31 dias."'
        )
        self.assertEqual(policy_days("pt-BR", text), {30, 31})
        text = (
            '"una copia de seguridad que se rehace periódicamente y se conserva por 30 días.",'
            '"las copias de seguridad que aún tengan tus datos vencen en un máximo de 31 días."'
        )
        self.assertEqual(policy_days("es", text), {30, 31})


if __name__ == "__main__":
    unittest.main()
