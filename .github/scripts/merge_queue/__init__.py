"""`merge-queue.py`'s parts, one responsibility a module (#1136).

The script's module doc is the design; its *Where each part lives* section
names what each module here holds. This file only loads the two sibling
scripts every part leans on — `mergeable.py` and `merge-watch.py` — once, by
path, since their names have hyphens in them.
"""

from __future__ import annotations

import importlib.util
from pathlib import Path

_SCRIPTS = Path(__file__).resolve().parent.parent

_spec = importlib.util.spec_from_file_location("mergeable", _SCRIPTS / "mergeable.py")
mergeable = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(mergeable)

_spec = importlib.util.spec_from_file_location("merge_watch", _SCRIPTS / "merge-watch.py")
watch = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(watch)
# One `mergeable` between the two, so the `Unreachable` the watch's questions
# raise is the class this script catches.
watch.mergeable = mergeable
