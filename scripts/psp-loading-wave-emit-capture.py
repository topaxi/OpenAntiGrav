#!/usr/bin/env python3
"""Runtime-verify the loading wave's *emitted* vertex Y, not just its
internal recurrence.

`psp-loading-wave-capture.py` confirms `local_70`'s own state evolution -
that script's own doc comment is explicit that this is the internal
recurrence, not the draw call's actual arguments. This script closes the
other half: it breaks on the generic quad emitter at `0x08810f84`
(`0x08804000 + 0xcf84`, the unrelocated `func_0x0000cf84` call target in
`Loading_DrawWave`'s decompile - confirmed against PPSSPP's own live
`memory.disasm`, not just Ghidra's, since Ghidra's function manager has no
`Function` object at that address even though the doc names it
`FUN_08810f84`) and reads `a0`/`a1` - the emitted `(x, y)` - directly, then
predicts `y` in Python from `local_70` and compares.

**A third trap, found getting this capture to work at all: a breakpoint on
code the JIT already compiled before the breakpoint existed does not take,
with no error and no distinguishing symptom** - `cpu.breakpoint.list` shows
it armed exactly as it should. Chaining this capture off `Loading_Show`'s
`a0=4` hit and *then* arming the draw breakpoint (the approach that works
for `psp-loading-wave-capture.py`'s column breakpoint) reproducibly times
out here, because the boot logo screen (type 0, drawn first) already calls
this same generic emitter for its own two sprites, and that already
JIT-compiled the block before this script ever got a chance to arm
anything. The fix is to arm the draw breakpoint **before the very first
`cpu.resume`**, from cold boot, and filter hits in Python by return address
(`ra`) - `0x0890ab60` is `Loading_DrawWave`'s per-layer call, `0x0890ac10`
its third-band call, anything else (the boot logo, or any other caller) is
skipped rather than treated as a wrong hit.

That filtering also sidesteps `local_70`'s address entirely: caught at the
callee's very first instruction, before its own prologue runs, `sp` is
still the *caller's* stack pointer - `Loading_DrawWave`'s own `sp`, and
therefore `local_70`'s address directly. No separate breakpoint to learn it
first (`jal` never touches `sp`).

The disassembly at both call sites was read directly, not assumed: the
per-layer draw (`ra=0x0890ab60`) truncates `raw[i] + 220.0` with
`trunc.w.s` (`0890ab38: add.s f12,f12,f26` / `0890ab3c: trunc.w.s f12,f12` /
`0890ab40: mfc1 a1,f12`, `f26` holding the literal `220.0`); the third-band
draw (`ra=0x0890ac10`) truncates `disp0*0.7 + disp1*0.3 + 220.0` the same
way. Each hit is classified after the fact by which of the three
predictions (per-layer i=0, per-layer i=1, third-band) actually matches
`a1`, not by assuming call order - `ra` alone cannot distinguish `i=0` from
`i=1`, since both go through the same call site.

Run this immediately against a freshly launched, never-before-connected-to
`PPSSPPHeadless` instance. **A fourth trap, distinct from the sibling
script's early process exit**: this breakpoint is far hotter than any other
used on this page - every quad drawn anywhere this early in boot goes
through it, not just the wave's own columns - and three separate runs all
stopped within the same narrow band, about 130-140 total hits (120 of them
always the boot logo, consistently, before any wave hit at all). Unlike the
sibling script's clean process exit, `ps` after the fact shows
`PPSSPPHeadless` still running and burning CPU - the debugger connection
stops answering, the process does not. Something about arming a breakpoint
this hot destabilizes PPSSPP's own JIT/breakpoint machinery after enough
hits; it has not been root-caused, and this script does not work around it.
Treat a capture that stops in that range as having hit this ceiling, not as
a bug in the script, and kill the orphaned process rather than reconnecting
to it - reported hung, not crashed, in every run so far.

**Every capture run so far landed entirely inside `x=0..14`** - a handful of
columns, well before `raw`/`disp` develop any real amplitude (`local_70` is
zeroed at the top of every call; per
`psp-loading-wave-capture.py`'s own captures, visible motion needs on the
order of a hundred-plus columns of accumulated random walk). Every predicted
`y` in that range is `220` regardless of which layer or band it came from,
because `raw`/`disp` are all still ~0 there - **so a match in this range
proves the argument-passing wiring is correct (the right value in the right
register, `trunc.w.s` applied, the `220.0` literal used) but does not by
itself distinguish `layer0` from `layer1`, and does not exercise the formula
at a value where a wiring bug would actually produce a visibly different
number.** The classification in the CSV (`call`) picks the first prediction
that matches when more than one does, which is why `layer1` has never once
appeared in a capture despite the call site firing exactly as often as
`layer0` - see the doc page's own "Not determined" section rather than
trusting the `layer0`/`layer1` split in isolation.

    uv run --with websocket-client scripts/psp-loading-wave-emit-capture.py \\
        --port 47800 --hits 200 --out /tmp/loading-wave-emit.csv
"""

import argparse
import csv
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger

DRAW_ENTRY = 0x08810F84  # generic quad emitter Loading_DrawWave calls
RA_PER_LAYER = 0x0890AB60
RA_THIRD_BAND = 0x0890AC10


def read_gpr(dbg):
    regs = dbg.call("cpu.getAllRegs")
    gpr = next(c for c in regs["categories"] if c["name"] == "GPR")
    return dict(zip(gpr["registerNames"], gpr["uintValues"]))


def to_signed32(v):
    return v - (1 << 32) if v & 0x80000000 else v


def c_trunc_toward_zero(v):
    return int(v)  # Python's int() on a float already truncates toward zero


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, default=47800)
    parser.add_argument("--hits", type=int, default=200, help="wave-draw hits to collect (boot-logo hits are skipped and not counted)")
    parser.add_argument("--max-skipped", type=int, default=2000, help="give up after this many non-wave hits")
    parser.add_argument("--out", default="/tmp/loading-wave-emit.csv")
    parser.add_argument("--per-hit-timeout", type=float, default=30.0)
    args = parser.parse_args()

    dbg = Debugger(args.port)
    status = dbg.call("cpu.status")
    if status["ticks"] != 0:
        raise SystemExit(
            "CPU has already run (ticks != 0) - this must be a freshly launched, "
            "never-before-connected-to PPSSPPHeadless instance. See the trap in "
            "this script's own docstring: a breakpoint on already-JIT-compiled "
            "code does not take."
        )
    dbg.add_breakpoint(DRAW_ENTRY)

    rows = []
    skipped = 0
    try:
        while len(rows) < args.hits and skipped < args.max_skipped:
            dbg.call("cpu.resume")
            dbg.wait_for_break(DRAW_ENTRY, timeout=args.per_hit_timeout)

            regs = read_gpr(dbg)
            ra = regs["ra"]
            if ra not in (RA_PER_LAYER, RA_THIRD_BAND):
                skipped += 1
                continue

            x = to_signed32(regs["a0"])
            y_actual = to_signed32(regs["a1"])
            local70 = regs["sp"]  # caller's sp: the callee's own prologue has not run yet
            disp0, disp1, raw0, raw1, energy0, energy1 = dbg.read_f32s(local70, 6)

            y_layer0 = c_trunc_toward_zero(raw0 + 220.0)
            y_layer1 = c_trunc_toward_zero(raw1 + 220.0)
            y_third = c_trunc_toward_zero(disp0 * 0.7 + disp1 * 0.3 + 220.0)

            if ra == RA_PER_LAYER and y_actual == y_layer0:
                call, match = "layer0", True
            elif ra == RA_PER_LAYER and y_actual == y_layer1:
                call, match = "layer1", True
            elif ra == RA_THIRD_BAND and y_actual == y_third:
                call, match = "third_band", True
            else:
                call, match = f"UNMATCHED(ra=0x{ra:08x})", False

            rows.append(
                dict(
                    hit=len(rows),
                    x=x,
                    ra=hex(ra),
                    y_actual=y_actual,
                    y_layer0_predicted=y_layer0,
                    y_layer1_predicted=y_layer1,
                    y_third_predicted=y_third,
                    call=call,
                    match=match,
                )
            )
    except TimeoutError as e:
        print(f"stopped early after {len(rows)} matched, {skipped} skipped: {e}")

    try:
        dbg.brk()
        for bp in dbg.call("cpu.breakpoint.list")["breakpoints"]:
            dbg.call("cpu.breakpoint.remove", address=bp["address"])
        dbg.resume()
        dbg.close()
    except Exception as e:
        print(f"cleanup after capture failed, likely because the emulator process exited: {e}")

    print(f"skipped {skipped} hits that were not the wave's own draw calls (boot logo etc.)")
    if not rows:
        raise SystemExit("captured zero wave-draw rows")

    with open(args.out, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        writer.writeheader()
        writer.writerows(rows)
    print(f"wrote {len(rows)} rows to {args.out}")

    matched = sum(1 for r in rows if r["match"])
    print(f"emitted y matched a prediction: {matched}/{len(rows)}")
    by_call = {}
    for r in rows:
        key = r["call"] if r["match"] else "UNMATCHED"
        by_call[key] = by_call.get(key, 0) + 1
    print("breakdown:", by_call)


if __name__ == "__main__":
    main()
