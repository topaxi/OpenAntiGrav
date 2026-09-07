#!/usr/bin/env python3
"""Filter `DumpDatabaseState.java`'s raw dumps into `docs/ghidra/captures/`.

The raw dump is everything a database holds. Most of that is not knowledge: it
is Ghidra's ELF loader inventing section markup, Ghidra's library signature
database recognising `strlen`, or vita-loader-redux resolving a NID. All of it
regenerates for free on a reimport, and capturing it would record a tool's
output as if somebody had recovered it - which is exactly the failure that made
the original question ("weeks of struct work are about to be deleted") look
true when it was not. See ADR-0047.

So this keeps only what a person wrote, and the filters are the argument:

- A **signature** is kept only when its source is not `IMPORTED` *and* its
  function carries a `Subsystem_VerbNoun` project name. Ghidra applies libc and
  libm prototypes in bulk; `erf`, `isalnum` and `getpid` are its recognitions,
  not ours.
- A **variable name** is kept on the same condition. Every named parameter in
  every database turns out to belong to a function Ghidra had a signature for
  (`__nptr`, `__s1`, `__x`), so the surviving set is empty everywhere.
- A **comment** is kept only when it is a plate comment at a real address with
  prose a person wrote. `namespace::offset` addresses are the ELF loader's own
  section table; `--- IMPORTED FUNCTION ---` is vita-loader-redux; "Common
  Information Entry" is the DWARF unwind reader.
- A **function name** is kept when its symbol source is `USER_DEFINED`.
  `ANALYSIS` covers the demangler and the NID resolver, which re-derive.
- **Labels** are kept whole: a `USER_DEFINED` non-function symbol is always
  somebody's doing.

Usage:
    scripts/capture-ghidra-state.py <raw-dump-dir>

`<raw-dump-dir>` is what `DumpDatabaseState.java` was pointed at. Run
`just capture-ghidra-state <dir>` rather than this directly.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "docs" / "ghidra" / "captures"

# The project's binaries, and the raw dump's filename stem for each. The stem is
# the program's DomainFile pathname with `/` turned into `_`, so it also records
# which of several same-named programs a capture came from.
BINARIES = {
    "psp-pulse-usa": "psp-pulse-usa_BOOT.BIN",
    "psp-pulse-eu": "psp-pulse-eu_BOOT.BIN",
    "psp-pure-usa": "psp-pure-usa_BOOT.BIN",
    "psp-pure-eu": "psp-pure-eu_BOOT.BIN",
    "ps2-pulse-eu": "ps2-pulse-eu_SCES_547.48",
    "ps3-hdfury-eu": "ps3-hdfury-eu_EBOOT.elf",
    "vita-2048-eu-v104": "vita-2048-eu-v104_eboot.elf",
}

PROJECT_NAME = re.compile(r"^[A-Z][A-Za-z0-9]*_[A-Z][A-Za-z0-9]*")
TOOL_COMMENT = re.compile(
    r"^(--- IMPORTED FUNCTION ---|Common Information Entry|Frame Descriptor Entry)"
)

HEADERS = {
    "functions.tsv": "# address\tname\tsource",
    "labels.tsv": "# address\tsymbol_type\tname\tnamespace",
    "comments.tsv": "# address\tkind\tfunction\ttext (\\n escaped)",
    "signatures.tsv": "# address\tname\tsource\tprototype",
    "variables.tsv": "# address\tfunction\tkind\tvariable\ttype\tstorage",
}


def rows(path: Path) -> list[str]:
    if not path.is_file():
        return []
    return [x for x in path.read_text().splitlines() if x.strip() and not x.startswith("#")]


def write(path: Path, rows_: list[str]) -> int:
    """Write a capture file, or remove it when there is nothing to say.

    An empty file and a missing one read identically to a person, and only one
    of them is honest about there being no such thing to capture.

    Rows are sorted, and that is load-bearing rather than tidiness. The format's
    job is letting two machines converge, so the same database must produce the
    same bytes on either one. Ghidra's iteration order is not a promised
    property, and leaving it to decide row order would turn every re-capture
    into a diff full of moved lines with no change in them - which is exactly
    the kind of noise that stops people running a sync at all.
    """
    path.parent.mkdir(parents=True, exist_ok=True)
    if not rows_:
        path.unlink(missing_ok=True)
        return 0
    ordered = sorted(rows_, key=lambda r: (r.split("\t")[0], r))
    path.write_text(HEADERS[path.name] + "\n" + "\n".join(ordered) + "\n")
    return len(ordered)


def capture(binary: str, stem: str, raw: Path) -> dict[str, int]:
    imported = {
        r.split("\t")[0] for r in rows(raw / f"{stem}.sigs.tsv") if r.split("\t")[2] == "IMPORTED"
    }

    signatures = [
        r
        for r in rows(raw / f"{stem}.sigs.tsv")
        if r.split("\t")[2] != "IMPORTED" and PROJECT_NAME.match(r.split("\t")[1])
    ]
    variables = [
        r
        for r in rows(raw / f"{stem}.vars.tsv")
        if r.split("\t")[0] not in imported and PROJECT_NAME.match(r.split("\t")[1])
    ]

    # A function's plate comment is reachable twice - through the Function
    # object, which knows the function's name, and through the listing's comment
    # iterator, which does not. They are one comment, so dedupe on address and
    # text rather than on either kind label; that also makes this filter
    # independent of which dump shape produced the input.
    #
    # Which of the two duplicates survives is pinned explicitly, on the third
    # column being populated. Keeping whichever arrived first would hand the
    # decision to the dump's iteration order, and a later re-run that happened
    # to iterate the other way would rewrite the kind and function columns on
    # every comment row without a single character of content changing - a diff
    # that looks like a finding and is not.
    best: dict[tuple[str, str], list[str]] = {}
    order: list[tuple[str, str]] = []
    for row in rows(raw / f"{stem}.comments.tsv"):
        fields = row.split("\t")
        address, kind, text = fields[0], fields[1], fields[-1]
        if not kind.startswith("PLATE") and kind != "FUNC_PLATE":
            continue
        if "::" in address or TOOL_COMMENT.match(text):
            continue
        key = (address, text)
        if key not in best:
            best[key] = fields
            order.append(key)
        elif not best[key][2].strip() and fields[2].strip():
            best[key] = fields
    comments = ["\t".join(best[k]) for k in order]

    labels = rows(raw / f"{stem}.labels.tsv")
    functions = [r for r in rows(raw / f"{stem}.funcs.tsv") if r.split("\t")[2] == "USER_DEFINED"]

    d = OUT / binary
    return {
        "functions.tsv": write(d / "functions.tsv", functions),
        "labels.tsv": write(d / "labels.tsv", labels),
        "comments.tsv": write(d / "comments.tsv", comments),
        "signatures.tsv": write(d / "signatures.tsv", signatures),
        "variables.tsv": write(d / "variables.tsv", variables),
    }


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("raw", type=Path, help="directory DumpDatabaseState.java wrote")
    ap.add_argument("--binary", help="capture one binary rather than all of them")
    args = ap.parse_args()

    if not args.raw.is_dir():
        print(f"{args.raw}: not a directory", file=sys.stderr)
        return 1

    wanted = {args.binary: BINARIES[args.binary]} if args.binary else BINARIES
    for binary, stem in wanted.items():
        if not (args.raw / f"{stem}.funcs.tsv").is_file():
            print(f"skip {binary}: no dump at {args.raw / (stem + '.funcs.tsv')}")
            continue
        counts = capture(binary, stem, args.raw)
        summary = " ".join(f"{k.removesuffix('.tsv')}={v}" for k, v in counts.items() if v)
        print(f"{binary:20s} {summary or 'nothing to capture'}")

    print("\nUpdate the measurement table in docs/ghidra/captures/README.md, then run")
    print("`just check-captures` - it asserts the table matches these files.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
