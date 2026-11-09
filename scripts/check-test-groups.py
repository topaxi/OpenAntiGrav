#!/usr/bin/env python3
"""The `gpu-vram` test group names every test that opens a GPU the unsafe way.

`.config/nextest.toml` caps how many tests of the `gpu-vram` group run at once,
because on a machine whose discrete GPU does not spill device memory into system
memory (NVIDIA on Linux) sixteen games drawing together exhaust it
(`wgpu error: Out of Memory`). The group is a list of test modules, and a list
rots: a new test that spawns the game and is not on it brings the failure back
on a loaded run, and nothing says why. This check keeps the list exact.

# What reaches a GPU, and what the group covers

Two kinds of test ask the driver for the machine's default adapter, which on a
laptop is its discrete GPU:

- a test that spawns the game (`CARGO_BIN_EXE_oag-game`, `CARGO_BIN_EXE_oag-view`):
  the binary's own `adapter::choose` takes wgpu's default pick;
- a test that calls `capture_pixels_from` / `capture_from` (`oag-mesh`), which
  does the same in process.

Every other in-process GPU test picks its adapter through
`oag_gpu::test_adapter`, which puts an integrated GPU ahead of a discrete one
that cannot spill. Those are not in the group; on a machine with no integrated
GPU each takes a few MiB to a few hundred (measured, `docs/architecture/workspace-layout.md`,
"Test groups"), which sixteen of them in parallel do not exhaust.

# What this checks

1. Per crate, the modules of `tests/*.rs` that contain one of those markers are
   exactly the modules the crate's `gpu-vram` override in `.config/nextest.toml`
   names. A missing module fails (the failure this exists for); a stale one
   fails too, so the list cannot only grow.
2. No marker sits in a place the list cannot name: a crate whose tests are one
   binary per file, a subdirectory of `tests/`, or an in-crate test module.
3. The call sites of `request_adapter` and `enumerate_adapters` are the known
   set. A new one is a new way to reach a GPU: decide whether a test can reach
   it, teach this script the marker if it can, and add it to `OPENERS`.

Run: `python3 scripts/check-test-groups.py` (no build, no GPU).
"""

from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GROUP = "gpu-vram"

MARKER = re.compile(
    r"CARGO_BIN_EXE_oag-(?:game|view)\b|\b(?:capture_pixels_from|capture_from)\b"
)

# `package(oag-game) & test(/^(a|b)::/)`, the only form an override of the group may take.
FILTER = re.compile(r"^package\(([\w-]+)\) & test\(/\^\(([\w|]+)\)::/\)$")

# The files that call `.request_adapter(` or `.enumerate_adapters(`. Each is
# either reached by the game binary (covered by the spawn marker), by the
# capture marker, or by no test.
OPENERS = {
    "crates/game/src/adapter.rs": "the game's adapter choice, reached by spawning the game",
    "crates/game/src/main/gpu.rs": "the game's window renderer, reached by spawning the game",
    "crates/gpu/src/test_adapter.rs": "the test adapter policy itself",
    "crates/mesh/src/capture.rs": "capture_pixels_from, the capture marker",
    "crates/view/src/main.rs": "oag-view, spawned by no test",
    "crates/view/src/offscreen.rs": "oag-view, spawned by no test",
    "crates/view/src/orbit.rs": "oag-view, spawned by no test",
}
OPENER = re.compile(r"\.(?:request_adapter|enumerate_adapters)\(")


def package_dirs() -> dict[str, Path]:
    out = {}
    for manifest in sorted((ROOT / "crates").glob("*/Cargo.toml")):
        name = tomllib.loads(manifest.read_text())["package"]["name"]
        out[name] = manifest.parent
    return out


def grouped_modules(errors: list[str]) -> dict[str, set[str]]:
    """Package name to the modules `.config/nextest.toml` puts in the group."""
    config = tomllib.loads((ROOT / ".config" / "nextest.toml").read_text())
    if GROUP not in config.get("test-groups", {}):
        errors.append(f"nextest.toml has no [test-groups] entry {GROUP!r}")
    named: dict[str, set[str]] = {}
    overrides = config.get("profile", {}).get("default", {}).get("overrides", [])
    for entry in overrides:
        if entry.get("test-group") != GROUP:
            continue
        found = FILTER.match(entry.get("filter", ""))
        if not found:
            errors.append(
                f"a {GROUP} override has the filter {entry.get('filter')!r}; "
                "it must read `package(NAME) & test(/^(a|b|c)::/)`"
            )
            continue
        package, modules = found.groups()
        named.setdefault(package, set()).update(modules.split("|"))
    return named


def marked_modules(packages: dict[str, Path], errors: list[str]) -> dict[str, set[str]]:
    """Package name to the `tests/*.rs` modules holding a marker."""
    marked: dict[str, set[str]] = {}
    by_dir = {path: name for name, path in packages.items()}
    for path in sorted((ROOT / "crates").rglob("*.rs")):
        rel = path.relative_to(ROOT)
        if "target" in rel.parts or not MARKER.search(path.read_text(errors="replace")):
            continue
        crate = by_dir.get(ROOT / "crates" / rel.parts[1])
        in_tests = len(rel.parts) > 3 and rel.parts[2] == "tests"
        is_test_module = rel.name == "tests.rs" or rel.name.endswith("_tests.rs")
        if not in_tests and not is_test_module:
            continue  # engine code or an example: no test of its own
        crate_dir = ROOT / "crates" / rel.parts[1]
        one_binary = any(crate_dir.glob("tests/it*.rs"))
        if not in_tests:
            errors.append(
                f"{rel}: an in-crate test module uses a default-adapter opener; "
                "this check does not name those, extend it"
            )
        elif len(rel.parts) != 4 or not one_binary:
            errors.append(
                f"{rel}: a marker in a crate without `tests/it*.rs` binaries, or in a "
                "subdirectory of tests/; extend this check to name it"
            )
        elif crate:
            marked.setdefault(crate, set()).add(rel.stem)
    return marked


def openers(errors: list[str]) -> None:
    seen = set()
    for path in sorted((ROOT / "crates").rglob("*.rs")):
        rel = path.relative_to(ROOT)
        if "target" in rel.parts:
            continue
        if OPENER.search(path.read_text(errors="replace")):
            seen.add(str(rel))
    for new in sorted(seen - OPENERS.keys()):
        errors.append(
            f"{new}: a new `request_adapter`/`enumerate_adapters` call site. If a test "
            f"can reach it, add its marker to MARKER here; then list the file in OPENERS"
        )
    for gone in sorted(OPENERS.keys() - seen):
        errors.append(f"{gone}: listed in OPENERS but no longer calls an adapter entry point")


def main() -> int:
    errors: list[str] = []
    named = grouped_modules(errors)
    marked = marked_modules(package_dirs(), errors)
    for package in sorted(set(named) | set(marked)):
        have, want = named.get(package, set()), marked.get(package, set())
        for module in sorted(want - have):
            errors.append(
                f"{package}: module {module} opens a GPU (spawns the game or captures a frame) "
                f"and is not in the {GROUP} group in .config/nextest.toml"
            )
        for module in sorted(have - want):
            errors.append(
                f"{package}: module {module} is in the {GROUP} group but no longer holds a marker"
            )
    openers(errors)
    if errors:
        print("check-test-groups: the gpu-vram group and the tests disagree\n", file=sys.stderr)
        for line in errors:
            print(f"  {line}", file=sys.stderr)
        return 1
    count = sum(len(v) for v in marked.values())
    print(f"check-test-groups: {count} test modules in the {GROUP} group, all accounted for")
    return 0


if __name__ == "__main__":
    sys.exit(main())
