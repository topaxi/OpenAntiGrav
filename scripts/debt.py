#!/usr/bin/env python3
"""One overview of the project's known tech debt, generated from the code.

    python3 -I scripts/debt.py

Prints each ratchet's frozen baseline (entry count and the five largest
entries where "largest" means something) and every `kind: debt` handover
thread with its size and difficulty. It imports each check script's constant
and `scripts/handover-threads.py`'s parser; it changes no check's behaviour.

Standard library only.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent
TOP = 5


def load(name: str):
    spec = importlib.util.spec_from_file_location(
        name.replace("-", "_"), SCRIPTS / f"{name}.py"
    )
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def largest(entries: dict, unit: str) -> str:
    ranked = sorted(entries.items(), key=lambda kv: -kv[1])[:TOP]
    return "; ".join(f"{k} ({v:g} {unit})" for k, v in ranked)


def ratchets() -> list[tuple[str, str, int, str]]:
    """(script, constant, entry count, five largest or a note)."""
    rows = []

    def add(script, constant, value, unit=None):
        if unit is None:
            names = sorted(map(str, value))
            detail = "; ".join(names[:TOP])
        else:
            detail = largest(dict(value), unit)
        rows.append((script, constant, len(value), detail))

    size = load("check-file-size")
    add("check-file-size", "BASELINE", size.BASELINE, "lines")
    add("check-file-size", "TEST_BASELINE", size.TEST_BASELINE, "lines")
    reach = load("check-title-reach")
    add("check-title-reach", "BASELINE", reach.BASELINE, "refs")
    branching = load("check-title-branching")
    add("check-title-branching", "BASELINE", branching.BASELINE, "branches")
    strings = load("check-strings")
    for name in ("BASELINE_LABELS", "BASELINE_TITLES", "BASELINE_SUBTITLES"):
        add("check-strings", name, getattr(strings, name))
    names = load("check-ghidra-names")
    add("check-ghidra-names", "SUFFIX_BASELINE", names.SUFFIX_BASELINE)
    budget = load("check-test-budget")
    add("check-test-budget", "BASELINE", budget.BASELINE, "s")
    return rows


def debt_threads() -> list[tuple[str, dict]]:
    ht = load("handover-threads")
    out = []
    for path in ht.threads():
        fields, errors = ht.parse(path)
        if fields and fields.get("kind") == "debt":
            out.append((str(path.relative_to(ht.ROOT)), fields))
    return out


def estimate(fields: dict, key: str) -> str:
    value = fields.get(key)
    return "?" if not value else str(value)


def main() -> int:
    rows = ratchets()
    print("Ratchet baselines (frozen lists; each may only shrink)")
    print(f"{'script':<24} {'constant':<20} {'entries':>7}  largest / first")
    total = 0
    for script, constant, count, detail in rows:
        total += count
        print(f"{script:<24} {constant:<20} {count:>7}  {detail or '-'}")
    print(f"{'total':<24} {'':<20} {total:>7}")
    print()
    threads = debt_threads()
    print("Handover threads tagged `kind: debt`")
    for path, f in threads:
        print(
            f"{f['status']:<18} {estimate(f, 'size'):<8} "
            f"{estimate(f, 'difficulty'):<12} {path}"
        )
        print(f"    {f['summary']}")
    print(f"total: {len(threads)} threads")
    print()
    print(f"Overall: {total} frozen ratchet entries, {len(threads)} debt threads")
    return 0


if __name__ == "__main__":
    sys.exit(main())
