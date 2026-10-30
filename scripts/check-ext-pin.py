#!/usr/bin/env python3
"""Every example guest pins the guest SDK this tree carries.

A guest crate under `assets/mods/examples/*/guest/` depends on `oag-ext` as a git
dependency on this repository at a `rev`, so a stand-alone clone of the examples
builds. Inside this checkout `assets/mods/.cargo/config.toml` patches the pin to the
local `crates/ext`, so nothing here notices a stale pin; this does. The pin must be a
commit in this repository's history whose `crates/ext` and `crates/ext-abi` equal the
working tree's. After an SDK change: commit it, set `rev` to that commit in each guest,
commit the examples, then the submodule pointer.
"""

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SDK = ["crates/ext", "crates/ext-abi"]
PIN = re.compile(
    r'^oag-ext\s*=\s*\{[^}]*git\s*=\s*"https://github.com/topaxi/OpenAntiGrav"[^}]*rev\s*=\s*"([0-9a-f]{7,40})"',
    re.M,
)


def git(*args: str) -> subprocess.CompletedProcess:
    return subprocess.run(["git", "-C", str(ROOT), *args], capture_output=True, text=True)


def main() -> int:
    guests = sorted(ROOT.glob("assets/mods/examples/*/guest/Cargo.toml"))
    if not guests:
        print("check-ext-pin: no example guests (submodule not checked out?)", file=sys.stderr)
        return 1
    failures = []
    for manifest in guests:
        name = manifest.relative_to(ROOT)
        text = manifest.read_text()
        revs = set(PIN.findall(text))
        if not revs or "path =" in "".join(l for l in text.splitlines() if l.startswith("oag-ext")):
            failures.append(f"{name}: oag-ext is not a git dependency on the engine with a rev")
            continue
        for rev in sorted(revs):
            if git("cat-file", "-e", f"{rev}^{{commit}}").returncode != 0:
                failures.append(f"{name}: pin {rev} is not in this repository's history")
            elif git("diff", "--quiet", rev, "--", *SDK).returncode != 0:
                failures.append(f"{name}: pin {rev[:9]} predates the SDK in this tree; bump it")
    for failure in failures:
        print(f"check-ext-pin: {failure}", file=sys.stderr)
    if not failures:
        print(f"check-ext-pin: {len(guests)} guests pin the SDK this tree carries")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
