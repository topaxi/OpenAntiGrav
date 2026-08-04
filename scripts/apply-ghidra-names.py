#!/usr/bin/env python3
"""Applies documented symbol names to a Ghidra database through the MCP bridge.

The Ghidra project is not committed, so a fresh import starts at
`FUN_08940d0c` again. This script re-applies every name the documentation has
evidence for, which makes the database reproducible from the repository.

Two rules from [ADR-0005](../docs/architecture/adr/0005-ghidra-conventions.md)
are enforced here rather than trusted:

1. **No name without evidence.** Every row names the page that carries its
   evidence, and the row's address and name must both still appear on that page.
   A rename whose page has drifted is refused, not applied.
2. **Confidence is encoded in the name.** 70 and above is applied as written,
   50 to 69 gains a `_q` suffix, below 50 is skipped.

The bridge must be running with the target program open; see
docs/reverse-engineering/toolchain.md.

Note that the bridge's own `dry_run` parameter is **not** honoured by the rename
endpoints: it reports what it would do and then does it anyway. So `--dry-run`
here is implemented locally and sends nothing.

Usage:
    scripts/apply-ghidra-names.py                       # every default input
    scripts/apply-ghidra-names.py --dry-run
    scripts/apply-ghidra-names.py path/to/names.tsv
"""

from __future__ import annotations

import argparse
import json
import sys
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_INPUTS = [
    ROOT / "docs" / "ghidra" / "functions" / "psp-pulse-usa" / "names.tsv",
    ROOT / "data" / "ghidra" / "psp-imports.tsv",
]

QUALIFIED_MIN = 50
CONFIDENT_MIN = 70


class Bridge:
    def __init__(self, url: str, program: str) -> None:
        self.url = url.rstrip("/")
        self.program = program

    def get(self, endpoint: str, **params: str) -> str:
        if self.program:
            params.setdefault("program", self.program)
        query = urllib.parse.urlencode(params)
        with urllib.request.urlopen(f"{self.url}/{endpoint}?{query}", timeout=30) as r:
            return r.read().decode("utf-8", "replace")

    def post(self, endpoint: str, **body: object) -> str:
        if self.program:
            body.setdefault("program", self.program)
        data = json.dumps(body).encode()
        req = urllib.request.Request(
            f"{self.url}/{endpoint}",
            data=data,
            headers={"Content-Type": "application/json"},
        )
        with urllib.request.urlopen(req, timeout=60) as r:
            return r.read().decode("utf-8", "replace")

    def has_function(self, address: str) -> bool:
        return "No function found" not in self.get("get_function_by_address", address=address)

    def name_function(self, address: str, symbol: str) -> str:
        """Name a function, creating it first if Ghidra never found one.

        Auto-analysis leaves some import stubs undisassembled, so there is
        nothing to rename at a couple of dozen of them. The stub is eight bytes
        of known shape, so creating the function is safe rather than a guess.
        """
        if self.has_function(address):
            return self.post("rename_function_by_address", function_address=address, new_name=symbol)
        return self.post("create_function", address=address, name=symbol)

    def name_data(self, address: str, symbol: str) -> str:
        """Label a data address.

        `create_label` rather than `rename_or_label`: the bridge insists that
        globals carry a Hungarian type prefix, which is not this project's
        convention (see docs/ghidra/naming-conventions.md), and it rejects the
        rename outright. Labels are not policed.
        """
        return self.post("create_label", address=address, name=symbol)


class Row:
    __slots__ = ("address", "kind", "name", "confidence", "evidence", "source", "base_dir")

    def __init__(self, address, kind, name, confidence, evidence, source, base_dir):
        self.address = address
        self.kind = kind
        self.name = name
        self.confidence = confidence
        self.evidence = evidence
        self.source = source
        self.base_dir = base_dir

    @property
    def symbol(self) -> str:
        """The name as it should appear in Ghidra, `_q` included."""
        return self.name if self.confidence >= CONFIDENT_MIN else f"{self.name}_q"


def read_rows(path: Path) -> list[Row]:
    rows = []
    for lineno, line in enumerate(path.read_text().splitlines(), 1):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        fields = line.split("\t")
        if len(fields) != 5:
            raise SystemExit(f"{path}:{lineno}: expected 5 tab-separated fields, got {len(fields)}")
        address, kind, name, confidence, evidence = (f.strip() for f in fields)
        if kind not in ("function", "data"):
            raise SystemExit(f"{path}:{lineno}: kind must be `function` or `data`, got `{kind}`")
        if not confidence.isdigit():
            raise SystemExit(f"{path}:{lineno}: confidence must be a number, got `{confidence}`")
        rows.append(
            Row(
                address,
                kind,
                name,
                int(confidence),
                evidence,
                f"{path.name}:{lineno}",
                path.parent,
            )
        )
    return rows


def check_evidence(rows: list[Row]) -> list[str]:
    """Refuse any row whose evidence page no longer mentions it.

    The pages are the record of truth, so a disagreement means the row is stale,
    not that the page is.

    Evidence paths are relative to the TSV that carries the row, not to one
    fixed directory: psp-pulse-usa and ps2-pulse-eu each own their own names.tsv.
    """
    complaints = []
    cache: dict[Path, str] = {}
    for row in rows:
        if row.evidence == "-":
            continue
        page = (row.base_dir / row.evidence).resolve()
        if page not in cache:
            if not page.is_file():
                complaints.append(f"{row.source}: evidence page {row.evidence} not found")
                cache[page] = ""
                continue
            cache[page] = page.read_text()
        text = cache[page]
        if not text:
            continue
        if row.address not in text:
            complaints.append(f"{row.source}: {row.address} does not appear in {row.evidence}")
        if row.name not in text:
            complaints.append(f"{row.source}: name `{row.name}` does not appear in {row.evidence}")
    return complaints


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("inputs", nargs="*", type=Path, help="names TSVs (default: both known sets)")
    ap.add_argument("--url", default="http://127.0.0.1:8089", help="GhidraMCP bridge URL")
    ap.add_argument(
        "--program",
        default="/psp-pulse-usa/BOOT.BIN",
        help="program path in the Ghidra project",
    )
    ap.add_argument("--dry-run", action="store_true", help="print what would change, send nothing")
    ap.add_argument("--no-save", action="store_true", help="leave the program unsaved")
    args = ap.parse_args()

    inputs = args.inputs or [p for p in DEFAULT_INPUTS if p.is_file()]
    if not inputs:
        print("nothing to apply. Run `just resolve-imports` first?", file=sys.stderr)
        return 1

    rows: list[Row] = []
    for path in inputs:
        if not path.is_file():
            print(f"{path}: not found", file=sys.stderr)
            return 1
        found = read_rows(path)
        rows.extend(found)
        print(f"{path}: {len(found)} rows")

    seen: dict[str, Row] = {}
    for row in rows:
        if row.address in seen:
            print(
                f"{row.source}: duplicate address {row.address}, "
                f"first seen at {seen[row.address].source}",
                file=sys.stderr,
            )
            return 1
        seen[row.address] = row

    complaints = check_evidence(rows)
    if complaints:
        print("\nevidence check failed:", file=sys.stderr)
        for c in complaints:
            print(f"  {c}", file=sys.stderr)
        print(
            "\nThe pages are the record of truth (ADR-0005). Fix the row, or add "
            "the evidence to the page.",
            file=sys.stderr,
        )
        return 1

    bridge = Bridge(args.url, args.program)
    if not args.dry_run:
        try:
            bridge.get("get_metadata")
        except (urllib.error.URLError, OSError) as e:
            print(f"cannot reach the bridge at {args.url}: {e}", file=sys.stderr)
            return 1

    applied = skipped = failed = 0
    for row in sorted(rows, key=lambda r: r.address):
        if row.confidence < QUALIFIED_MIN:
            print(f"  skip  {row.address} {row.name} (confidence {row.confidence})")
            skipped += 1
            continue
        if args.dry_run:
            print(f"  would {row.kind:<8} {row.address} -> {row.symbol}")
            applied += 1
            continue
        try:
            if row.kind == "function":
                reply = bridge.name_function(row.address, row.symbol)
            else:
                reply = bridge.name_data(row.address, row.symbol)
        except (urllib.error.URLError, OSError) as e:
            print(f"  FAIL  {row.address} {row.symbol}: {e}")
            failed += 1
            continue
        # The bridge answers in several shapes, and one of the names applied here
        # contains the substring "Error", so match on the status rather than on
        # the presence of the word.
        # `already exists` is what a second run of the same names looks like, so
        # it counts as applied: this script has to be re-runnable.
        if (
            '"status": "success"' in reply
            or '"status":"success"' in reply
            or '"success":true' in reply
            or "already exists" in reply
        ):
            applied += 1
        else:
            print(f"  FAIL  {row.address} {row.symbol}: {reply[:200]}")
            failed += 1

    print(f"\n{applied} applied, {skipped} skipped, {failed} failed")
    if failed:
        return 1
    if not args.dry_run and not args.no_save:
        print(bridge.post("save_program").strip()[:160])
    return 0


if __name__ == "__main__":
    sys.exit(main())
