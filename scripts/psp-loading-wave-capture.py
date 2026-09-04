#!/usr/bin/env python3
"""Runtime-verify the loading screen's wave recurrence at a `Loading_DrawWave`
breakpoint.

`docs/ghidra/functions/psp-pulse-usa/loading-screen.md#the-motion` reads the
whole two-layer oscillator off the decompiler at confidence 85 and says
plainly: the motion model has never been checked against the game actually
running. This closes that gap.

`Loading_DrawWave` (`0x0890a8e4`) keeps its whole state, `local_70[6]`, in a
stack array that Ghidra reports as `Stack[-0x70]` inside a frame the prologue
allocates with `addiu sp,sp,-0x70` - so at runtime the array's address is
simply the function's own `sp` after the prologue, no offset arithmetic
needed (confirmed by reading the disassembly, not assumed). The breakpoint
address below (`0x0890abbc`) sits right after each column's per-layer update
completes, once per column rather than twice, so `s5` (the column index) and
`sp+0..0x14` (the six floats) are both stable at the point of the hit.

Because each column's `raw`/`disp` update is a deterministic function of the
*previous* column's captured state (the only unpredictable input is the
`rand()`-driven impulse folded into `energy`, and that only feeds the column
after next), consecutive hits let a closed-form prediction in Python be
compared directly against what the game computed - the same
predict-then-diff shape as `psp-watch-soundemitter.py`, not a fresh pattern.
The array is reset to zero at the top of every `Loading_DrawWave` call, so a
capture long enough to cross a call boundary (column index wrapping back to
0) checks that too.

**Two traps this script works around, both cost a session each to find:**

1. `g_loading_finished` trips and the "Load screen" kernel thread exits
   itself a handful of frames later - fast enough in wall-clock time, running
   unthrottled, that connecting first and *then* arming a breakpoint misses
   the whole window. So this script chains straight from catching
   `Loading_Show`'s `a0=4` (tip/wave screen) hit into arming the column
   breakpoint, with no idle time between - never park on `Loading_Show` and
   reconnect later.
2. **A leftover breakpoint from an earlier connection silently defeats a new
   one.** PPSSPP v1.20.4 only fires the most-recently-added execution
   breakpoint (`ppsspp-debugger.md#each_hit_any`); if a stale breakpoint from
   a prior probe is still registered, arming a fresh one can leave the CPU
   running straight through it with no error at all. Confirmed directly:
   `Loading_Show`'s breakpoint timed out for several minutes of real
   wall-clock (given `screen_type` had visibly already flipped to 4 in
   memory - the call plainly happened) until every existing breakpoint was
   explicitly cleared first. This script always clears the full
   `cpu.breakpoint.list` before arming anything.

Requires a PPSSPP already broken at start with the debugger enabled -
`PPSSPPHeadless data/images/pulse-psp-usa.chd --debugger=47800
--graphics=software --timeout=1800` boots straight to a `cpu.stepping` break
at `pc=0x08804000`, no ISO extraction needed on v1.20.4. Run this
immediately against a freshly launched, never-before-connected-to instance -
per trap 2 above, a prior probe against the same process is exactly the
condition that has bitten this capture already.

    uv run --with websocket-client scripts/psp-loading-wave-capture.py \\
        --port 47800 --hits 260 --out /tmp/loading-wave.csv
"""

import argparse
import csv
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger

LOADING_SHOW = 0x0890AED8  # entry point, dispatches on a0 = screen type
COLUMN_BREAK = 0x0890ABBC  # inside Loading_DrawWave, once per column, after both layers update
PHASE = 0x08ABF470  # g_loading_wave_phase
RATES = 0x08ABF474  # g_loading_wave_rates, two floats {0.1, 0.05}
ENVELOPE = 0x08A88034  # g_loading_wave_envelope, 24 floats
SCREEN_TYPE = 0x08ABF45C  # g_loading_screen_type
FINISHED = 0x08ABF454  # g_loading_finished
TIP_SCREEN = 4


def read_gpr(dbg):
    regs = dbg.call("cpu.getAllRegs")
    gpr = next(c for c in regs["categories"] if c["name"] == "GPR")
    return dict(zip(gpr["registerNames"], gpr["uintValues"]))


def clear_breakpoints(dbg):
    """Remove every armed execution breakpoint. See trap 2 in the module
    docstring: a leftover breakpoint from an earlier connection silently
    outranks a freshly-added one on this PPSSPP version."""
    for bp in dbg.call("cpu.breakpoint.list")["breakpoints"]:
        dbg.call("cpu.breakpoint.remove", address=bp["address"])


def c_idiv(a, b):
    """C-style integer division, truncating toward zero, not toward -inf."""
    q = a // b
    if a % b != 0 and (a < 0) != (b < 0):
        q += 1
    return q


def ramp255(lo, hi, x):
    v = c_idiv((x - lo) * 255, hi - lo)
    return max(0, min(255, v))


def slew_toward(current, target, step, threshold):
    if abs(current - target) <= threshold:
        return current
    if current < target:
        moved = current + step
        return moved if moved <= target else target
    moved = current - step
    return moved if moved >= target else target


def predict(prev, x, pulse, rates):
    """Predict this column's disp/raw from the previous column's captured
    state plus this column's own x (envelope input) and pulse (same for the
    whole draw call). Energy's next value is not predicted - it carries an
    unpredictable rand() impulse - only its damping factor is checked."""
    x_env = ramp255(30, 286, x) / 255.0
    pulse_factor = pulse / 99.0 + 0.1
    out = {}
    for i in (0, 1):
        target = prev[f"energy{i}"] * x_env * pulse_factor
        raw = prev[f"raw{i}"] + (target - prev[f"raw{i}"]) * rates[i]
        disp = slew_toward(prev[f"disp{i}"], raw, 0.5, 4.0)
        out[f"raw{i}"] = raw
        out[f"disp{i}"] = disp
    return out


def wait_for_tip_screen(dbg, max_show_hits=6, timeout=90.0):
    """Run until `Loading_Show(4)` fires, leaving the CPU stopped right
    there. Every earlier `Loading_Show` call (the boot logo, type 0) is
    skipped over rather than treated as a failure."""
    clear_breakpoints(dbg)
    dbg.add_breakpoint(LOADING_SHOW)
    for _ in range(max_show_hits):
        dbg.call("cpu.resume")
        dbg.wait_for_break(LOADING_SHOW, timeout=timeout)
        regs = read_gpr(dbg)
        if regs["a0"] == TIP_SCREEN:
            return
    raise SystemExit(f"Loading_Show never passed a0={TIP_SCREEN} in {max_show_hits} hits")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, default=47800)
    parser.add_argument("--hits", type=int, default=260)
    parser.add_argument("--out", default="/tmp/loading-wave.csv")
    parser.add_argument("--per-hit-timeout", type=float, default=20.0)
    args = parser.parse_args()

    dbg = Debugger(args.port)
    dbg.brk()

    rates = dbg.read_f32s(RATES, 2)
    print(f"g_loading_wave_rates read live: {rates}")

    wait_for_tip_screen(dbg)
    print("Loading_Show(4) hit - chaining straight into the column breakpoint")

    clear_breakpoints(dbg)
    dbg.add_breakpoint(COLUMN_BREAK)

    rows = []
    try:
        for i in range(args.hits):
            dbg.call("cpu.resume")
            dbg.wait_for_break(COLUMN_BREAK, timeout=args.per_hit_timeout)

            regs = read_gpr(dbg)
            sp, x = regs["sp"], regs["s5"]
            if x & 0x80000000:
                x -= 1 << 32
            disp0, disp1, raw0, raw1, energy0, energy1 = dbg.read_f32s(sp, 6)
            phase = dbg.read_u32(PHASE)
            pulse = dbg.read_f32(ENVELOPE + phase * 4)
            screen_type = dbg.read_u32(SCREEN_TYPE)
            finished = dbg.read_u8(FINISHED)

            rows.append(
                dict(
                    hit=i,
                    x=x,
                    phase=phase,
                    pulse=pulse,
                    screen_type=screen_type,
                    finished=finished,
                    disp0=disp0,
                    disp1=disp1,
                    raw0=raw0,
                    raw1=raw1,
                    energy0=energy0,
                    energy1=energy1,
                )
            )
    except TimeoutError as e:
        print(f"stopped early after {len(rows)} hits: {e}")

    try:
        dbg.brk()
        clear_breakpoints(dbg)
        dbg.resume()
        dbg.close()
    except Exception as e:
        # The loading thread tearing itself down can take the whole
        # PPSSPPHeadless process with it (--timeout has nothing left to
        # wait for), which closes the socket out from under this cleanup.
        # The rows already captured are what matters; don't lose them over
        # a cleanup call to a process that is already gone.
        print(f"cleanup after capture failed, likely because the emulator process exited: {e}")

    if not rows:
        raise SystemExit("captured zero rows")

    with open(args.out, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        writer.writeheader()
        writer.writerows(rows)
    print(f"wrote {len(rows)} rows to {args.out}")

    reset_checked = 0
    reset_ok = 0
    recurrence_checked = 0
    recurrence_ok = 0
    max_diff = 0.0
    energy_drift_ok = 0
    energy_drift_checked = 0

    for prev, cur in zip(rows, rows[1:]):
        if cur["x"] != prev["x"] + 2:
            # Column index did not continue: a fresh Loading_DrawWave call
            # started and the array was reset to zero at its top.
            reset_checked += 1
            if (
                cur["disp0"] == 0.0
                and cur["disp1"] == 0.0
                and cur["raw0"] == 0.0
                and cur["raw1"] == 0.0
            ):
                reset_ok += 1
            continue

        predicted = predict(prev, cur["x"], cur["pulse"], rates)
        recurrence_checked += 1
        diffs = [abs(predicted[k] - cur[k]) for k in ("raw0", "raw1", "disp0", "disp1")]
        max_diff = max(max_diff, *diffs)
        if all(d < 1e-4 for d in diffs):
            recurrence_ok += 1

        for i in (0, 1):
            energy_drift_checked += 1
            drift = cur[f"energy{i}"] / 0.985 - prev[f"energy{i}"]
            if abs(drift) <= 7.5 + 1e-4:
                energy_drift_ok += 1

    print(f"recurrence (raw/disp predicted from previous column): {recurrence_ok}/{recurrence_checked} exact, max diff {max_diff:.6g}")
    print(f"energy impulse within +-7.5 of its damped carry-over: {energy_drift_ok}/{energy_drift_checked}")
    print(f"array reset to zero at a call boundary: {reset_ok}/{reset_checked}")


if __name__ == "__main__":
    main()
