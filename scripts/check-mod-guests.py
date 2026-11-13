#!/usr/bin/env python3
"""Every mod's guest crate still passes its own tests, and its committed module is its build.

A simulation mod under `assets/mods/` ships two things from one crate: `guest/src`, the rules,
and `ext/<name>.wasm`, the module the game loads, which `guest/build.sh` builds from it and a
person commits. Each guest is a workspace of its own, so the engine's gate neither compiles it
nor runs its tests, and nothing ties the committed module to the source beside it. Both rot
quietly:

- the guest SDK moves and a guest's tests stop compiling (`oag.slipstream`'s did, on a changed
  `PortalView`), while the module committed before the change still loads and runs;
- a guest's source or the SDK changes and the module is not rebuilt, so what the game runs is
  not what the source says.

# What this checks

For every `assets/mods/*/*/guest/Cargo.toml`:

1. `cargo test` in the guest passes. A failure fails the check.
2. `cargo build --release --target wasm32-unknown-unknown` gives, byte for byte, the module
   committed under `ext/`. One that differs is reported as stale: rebuild it with the guest's
   `build.sh` and commit it. Without `--strict` staleness is a report; with it, a stale module
   fails, and so does a missing wasm target (a check that compared nothing must not pass).
   Install it with `rustup target add wasm32-unknown-unknown`.

A guest builds against the SDK in this checkout (`assets/mods/.cargo/config.toml` patches the
git dependency), with the toolchain `rust-toolchain.toml` pins, so the bytes are a function of
the tree.

`just` (the `check` recipe), `just gate-affected` and CI's `test` job run it with `--strict`.
A toolchain bump or an SDK change stales every module at once, which is the intended failure;
one command rebuilds them all: `just rebuild-guests`.
"""

import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TARGET = "wasm32-unknown-unknown"


# A guest is built the way its build.sh builds it, with no flags from the caller's shell. RUSTFLAGS
# feeds the crate hash, so CI's job-wide `-D warnings` changed every module's bytes (and turned the
# guests' test-only dead code into errors) while the same check passed locally.
GUEST_ENV = {
    key: value
    for key, value in os.environ.items()
    if key not in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS")
}


def run(guest: Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["cargo", *args], cwd=guest, capture_output=True, text=True, env=GUEST_ENV
    )


def has_target() -> bool:
    listed = subprocess.run(
        ["rustup", "target", "list", "--installed"], cwd=ROOT, capture_output=True, text=True
    )
    return listed.returncode == 0 and TARGET in listed.stdout.split()


def main() -> int:
    strict = "--strict" in sys.argv[1:]
    guests = sorted(ROOT.glob("assets/mods/*/*/guest/Cargo.toml"))
    if not guests:
        print("check-mod-guests: no guest crate under assets/mods (submodules not checked out)")
        return 0
    build = has_target()
    if not build:
        print(f"check-mod-guests: {TARGET} is not installed, modules not compared")
        if strict:
            print(f"    rustup target add {TARGET}")
            return 1
    failed, stale = [], []
    for manifest in guests:
        guest = manifest.parent
        name = guest.parent.relative_to(ROOT / "assets/mods")
        tests = run(guest, "test", "--quiet")
        if tests.returncode != 0:
            failed.append(name)
            tail = (tests.stdout + tests.stderr).strip().splitlines()[-12:]
            print(f"FAIL {name}: cargo test\n    " + "\n    ".join(tail))
            continue
        verdict = "tests pass"
        if build:
            built = run(guest, "build", "--release", "--target", TARGET, "--quiet")
            modules = sorted((guest / "target" / TARGET / "release").glob("*.wasm"))
            if built.returncode != 0 or not modules:
                failed.append(name)
                print(f"FAIL {name}: does not build for {TARGET}\n    {built.stderr.strip()[-400:]}")
                continue
            for module in modules:
                committed = guest.parent / "ext" / module.name
                if not committed.is_file():
                    stale.append(name)
                    verdict += f", ext/{module.name} is not committed"
                elif committed.read_bytes() != module.read_bytes():
                    stale.append(name)
                    verdict += (
                        f", ext/{module.name} is stale ({committed.stat().st_size} bytes "
                        f"committed, {module.stat().st_size} built)"
                    )
                else:
                    verdict += f", ext/{module.name} is its build"
        print(f"ok   {name}: {verdict}")
    if stale:
        print(
            f"\n{len(stale)} stale module(s): run the guest's build.sh and commit ext/. "
            + ("Failing: --strict." if strict else "Reported, not failed (--strict fails them).")
        )
    return 1 if failed or (strict and stale) else 0


if __name__ == "__main__":
    sys.exit(main())
