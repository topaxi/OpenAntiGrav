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

The bridge must be running with every target program open; see
docs/reverse-engineering/toolchain.md.

Note that the bridge's own `dry_run` parameter is **not** honoured by the rename
endpoints: it reports what it would do and then does it anyway. So `--dry-run`
here is implemented locally and sends nothing.

**Each binary owns its own `names.tsv` and its own Ghidra program.** A default
run (no positional args) walks every `docs/ghidra/functions/<binary>/names.tsv`
that exists and applies it to that binary's own program - `psp-pulse-eu` and
`psp-pulse-usa` alike, not just whichever one used to be hardcoded here. An
explicit input path is mapped to its program the same way, from the file's own
location (`program_for_path`) - `docs/ghidra/functions/ps3-hdfury-eu/names.tsv`
resolves to `/hdfury/EBOOT-ps3-hdfury-eu.elf` with no flag needed, and paths
targeting different binaries in one invocation are grouped and applied
separately. `--program` overrides that mapping - use it for a one-off run
against a binary with no `names.tsv` of its own yet (`program_for_path` has
nothing to map), or to replay a subset against a different program on
purpose. Before this, `--program` silently defaulted to
`/pulse/BOOT-psp-pulse-eu.BIN` for any explicit path with no flag, which
applied ps3-hdfury-eu's addresses to the PSP program and reported
false-positive successes there.

Usage:
    scripts/apply-ghidra-names.py                       # every binary's own names.tsv
    scripts/apply-ghidra-names.py --dry-run
    scripts/apply-ghidra-names.py docs/ghidra/functions/ps3-hdfury-eu/names.tsv
    scripts/apply-ghidra-names.py path/to/names.tsv --program /some/Binary
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FUNCTIONS_DIR = ROOT / "docs" / "ghidra" / "functions"

# Directory name under docs/ghidra/functions/ -> Ghidra project program path.
# Project folders are grouped by title (`/pulse`, `/pure`, `/hdfury`, `/omega`,
# `/2048`); each file's own name now carries the platform/region suffix that
# used to be its one-file-per-folder's folder name instead. `switch_program`
# disambiguates by basename alone, not by folder, so under the old layout four
# PSP `BOOT.BIN`s - and, once `/omega`'s `eboot.bin` and `/hdfury`'s
# `EBOOT.elf` were both open at once, two differently-cased `eboot`s too - were
# unreachable for writes "regardless of sequence" (HANDOVER.md), even though
# each already lived in its own uniquely-named folder. That is a bug this
# script cannot work around from the outside; only renaming the files
# themselves, in the Ghidra GUI, fixes it. Add a line here when a names.tsv
# appears for a binary not yet listed - the default run silently skips any
# names.tsv it can't map, printing which one and why, rather than guessing a
# program path.
BINARY_PROGRAMS = {
    "psp-pulse-usa": "/pulse/BOOT-psp-pulse-usa.BIN",
    "psp-pulse-eu": "/pulse/BOOT-psp-pulse-eu.BIN",
    "psp-pure-usa": "/pure/BOOT-psp-pure-usa.BIN",
    "psp-pure-eu": "/pure/BOOT-psp-pure-eu.BIN",
    # The PS2 build's main executable is `SCES_547.48` - Sony's disc product
    # code, not a `name.extension` pair - so the rename that gave every other
    # entry here a `-<binary>` suffix before its extension has nothing to
    # insert before; the suffix goes on the end instead. It was never part of
    # the basename collision (always the only program named `SCES_547.48`),
    # so this is cosmetic, not a fix.
    "ps2-pulse-eu": "/pulse/SCES_547.48-ps2-pulse-eu",
    "ps4-omega-eu": "/omega/eboot-ps4-omega-eu.bin",
    # The only entry naming a file that does not exist on its disc: a PS3
    # `EBOOT.BIN` is an encrypted SELF, and what gets imported is the ELF
    # `rpcs3 --decrypt` writes beside it. See
    # docs/reverse-engineering/toolchain.md#ps3.
    "ps3-hdfury-eu": "/hdfury/EBOOT-ps3-hdfury-eu.elf",
    # Vita `eboot.elf` is likewise not what ships on disc/in the PKG: it is
    # `scripts/vita-self-decrypt.py`'s output, not the SELF itself. Target of
    # record; the other three are corroboration-only. See
    # docs/reverse-engineering/toolchain.md#vita.
    "vita-2048-eu-v104": "/2048/eboot-vita-2048-eu-v104.elf",
    "vita-2048-usa-v104": "/2048/eboot-vita-2048-usa-v104.elf",
    "vita-2048-eu-base": "/2048/eboot-vita-2048-eu-base.elf",
    "vita-2048-usa-base": "/2048/eboot-vita-2048-usa-base.elf",
}

# The two resolve-imports outputs are matched by filename rather than by
# parent directory, since they live under gitignored data/ghidra/ rather than
# under a docs/ghidra/functions/<binary>/ directory of their own.
IMPORTS_TABLE_PROGRAMS = {
    "psp-imports.tsv": BINARY_PROGRAMS["psp-pulse-usa"],
    "psp-imports-eu.tsv": BINARY_PROGRAMS["psp-pulse-eu"],
}


def program_for_path(path: Path) -> str | None:
    """The Ghidra program an input path belongs to, from its location alone.

    Used for both the default run and an explicit-path one, so a path always
    maps to its own binary's program unless `--program` overrides it - an
    explicit `docs/ghidra/functions/ps3-hdfury-eu/names.tsv` used to silently
    fall back to `/psp-pulse-eu/BOOT.BIN` (ADR-0048's default) whenever
    `--program` was left off, applying one binary's addresses to another's
    program and reporting false-positive successes where the two happened to
    share a function name.
    """
    if path.name in IMPORTS_TABLE_PROGRAMS:
        return IMPORTS_TABLE_PROGRAMS[path.name]
    return BINARY_PROGRAMS.get(path.parent.name)


# (names.tsv, program) pairs used when no positional inputs are given. The
# per-binary files are discovered from BINARY_PROGRAMS so a new binary's
# names.tsv is picked up the moment it's added there; psp-imports.tsv is
# resolve-imports' own output and has always been USA-specific (see its
# `boot` default in the justfile), so it stays pinned to that one program
# rather than joining the generic per-binary loop.
#
# psp-imports-eu.tsv is the same idea for the EU binary - ADR-0048 made
# psp-pulse-eu the Ghidra target of record, but `just resolve-imports`'s own
# default still reads the USA-only `data/extracted/psp` path (see
# docs/reverse-engineering/methodology.md), so an EU imports table does not
# exist yet anywhere on a fresh checkout. It is paired here, additively,
# against the day someone runs `just resolve-imports boot=<eu boot.bin>
# out=data/ghidra/psp-imports-eu.tsv` - both files live under gitignored
# `data/`, so `present`'s `path.is_file()` filter below silently drops
# whichever one a given checkout does not have, the same as every other
# binary's names.tsv already does.
def default_inputs() -> list[tuple[Path, str]]:
    pairs = [
        (FUNCTIONS_DIR / binary / "names.tsv", program)
        for binary, program in BINARY_PROGRAMS.items()
    ]
    pairs.append((ROOT / "data" / "ghidra" / "psp-imports.tsv", BINARY_PROGRAMS["psp-pulse-usa"]))
    pairs.append((ROOT / "data" / "ghidra" / "psp-imports-eu.tsv", BINARY_PROGRAMS["psp-pulse-eu"]))
    return pairs


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

    def _true_current_executable_path(self) -> str:
        """The executable path of whatever program the bridge will actually
        write to right now - a plain `get_metadata` with no `program` query
        parameter at all, bypassing `get()`'s default so this can't quietly
        re-resolve to the program this Bridge wants rather than the one the
        bridge is actually holding as current."""
        with urllib.request.urlopen(f"{self.url}/get_metadata", timeout=30) as r:
            reply = r.read().decode("utf-8", "replace")
        for line in reply.splitlines():
            if line.startswith("Executable Path:"):
                return line.removeprefix("Executable Path:").strip()
        return ""

    def is_open(self) -> bool:
        """Whether this program currently exists in the bridge at all.

        A program the bridge has never opened still answers `get_metadata`
        with HTTP 200 and a JSON error body rather than raising, so this
        checks for the one line a real program always answers with, instead
        of trusting the status code.
        """
        return "Executable Path:" in self.get("get_metadata")

    def wait_until_open(self, timeout: float) -> bool:
        """Poll until this program shows up in the bridge, or `timeout` elapses.

        A program that was just imported, or a Ghidra instance that was just
        reopened after an import, can take several seconds before UDS
        discovery picks it up. Without this, a run right after
        `import-ps3-eboot.sh` fails with "Program not found" even though the
        import itself succeeded, and the fix is always just to run it again a
        moment later - so do that waiting here instead of making it manual.
        """
        deadline = time.monotonic() + timeout
        announced = False
        while True:
            if self.is_open():
                return True
            if time.monotonic() >= deadline:
                return False
            if not announced:
                print(f"  waiting for {self.program} to appear (up to {timeout:.0f}s)...")
                announced = True
            time.sleep(2)

    def switch_program(self) -> None:
        """Make this Bridge's program the bridge's active one.

        `rename_function_by_address` and `create_function` both ignore the
        `program` field on the request and act on whatever program the bridge
        currently considers active - unlike `get_function_by_address`, which
        resolves `program` correctly. Without this, a multi-binary run keeps
        writing every rename after the first group into whichever program was
        active when the *previous* group left off, and reports "No function
        found" for addresses that exist perfectly well in the intended one -
        or worse, silently succeeds by writing this group's names into that
        other program instead.

        `switch_program`'s own `"success": true` reply is not trustworthy:
        confirmed live, it reports success while leaving the bridge's true
        current program completely unchanged, for any of the three binaries
        that share the display name `BOOT.BIN` with `psp-pulse-usa`. So this
        does not trust the reply at all - it re-reads the true current
        program afterward with no `program` parameter, the same lookup a
        write call implicitly uses, and compares its executable path against
        what `get_metadata` reports *for the program this Bridge asked for*
        (which, like every other GET, resolves correctly). Only a match
        means a write will actually land where it's supposed to.
        """
        self.get("switch_program", program=self.program)
        wanted = self.get("get_metadata").strip()  # program set by get() -> resolves correctly
        wanted_path = next(
            (
                l.removeprefix("Executable Path:").strip()
                for l in wanted.splitlines()
                if l.startswith("Executable Path:")
            ),
            "",
        )
        actual_path = self._true_current_executable_path()
        if not wanted_path or actual_path != wanted_path:
            raise RuntimeError(
                f"switch_program to {self.program!r} did not take: bridge is still "
                f"writing to {actual_path!r}, not {wanted_path!r}"
            )

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
        # Word-boundary rather than substring, and either spelling counts.
        #
        # `names.tsv` stores the **bare** name; `Row.symbol` derives the `_q`
        # from the confidence column, so a page documenting a sub-70 name writes
        # `Craft_Construct_q` while its row says `Craft_Construct`. Both have to
        # satisfy this check, which is why the accepted set is `name|name_q`.
        #
        # The bare `in` test this replaces was too lenient in the other
        # direction: `Sap_Insert` is satisfied by a page that only mentions
        # `Sap_InsertPair`, so a row could keep pointing at evidence for a
        # neighbouring function after a rename. The boundaries close that
        # without breaking the suffix.
        if not re.search(rf"\b{re.escape(row.name)}(_q)?\b", text):
            complaints.append(f"{row.source}: name `{row.name}` does not appear in {row.evidence}")
    return complaints


def apply_group(paths: list[Path], program: str, args) -> int:
    """Apply every row from `paths` to one Ghidra `program`. Returns an exit code."""
    print(f"\n== {program} ==")
    rows: list[Row] = []
    for path in paths:
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

    bridge = Bridge(args.url, program)
    if not args.dry_run:
        try:
            if not bridge.wait_until_open(args.wait):
                print(
                    f"{program} never appeared at {args.url} within {args.wait:.0f}s - "
                    "is it open in a headful Ghidra instance? (the bridge needs a GUI, "
                    "not headless analysis)",
                    file=sys.stderr,
                )
                return 1
            bridge.switch_program()
        except (urllib.error.URLError, OSError) as e:
            print(f"cannot reach the bridge at {args.url} for {program}: {e}", file=sys.stderr)
            return 1
        except RuntimeError as e:
            # Refuse to guess which program is active: every rename below would
            # otherwise land wherever the *previous* group left the bridge,
            # silently writing this group's names into an unrelated binary.
            print(f"  {e}", file=sys.stderr)
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

    print(f"{applied} applied, {skipped} skipped, {failed} failed")
    if failed:
        return 1
    if not args.dry_run and not args.no_save:
        print(bridge.post("save_program").strip()[:160])
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument(
        "inputs", nargs="*", type=Path, help="names TSVs (default: every binary's own)"
    )
    ap.add_argument("--url", default="http://127.0.0.1:8089", help="GhidraMCP bridge URL")
    ap.add_argument(
        "--program",
        default=None,
        help="program path in the Ghidra project - overrides the mapping program_for_path() "
        "derives from each input's own location; only needed for a binary with no "
        "names.tsv of its own yet, or to replay a subset against a different program "
        "on purpose",
    )
    ap.add_argument(
        "--wait",
        type=float,
        default=60.0,
        help="seconds to wait for each target program to appear in the bridge before "
        "giving up, since a program just imported or a Ghidra instance just reopened "
        "can take a few seconds to show up (default 60; 0 disables waiting)",
    )
    ap.add_argument("--dry-run", action="store_true", help="print what would change, send nothing")
    ap.add_argument("--no-save", action="store_true", help="leave the program unsaved")
    args = ap.parse_args()

    if args.inputs:
        present = []
        for path in args.inputs:
            program = args.program or program_for_path(path)
            if program is None:
                print(
                    f"{path}: no known Ghidra program for {path.parent.name!r} - "
                    "add it to BINARY_PROGRAMS or pass --program explicitly",
                    file=sys.stderr,
                )
                return 1
            present.append((path, program))
    else:
        present = [(path, program) for path, program in default_inputs() if path.is_file()]
        if not present:
            print("nothing to apply. Run `just resolve-imports` first?", file=sys.stderr)
            return 1

        skipped_dirs = sorted(set(BINARY_PROGRAMS) - {p.parent.name for p, _ in present})
        for binary in skipped_dirs:
            print(f"skip  {binary}: no names.tsv yet")

    # Merge files that target the same program (psp-pulse-usa's own names.tsv
    # and psp-imports.tsv both do) so the duplicate-address check still sees
    # them together, not just within one file.
    by_program: dict[str, list[Path]] = {}
    for path, program in present:
        by_program.setdefault(program, []).append(path)

    worst = 0
    for program, paths in by_program.items():
        worst = max(worst, apply_group(paths, program, args))
    return worst


if __name__ == "__main__":
    sys.exit(main())
