#!/usr/bin/env python3
"""Reads every function's stack frame off an aarch64 Android library and fails on a big one.

    python3 scripts/check-android-stack.py target/aarch64-linux-android/dist/examples/liboag_android.so
    python3 scripts/check-android-stack.py LIB --top 20 --match 'oag_game::app'

NativeActivity runs `android_main` on a 2 MiB thread that `android-activity`
spawns itself, and winit must poll on that thread (its looper is bound to it), so
the game cannot choose the stack the front end and the `--race` route start on
(docs/tools/android.md, "The stack"). One frame of 500 KB there, with naga's
recursive WGSL parser below it, ended in SIGSEGV. A frame is read off a
function's prologue: `stp x29, x30, [sp, #-N]!`, then `sub sp, sp, #imm
[, lsl #12]` (a frame over 4 KiB is built in 4 KiB steps; with stack probes on
it is a probe loop whose bound is `sub x9, sp, #imm, lsl #12`). A call chain's
depth is not computed, so `--max-frame` is a margin, not a proof; the on-device
`--race` run is the proof.

Needs `llvm-objdump` (the NDK's, or the system's). The library must not be
stripped of its symbols: build with `scripts/build-apk.sh` and point at the
`target/.../examples/liboag_android.so` it prints, not the stripped copy in the
APK.

Exit status: 1 if a function's frame is over `--max-frame`, 2 if the library
could not be read.
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
            if name is not None and total(pre, plain, probe):
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
    if name is not None and total(pre, plain, probe):
        yield name, total(pre, plain, probe)
    if proc.wait() != 0:
        sys.exit(2)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("lib")
    parser.add_argument("--max-frame", type=int, default=64 * 1024, help="largest allowed frame, bytes (default 65536)")
    parser.add_argument("--top", type=int, default=15, help="how many of the largest to list")
    parser.add_argument("--match", help="regex: only functions whose demangled name matches are checked and listed")
    args = parser.parse_args()
    if not Path(args.lib).is_file():
        print(f"no such library: {args.lib}", file=sys.stderr)
        return 2
    pattern = re.compile(args.match) if args.match else None
    found = sorted(
        ((size, name) for name, size in frames(args.lib) if pattern is None or pattern.search(name)),
        reverse=True,
    )
    for size, name in found[: args.top]:
        print(f"{size / 1024:9.1f} KiB  {name[:150]}")
    over = [(size, name) for size, name in found if size > args.max_frame]
    print(f"{len(found)} frames read, {len(over)} over {args.max_frame // 1024} KiB")
    return 1 if over else 0


if __name__ == "__main__":
    sys.exit(main())
