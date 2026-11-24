#!/usr/bin/env python3
"""Reads every function's stack frame off an aarch64 Android library and fails on a big one.

    python3 scripts/check-android-stack.py target/aarch64-linux-android/dist/examples/liboag_android.so
    python3 scripts/check-android-stack.py LIB --top 20 --match 'oag_game::app'

NativeActivity runs `android_main` on a 2 MiB thread that `android-activity`
spawns itself, and winit must poll on that thread (its looper is bound to it), so
the game cannot choose the stack the front end and a race start on
(docs/tools/android.md, "The stack"). Frames of 480 to 545 KB under `android_main`,
with naga's recursive WGSL parser below them, ended in SIGSEGV. A frame is read off a
function's prologue by following the stack pointer: pre-indexed `stp`s, `sub sp, sp,
#imm [, lsl #12]`, and with stack probes `sub x9, sp, #imm, lsl #12` then
`sub x9, x9, #imm` and `mov sp, x9` (the probe loop's own 4 KiB steps are
overwritten by that `mov`). The peak over the first 300 instructions is the frame,
so a prologue after an early branch is still found. A call chain's
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

`--selftest` runs the prologue reader on disassembly snippets (no library, no
NDK): `just check-android-stack-script`, part of the gate.

Exit status: 1 if a checked frame is over `--max-frame`, a resident function is
missing or shows no prologue, 2 if the library could not be read or holds no
function symbols (a stripped one).
"""

from __future__ import annotations

import argparse
import os
import re
import shutil
import subprocess
import sys
import unittest
from pathlib import Path

HEADER = re.compile(r"^[0-9a-f]+ <(.+)>:$")
IMM = r"#(0x[0-9a-f]+|\d+)(?:, lsl #(\d+))?"
PRE_INDEX = re.compile(r"\b(?:stp|str)\s+.*\[sp, #-(0x[0-9a-f]+|\d+)\]!")
POST_INDEX = re.compile(r"\b(?:ldp|ldr)\s+.*\[sp\], #(0x[0-9a-f]+|\d+)")
SUB_SP = re.compile(rf"\bsub\s+sp, sp, {IMM}")
ADD_SP = re.compile(rf"\badd\s+sp, sp, {IMM}")
SUB_FROM_SP = re.compile(rf"\bsub\s+(x\d+), sp, {IMM}")
SUB_REG = re.compile(rf"\bsub\s+(x\d+), (x\d+), {IMM}")
SET_SP = re.compile(r"\b(?:mov|and)\s+sp, (x\d+)")
PROLOGUE_LINES = 300
MIN_FUNCTIONS = 100
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


def version_key(path: Path) -> list[int]:
    return [int(part) for part in re.findall(r"\d+", path.name)]


def objdump() -> str:
    """The NDK's llvm-objdump the way scripts/build-apk.sh finds the NDK, else the system's."""
    ndks: list[Path] = []
    if os.environ.get("ANDROID_NDK_HOME"):
        ndks.append(Path(os.environ["ANDROID_NDK_HOME"]))
    for root in (os.environ.get("ANDROID_HOME"), os.environ.get("ANDROID_SDK_ROOT"), "/opt/android-sdk"):
        if root and (Path(root) / "ndk").is_dir():
            ndks += sorted((Path(root) / "ndk").iterdir(), key=version_key, reverse=True)
    for ndk in ndks:
        for candidate in sorted(ndk.glob("toolchains/llvm/prebuilt/*/bin/llvm-objdump")):
            return str(candidate)
    found = shutil.which("llvm-objdump")
    if found:
        return found
    sys.exit("no llvm-objdump (set ANDROID_NDK_HOME, or install the NDK or LLVM)")


def immediate(value: str, shift: str | None = None) -> int:
    return int(value, 0) << int(shift or 0)


def frame_of(lines) -> int:
    """The largest stack reservation in a function's first instructions, in bytes."""
    delta = peak = 0
    regs: dict[str, int] = {}
    for count, line in enumerate(lines):
        if count >= PROLOGUE_LINES:
            break
        if m := PRE_INDEX.search(line):
            delta += immediate(m.group(1))
        elif m := POST_INDEX.search(line):
            delta -= immediate(m.group(1))
        elif m := SUB_SP.search(line):
            delta += immediate(*m.groups())
        elif m := ADD_SP.search(line):
            delta -= immediate(*m.groups())
        elif m := SUB_FROM_SP.search(line):
            regs[m.group(1)] = delta + immediate(m.group(2), m.group(3))
        elif (m := SUB_REG.search(line)) and m.group(1) == m.group(2) and m.group(1) in regs:
            regs[m.group(1)] += immediate(m.group(3), m.group(4))
        elif (m := SET_SP.search(line)) and m.group(1) in regs:
            delta = regs[m.group(1)]
        peak = max(peak, delta, *regs.values())
    return peak


def frames(lib: str):
    """Yields (name, bytes) for every function in `lib`; 0 is a function with no reservation found."""
    proc = subprocess.Popen(
        [objdump(), "-d", "--no-show-raw-insn", "--demangle", lib],
        stdout=subprocess.PIPE,
        text=True,
        errors="replace",
    )
    assert proc.stdout is not None
    name = None
    body: list[str] = []
    for line in proc.stdout:
        header = HEADER.match(line)
        if header:
            if name is not None:
                yield name, frame_of(body)
            name, body = header.group(1), []
        elif name is not None and len(body) < PROLOGUE_LINES:
            body.append(line)
    if name is not None:
        yield name, frame_of(body)
    if proc.wait() != 0:
        sys.exit(2)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("lib", nargs="?")
    parser.add_argument("--max-frame", type=int, default=64 * 1024, help="largest allowed frame, bytes (default 65536)")
    parser.add_argument("--top", type=int, default=15, help="how many of the largest to list")
    parser.add_argument("--all", action="store_true", help="check every function, not only the resident ones")
    parser.add_argument("--match", help="regex: list only functions whose demangled name matches")
    parser.add_argument("--selftest", action="store_true", help="run the prologue reader on fixture snippets and exit")
    args = parser.parse_args()
    if args.selftest:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(PrologueTests)
        return 0 if unittest.TextTestRunner(verbosity=1).run(suite).wasSuccessful() else 1
    if not args.lib or not Path(args.lib).is_file():
        print(f"no such library: {args.lib}", file=sys.stderr)
        return 2
    every = list(frames(args.lib))
    if len(every) < MIN_FUNCTIONS:
        print(f"only {len(every)} functions read from {args.lib}: stripped, or not a library", file=sys.stderr)
        return 2
    checked: list[tuple[int, str]] = []
    status = 0
    for pattern, required in [(p, True) for p in RESIDENT] + [(p, False) for p in RESIDENT_IF_PRESENT]:
        hits = [(size, name) for name, size in every if re.search(pattern, name)]
        if required and not hits and not args.all:
            print(f"MISSING: no function matches {pattern}", file=sys.stderr)
            status = 1
        for size, name in hits:
            if size == 0 and required:
                print(f"NO PROLOGUE: no stack reservation found in {name}", file=sys.stderr)
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


def disasm(text: str) -> list[str]:
    return [line.strip() for line in text.strip().splitlines()]


class PrologueTests(unittest.TestCase):
    """Real aarch64 prologue shapes, as llvm-objdump prints them."""

    def test_a_probed_frame_is_its_bound_plus_the_second_sub(self):
        # app::run on the unfixed build: 0x75000 + 0x690 below the saved registers.
        lines = disasm(
            """
            stp d9, d8, [sp, #-0x70]!
            stp x29, x30, [sp, #0x10]
            add x29, sp, #0x10
            sub x9, sp, #0x75, lsl #12 // =0x75000
            sub x9, x9, #0x690
            and x9, x9, #0xffffffffffffff80
            sub sp, sp, #0x1, lsl #12
            cmp sp, x9
            b.le 0x20
            ldr xzr, [sp]
            sub sp, sp, #0x1, lsl #12
            cmp sp, x9
            b.gt 0x1c
            mov sp, x9
            """
        )
        self.assertEqual(frame_of(lines), 0x70 + 0x75000 + 0x690)

    def test_an_aligned_frame_sets_sp_from_the_register(self):
        lines = disasm(
            """
            stp d9, d8, [sp, #-0x70]!
            sub x9, sp, #0x590
            and sp, x9, #0xffffffffffffff80
            """
        )
        self.assertEqual(frame_of(lines), 0x70 + 0x590)

    def test_a_plain_sub_counts_beside_an_unrelated_sub_from_sp(self):
        # The old reader took `probe or plain`: the small address computation hid the 300 KB frame.
        lines = disasm(
            """
            stp x29, x30, [sp, #-0x10]!
            sub x8, sp, #0x10
            sub sp, sp, #0x49, lsl #12
            sub sp, sp, #0x2e0
            """
        )
        self.assertEqual(frame_of(lines), 0x10 + 0x49000 + 0x2E0)

    def test_the_sub_after_a_probe_loop_is_not_dropped(self):
        lines = disasm(
            """
            stp x29, x30, [sp, #-0x20]!
            sub x9, sp, #0x2, lsl #12
            sub sp, sp, #0x1, lsl #12
            cmp sp, x9
            b.le 0x20
            ldr xzr, [sp]
            sub sp, sp, #0x1, lsl #12
            mov sp, x9
            sub sp, sp, #0x40
            """
        )
        self.assertEqual(frame_of(lines), 0x20 + 0x2000 + 0x40)

    def test_a_prologue_after_a_long_early_exit_is_found(self):
        # Shrink-wrapped: 60 instructions of fast path before the frame is made.
        fast = ["add x0, x0, #0x1"] * 60
        lines = fast + disasm("stp x29, x30, [sp, #-0x10]!\nsub sp, sp, #0x8, lsl #12")
        self.assertEqual(frame_of(lines), 0x10 + 0x8000)

    def test_a_function_that_restores_its_frame_still_reports_the_peak(self):
        lines = disasm(
            """
            stp x29, x30, [sp, #-0x10]!
            sub sp, sp, #0x400
            add sp, sp, #0x400
            ldp x29, x30, [sp], #0x10
            ret
            """
        )
        self.assertEqual(frame_of(lines), 0x410)

    def test_no_reservation_reads_zero(self):
        self.assertEqual(frame_of(disasm("add x0, x0, #0x1\nret")), 0)


if __name__ == "__main__":
    sys.exit(main())
