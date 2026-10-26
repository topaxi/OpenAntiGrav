#!/usr/bin/env python3
"""List the `handover/` threads from their YAML frontmatter.

Every thread under `handover/<category>/` opens with a frontmatter block:

    ---
    status: ready-to-wire
    titles: [pulse, hd]
    updated: 2026-10-26
    summary: "One line, at most 150 characters, saying what is OPEN."
    categories: [rendering]      # optional: extra categories, on top of the directory
    ---

`status` is one of STATUSES, `titles` a list from TITLES (or `all`),
`updated` an ISO date, `summary` one line of at most 150 characters.
`scripts/check-handover-size.py` (`just check-handover`) refuses a thread
whose block is missing or invalid; this module owns the parser and the rules.

    python3 -I scripts/handover-threads.py [--status S] [--title T] [--category C] [--counts|--paths]

Standard library only.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HANDOVER = ROOT / "handover"

STATUSES = (
    "ready-to-wire",
    "re-first",
    "measure",
    "blocked-maintainer",
    "blocked-hardware",
    "parked",
)
TITLES = ("pulse", "pure", "hd", "2048", "omega", "all")
CATEGORIES = ("rendering", "gameplay", "frontend", "tooling", "audio")
KEYS = ("status", "titles", "updated", "summary", "categories")
REQUIRED = ("status", "titles", "updated", "summary")
SUMMARY_MAX = 150

DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")


def split_frontmatter(text: str) -> tuple[list[str] | None, str | None]:
    """Return (frontmatter lines, error). Lines are None when there is an error."""
    lines = text.split("\n")
    if not lines or lines[0].rstrip() != "---":
        return None, "no frontmatter (the file must start with `---`)"
    for i in range(1, len(lines)):
        if lines[i].rstrip() == "---":
            return lines[1:i], None
    return None, "frontmatter is never closed with `---`"


def parse_value(raw: str):
    raw = raw.strip()
    if raw.startswith('"'):
        return json.JSONDecoder().raw_decode(raw)[0]
    raw = re.split(r"\s#", raw, maxsplit=1)[0].strip()
    if raw.startswith("["):
        if not raw.endswith("]"):
            raise ValueError("unterminated list")
        inner = raw[1:-1].strip()
        return [p.strip() for p in inner.split(",")] if inner else []
    return raw


def parse(path: Path) -> tuple[dict | None, list[str]]:
    """Parse and validate one thread. Returns (fields, errors)."""
    parts = path.relative_to(HANDOVER).parts
    if len(parts) < 2 or parts[0] not in CATEGORIES:
        return None, [f"a thread lives in handover/<{'|'.join(CATEGORIES)}>/, not here"]
    block, err = split_frontmatter(path.read_text(encoding="utf-8"))
    if err:
        return None, [err]
    errors: list[str] = []
    fields: dict = {}
    for line in block:
        if not line.strip():
            continue
        m = re.match(r"^([a-z_]+):\s*(.*)$", line)
        if not m:
            errors.append(f"cannot parse frontmatter line: {line!r}")
            continue
        key, raw = m.groups()
        if key not in KEYS:
            errors.append(f"unknown frontmatter key `{key}`")
            continue
        if key in fields:
            errors.append(f"duplicate key `{key}`")
            continue
        try:
            fields[key] = parse_value(raw)
        except ValueError as e:
            errors.append(f"`{key}`: {e}")
    for key in REQUIRED:
        if key not in fields:
            errors.append(f"missing `{key}`")
    if errors:
        return None, errors

    if fields["status"] not in STATUSES:
        errors.append(f"status `{fields['status']}` is not one of {', '.join(STATUSES)}")
    titles = fields["titles"]
    if not isinstance(titles, list) or not titles:
        errors.append("`titles` must be a non-empty list like [pulse, hd]")
    else:
        bad = [t for t in titles if t not in TITLES]
        if bad:
            errors.append(f"unknown titles {bad}; allowed: {', '.join(TITLES)}")
        if "all" in titles and len(titles) > 1:
            errors.append("`all` must stand alone in `titles`")
    if not isinstance(fields["updated"], str) or not DATE.match(fields["updated"]):
        errors.append(f"`updated` must be YYYY-MM-DD, got {fields['updated']!r}")
    summary = fields["summary"]
    if not isinstance(summary, str) or not summary.strip():
        errors.append("`summary` is empty")
    elif len(summary) > SUMMARY_MAX:
        errors.append(f"`summary` is {len(summary)} characters, over {SUMMARY_MAX}")
    cats = fields.get("categories", [])
    if not isinstance(cats, list) or any(c not in CATEGORIES for c in cats):
        errors.append(f"`categories` must be a list from {', '.join(CATEGORIES)}")
    return (None, errors) if errors else (fields, [])


def threads() -> list[Path]:
    return sorted(HANDOVER.rglob("*.md")) if HANDOVER.is_dir() else []


def categories_of(path: Path, fields: dict) -> list[str]:
    return [path.relative_to(HANDOVER).parts[0], *fields.get("categories", [])]


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--status", choices=STATUSES)
    ap.add_argument("--title", choices=TITLES, help="threads naming this title (or `all`)")
    ap.add_argument("--category", choices=CATEGORIES, help="directory or extra category")
    ap.add_argument("--counts", action="store_true", help="print the count per status")
    ap.add_argument("--paths", action="store_true", help="print only the paths, one per line (for shuf)")
    args = ap.parse_args()

    rows = []
    bad = 0
    for path in threads():
        fields, errors = parse(path)
        if fields is None:
            bad += 1
            print(f"invalid frontmatter: {path.relative_to(ROOT)}: {errors[0]}", file=sys.stderr)
            continue
        if args.status and fields["status"] != args.status:
            continue
        if args.title and args.title not in fields["titles"] and "all" not in fields["titles"]:
            continue
        if args.category and args.category not in categories_of(path, fields):
            continue
        rows.append((path, fields))

    if args.counts:
        for s in STATUSES:
            print(f"{s}: {sum(1 for _, f in rows if f['status'] == s)}")
        print(f"total: {len(rows)}")
        return 1 if bad else 0

    order = {s: i for i, s in enumerate(STATUSES)}
    rows.sort(key=lambda r: (order[r[1]["status"]], _neg_date(r[1]["updated"]), str(r[0])))
    if args.paths:
        for path, _ in rows:
            print(path.relative_to(ROOT))
        return 1 if bad else 0
    for path, f in rows:
        titles = ",".join(f["titles"])
        print(f"{f['status']:<18} {titles:<12} {f['updated']} {path.relative_to(ROOT)}")
        print(f"    {f['summary']}")
    print(f"{len(rows)} threads", file=sys.stderr)
    return 1 if bad else 0


def _neg_date(d: str) -> tuple[int, ...]:
    """Newest first within a status."""
    return tuple(-int(p) for p in d.split("-"))


if __name__ == "__main__":
    sys.exit(main())
