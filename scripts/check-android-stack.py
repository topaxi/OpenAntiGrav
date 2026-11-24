#!/usr/bin/env python3
"""Reads every function's stack frame off an aarch64 Android library and fails on a big one.

    python3 scripts/check-android-stack.py target/aarch64-linux-android/dist/examples/liboag_android.so
    python3 scripts/check-android-stack.py LIB --top 20 --match 'oag_game::app'

NativeActivity runs `android_main` on a 2 MiB thread that `android-activity`
spawns itself, and winit must poll on that thread (its looper is bound to it), so
the game cannot choose the stack the front end and a race start on
(docs/tools/android.md, "The stack"). Frames of 480 to 545 KB under `android_main`,
with naga's recursive WGSL parser below them, ended in SIGSEGV. A frame is read off a
function's prologue: pre-indexed `stp`s, then `sub sp, sp, #imm [, lsl #12]`, or
with stack probes `sub x9, sp, #imm, lsl #12` and `sub x9, x9, #imm`. A call chain's
depth is not computed, so a limit is a margin, not a proof; the on-device `--race`
run is the proof.

By default it checks the functions that stay on the stack for the whole run
(`RESIDENT`: `android_main`, the window loop, the callback winit calls, the
race stage's entry) against `--max-frame` (64 KiB), and fails if one of them is
gone from the library, so a rename cannot leave a name unchecked. `--all` checks
every function instead (some elsewhere are big on purpose: a decoder's, a
capture's, anything on its own thread). `--top N` lists the largest either way.

Needs `llvm-objdump` (the NDK's, or the system's). The library must keep its
symbols: point it at the `target/.../examples/liboag_android.so` that
`scripts/build-apk.sh` builds, not the stripped copy in the APK.

Exit status: 1 if a checked frame is over `--max-frame` or a resident function
is missing, 2 if the library could not be read.
"""

from __future__ import annotations

import argparse
import re
import shutil
import subprocess
import sys
from pathlib import Path

HEADER = re.compile(r"^[0-9a-f]+ <(.+)>:$")
PRE_INDEX = re.compile(r"\bstp\s+\w+, \w+, \[sp, #-(0x[0-9a-f]+|\d+)\]!")
SUB_SP = re.compile(r"\bsub\s+sp, sp, #(0x[0-9a-f]+|\d+)(?:, lsl #(\d+))?\s*(?://.*)?$")
PROBE = re.compile(r"\bsub\s+(x\d+), sp, #(0x[0-9a-f]+|\d+)(?:, lsl #(\d+))?\s*(?://.*)?$")
PROBE_MORE = re.compile(r"\bsub\s+(x\d+), (x\d+), #(0x[0-9a-f]+|\d+)(?:, lsl #(\d+))?\s*(?://.*)?$")
PROLOGUE_LINES = 40
# Functions on the stack under every callback and every frame of a run. Each is a regex
# matched against the demangled name; each must match something in the library.
RESIDENT = (
    r"^oag_game::app::android_main$",
    r"^oag_game::app::run_window$",
    r"EventLoop<\(\)>>::run_app<oag_game::app::handler::App>::\{closure#0\} as core::ops::function::FnMut",
    r"^<oag_game::app::stage::Stage>::race(::<.*>)?$",
)
# Resident too, but the optimiser may fold them into their caller, so a missing one is not a failure.
RESIDENT_IF_PRESENT = (r"^oag_game::app::run$",)


def objdump() -> str:
    for candidate in sorted(Path("/opt/android-sdk/ndk").glob("*/toolchains/llvm/prebuilt/*/bin/llvm-objdump")):
        return str(candidate)
    found = shutil.which("llvm-objdump")
    if found:
        return found
    sys.exit("no llvm-objdump (install the NDK or LLVM)")


def immediate(value: str, shift: str | None) -> int:
    return int(value, 0) << int(shift or 0)


def frames(lib: str):
    """Yields (name, bytes) for every function whose prologue reserves stack."""
    proc = subprocess.Popen(
        [objdump(), "-d", "--no-show-raw-insn", "--demangle", lib],
        stdout=subprocess.PIPE,
        text=True,
        errors="replace",
    )
    assert proc.stdout is not None

    def total(pre: int, plain: int, probe: int) -> int:
        # A probed frame loops `sub sp, sp, #1, lsl #12`: those are not extra.
        return pre + (probe or plain)

    name = None
    seen = pre = plain = probe = 0
    reg = None
    for line in proc.stdout:
        header = HEADER.match(line)
        if header:
            if name is not None:
                yield name, total(pre, plain, probe)
            name, seen, pre, plain, probe, reg = header.group(1), 0, 0, 0, 0, None
            continue
        if name is None or seen >= PROLOGUE_LINES:
            continue
        seen += 1
        if m := PRE_INDEX.search(line):
            pre += int(m.group(1), 0)
        elif m := SUB_SP.search(line):
            plain += immediate(*m.groups())
        elif (m := PROBE.search(line)) and reg is None:
            # `sub x9, sp, #N, lsl #12` then `sub x9, x9, #M` is the probed
            # frame's lowest address; the loop that follows only touches pages.
            reg = m.group(1)
            probe = immediate(m.group(2), m.group(3))
        elif reg and (m := PROBE_MORE.search(line)) and m.group(1) == reg and m.group(2) == reg:
            probe += immediate(m.group(3), m.group(4))
    if name is not None:
        yield name, total(pre, plain, probe)
    if proc.wait() != 0:
        sys.exit(2)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("lib")
    parser.add_argument("--max-frame", type=int, default=64 * 1024, help="largest allowed frame, bytes (default 65536)")
    parser.add_argument("--top", type=int, default=15, help="how many of the largest to list")
    parser.add_argument("--all", action="store_true", help="check every function, not only the resident ones")
    parser.add_argument("--match", help="regex: list only functions whose demangled name matches")
    args = parser.parse_args()
    if not Path(args.lib).is_file():
        print(f"no such library: {args.lib}", file=sys.stderr)
        return 2
    every = list(frames(args.lib))
    checked: list[tuple[int, str]] = []
    status = 0
    for pattern, required in [(p, True) for p in RESIDENT] + [(p, False) for p in RESIDENT_IF_PRESENT]:
        hits = [(size, name) for name, size in every if re.search(pattern, name)]
        if not hits and required and not args.all:
            print(f"MISSING: no function matches {pattern}", file=sys.stderr)
            status = 1
        checked += hits
    if args.all:
        checked = [(size, name) for name, size in every]
    listed = [(size, name) for name, size in every if not args.match or re.search(args.match, name)]
    for size, name in sorted(listed, reverse=True)[: args.top]:
        print(f"{size / 1024:9.1f} KiB  {name[:150]}")
    over = [(size, name) for size, name in checked if size > args.max_frame]
    for size, name in sorted(over, reverse=True):
        print(f"OVER {args.max_frame // 1024} KiB: {size / 1024:.1f} KiB  {name[:150]}", file=sys.stderr)
    print(f"{len(every)} functions read, {len(checked)} checked, {len(over)} over {args.max_frame // 1024} KiB")
    return 1 if over else status


if __name__ == "__main__":
    sys.exit(main())
