#!/usr/bin/env python3
"""Breakpoint-trace `EngineFlare_RenderTick`'s fade math with a ladder of
checkpoints, not just its two endpoints.

**This is the corrected second version.** The first tried to distinguish
"a gate closed" from "the fade ran and evaluated to `<= 0`" using only two
breakpoints, one of which (`0x002a1110`) turned out to be the function's
*shared epilogue* - reached by every returning call, successful or not, not
a gate landing at all. "12 of 12 hits there" in that version's run was
never evidence of anything; see `docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`
("Fifth session") for the full account of the mistake and the review that
caught it. This version arms three points *inside* the fade math instead,
so their relative hit counts can actually tell the three cases apart:

- `0x002a0bb8` (`FADE_START`) - the camera-relative length computation
  begins. A hit here at all confirms the fade math runs.
- `0x002a0d70` (`FADE_STORE`) - the `stfs f13,0x18c(r31)` instruction
  itself, right before its own `ble`. Fires whatever value was just
  computed, positive or not - reaching this but not the next checkpoint
  means the fade ran and evaluated to `<= 0`.
- `0x002a0d78` (`FADE_POSITIVE`) - only reached when that value is `> 0`.
  A hit here is the fade producing something a draw call could use.

**Needs `PPU Decoder: Interpreter (static)` in `config.yml`** - `Z0`
breakpoints are silently dead under the default `Recompiler (LLVM)`
([rpcs3-debugger.md](../docs/reverse-engineering/rpcs3-debugger.md#z0-breakpoints-fire-but-only-under-the-interpreter)).
This is a shared, session-scoped config edit, not a permanent one - switch
it back after this script exits, the same rule that page documents for
`Assume External Debugger`.

Two GDB-stub traps from the first version carry over, both worked around:
resuming a thread parked exactly on an armed breakpoint does not step over
it first, so the thread never visibly moves and the next unrelated command
hangs for a full socket timeout (`step_off_breakpoint` below); and a hit on
a *different* armed breakpoint can land during that step-off window - the
first version's `step_off_breakpoint` did not check for this and could
silently drop a hit, which is why its "zero `past_fade_store` hits" result
was never fully trusted. This version checks and carries any such hit
forward into the main loop rather than discarding it.

    uv run --with evdev python3 scripts/hd-flare-rendertick-break.py [out_dir]
"""

import importlib.util
import json
import struct
import sys
import time
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
spec = importlib.util.spec_from_file_location(
    "rpcs3_drive", ROOT / "scripts" / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)
from rpcs3_debugger import Debugger, REG_FPR, REG_PC  # noqa: E402

IMAGE = ROOT / "data/images/hdfury-ps3-eu-dec.iso"

FADE_START = 0x002A0BB8
FADE_STORE = 0x002A0D70
FADE_POSITIVE = 0x002A0D78
TARGETS = {FADE_START: "fade_start", FADE_STORE: "fade_store", FADE_POSITIVE: "fade_positive"}


def read_pc_hit(dbg, targets):
    """`(tid, address, registers)` for any thread currently stopped at one
    of `targets`, or `(None, None, None)`. Assumes the target is already
    paused - does not resume/sleep/pause itself."""
    for tid in dbg.threads():
        regs = dbg.registers(tid)
        if regs is None:
            continue
        pc = int.from_bytes(regs[REG_PC:REG_PC + 8], "big")
        if pc in targets:
            return tid, pc, regs
    return None, None, None


def wait_at_any(dbg, targets, tries, slice_seconds):
    """Like `Debugger.wait_at`, generalised to a set of addresses."""
    for _ in range(tries):
        dbg.resume()
        time.sleep(slice_seconds)
        dbg.pause()
        dbg.drain()
        hit = read_pc_hit(dbg, targets)
        if hit[0] is not None:
            return hit
    return None, None, None


def step_off_breakpoint(dbg, hit_addr, targets):
    """Un-stick a thread parked exactly at `hit_addr`, carrying forward any
    *other* armed breakpoint's hit that lands during the step rather than
    discarding it (see the module docstring - the first version's version
    of this function did not check and its hit counts were never fully
    trusted as a result). Returns a hit tuple or `(None, None, None)`.
    """
    dbg.remove_breakpoint(hit_addr)
    dbg.resume()
    time.sleep(0.05)
    dbg.pause()
    dbg.drain()
    hit = read_pc_hit(dbg, {a: n for a, n in targets.items() if a != hit_addr})
    dbg.add_breakpoint(hit_addr)
    return hit


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "/tmp/hd-flare-rendertick-break")
    out.mkdir(parents=True, exist_ok=True)
    with drive.Session(str(IMAGE), str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        if not session.wait_for_screen("Main Menu", 220.0):
            print("never reached the Main Menu", file=sys.stderr)
            return 1
        time.sleep(12.0)
        if session.walk_to_race() not in drive.RACE_ARRIVED:
            print("did not reach a race", file=sys.stderr)
            return 1
        print("in race; waiting for the load", flush=True)
        time.sleep(50.0)
        session.pad.set("cross", True)
        time.sleep(16.0)

        dbg = Debugger()
        hits = Counter()
        details = []
        stub_desync = False
        try:
            dbg.pause()
            for addr in TARGETS:
                dbg.add_breakpoint(addr)
            print("breakpoints armed: %s" % [hex(a) for a in TARGETS], flush=True)

            def record(tid, addr, regs):
                name = TARGETS[addr]
                hits[name] += 1
                entry = {"tid": tid, "addr": "%#x" % addr, "which": name}
                if name in ("fade_store", "fade_positive"):
                    # f13 (the value just computed, whatever it turned out to
                    # be) as the register dump's own byte offset: REG_FPR + 13*8.
                    f13_bytes = regs[REG_FPR + 13 * 8: REG_FPR + 14 * 8]
                    entry["f13"] = struct.unpack(">d", f13_bytes)[0]
                details.append(entry)
                print(entry, flush=True)

            deadline = time.time() + 45.0
            pending = None
            while time.time() < deadline and len(details) < 30:
                try:
                    if pending is not None:
                        tid, addr, regs = pending
                        pending = None
                    else:
                        tid, addr, regs = wait_at_any(dbg, TARGETS, tries=1, slice_seconds=0.15)
                    if addr is None:
                        continue
                    record(tid, addr, regs)
                    pending_hit = step_off_breakpoint(dbg, addr, TARGETS)
                    if pending_hit[0] is not None:
                        pending = pending_hit
                except (TimeoutError, OSError) as exc:
                    # A rare but real stub desync that outlives
                    # `step_off_breakpoint` itself - not fully characterised,
                    # see the module docstring. Save whatever was already
                    # caught rather than lose it; rerun to pick up more.
                    print("stub desync after %d hits: %r" % (len(details), exc),
                          file=sys.stderr, flush=True)
                    stub_desync = True
                    break
            if not stub_desync:
                for addr in TARGETS:
                    dbg.remove_breakpoint(addr)
                dbg.resume()
        finally:
            try:
                dbg.resume()
            except Exception:
                pass
            dbg.close()
        result = {"hits": dict(hits), "details": details, "stub_desync": stub_desync}
        (out / "meta.json").write_text(json.dumps(result, indent=1))
        if not stub_desync:
            session.pad.set("cross", False)
            drive.screenshot(out / "race.png")
        print(json.dumps(result, indent=1))
        print("done; artefacts in %s" % out, flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
