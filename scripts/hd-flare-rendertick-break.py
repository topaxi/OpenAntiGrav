#!/usr/bin/env python3
"""Breakpoint-trace `EngineFlare_RenderTick` to settle whether it ever
reaches its own fade-store while a flare is visibly drawn.

`scripts/hd-flare-tuning-dump.py` (fourth session, see
`docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`) confirmed the
formula's two tuning constants live, but the fade's own output at
`this+0x18c` read `0.0` on every sample despite both static gates
appearing open. This script sets `Z0` breakpoints at two points inside
`EngineFlare_RenderTick` (`0x002a08a8`-`0x002a14ff`) - the shared early-out
landing (`0x002a1110`, reached by both static gates and possibly others this
session did not trace) and just past the `this+0x18c` store (`0x002a0d78`,
right after the `stfs` and before its own `ble`) - and tallies which one
each hit lands at over a stretch of a driven, thrusting race. **Measured
2026-09-02: 11 of 11 consecutive hits landed at the early-out, none at the
fade store** - `EngineFlare_RenderTick` runs (confirmed separately: a first
version of this script also armed the entry point, `0x002a08a8`, and it
fired on the very first 0.4s poll), but for the sampled craft it reliably
takes the skip branch rather than reaching the fade computation this
session's static reading traced. That is consistent with, and now explains,
the live-memory finding above - it does not yet say whether this is simply
what this craft's situation calls for (out of fade range, wrong LOD tier,
whatever the gate actually tests) or whether the visible sprite draws
through a branch this function never reaches for it at all.

**Needs `PPU Decoder: Interpreter (static)` in `config.yml`** - `Z0`
breakpoints are silently dead under the default `Recompiler (LLVM)`
([rpcs3-debugger.md](../docs/reverse-engineering/rpcs3-debugger.md#z0-breakpoints-fire-but-only-under-the-interpreter)).
This is a shared, session-scoped config edit, not a permanent one - switch
it back after this script exits, the same rule that page documents for
`Assume External Debugger`.

Uses `wait_at`'s own pattern generalised to several addresses at once
(resume briefly, pause, check every thread's PC against the whole set) since
the stub does not reliably announce a breakpoint hit on its own, plus
`step_off_breakpoint` to make repeated hits actually work - **a trap this
script found and `rpcs3-debugger.md` does not yet carry**: resuming a
thread parked exactly on an armed breakpoint does not step over it first,
so the thread never visibly moves and the *next* unrelated command hangs for
a full socket timeout, indistinguishable from a dead emulator. It still
happens eventually even with the fix (measured after 11 clean hits in one
run, cause not settled) - the script saves whatever it caught rather than
losing a whole run to it; rerun to pick up more samples from a fresh boot.

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

ENTRY = 0x002A08A8
GATE_LANDING = 0x002A1110       # both static gates branch here
PAST_FADE_STORE = 0x002A0D78    # right after `stfs f13, 0x18c(r31)`
# ENTRY is deliberately not armed: a first run confirmed it fires on the very
# first 0.4s poll (this function runs at least once a frame), and arming it
# alongside the other two just spends every attempt re-catching the same
# already-answered question instead of the interesting one.
TARGETS = {GATE_LANDING: "gate_landing", PAST_FADE_STORE: "past_fade_store"}


def wait_at_any(dbg, targets, tries, slice_seconds):
    """Like `Debugger.wait_at`, generalised to a set of addresses.

    Returns `(tid, address, registers)` for the first thread caught at any
    of `targets`, or `(None, None, None)` if none hit within `tries`.
    """
    for _ in range(tries):
        dbg.resume()
        time.sleep(slice_seconds)
        dbg.pause()
        dbg.drain()
        for tid in dbg.threads():
            regs = dbg.registers(tid)
            if regs is None:
                continue
            pc = int.from_bytes(regs[REG_PC:REG_PC + 8], "big")
            if pc in targets:
                return tid, pc, regs
    return None, None, None


def step_off_breakpoint(dbg, addr):
    """Un-stick a thread parked exactly at `addr`.

    **Undocumented trap, found running this script**: resuming a thread
    whose PC sits exactly on an armed `Z0` breakpoint does not step over it
    first - the thread never visibly moves, no further stop reply ever
    arrives, and the next unrelated command hangs for a full socket timeout
    (looks exactly like a dead emulator, and cost two crashed runs here).
    The standard breakpoint-stepping fix: remove the breakpoint, resume just
    long enough to clear the address, pause, drain, then re-arm.
    """
    dbg.remove_breakpoint(addr)
    dbg.resume()
    time.sleep(0.05)
    dbg.pause()
    dbg.drain()
    dbg.add_breakpoint(addr)


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

            deadline = time.time() + 45.0
            while time.time() < deadline and len(details) < 20:
                try:
                    tid, addr, regs = wait_at_any(dbg, TARGETS, tries=1, slice_seconds=0.15)
                except (TimeoutError, OSError) as exc:
                    # A rare but real stub desync outlives `step_off_breakpoint`
                    # itself - measured after 11 clean hits in one run. Save
                    # what was already caught rather than lose it; a fresh
                    # emulator relaunch (rerun this script) picks up from here.
                    print("stub desync after %d hits: %r" % (len(details), exc),
                          file=sys.stderr, flush=True)
                    stub_desync = True
                    break
                if addr is None:
                    continue
                name = TARGETS[addr]
                hits[name] += 1
                entry = {"tid": tid, "addr": "%#x" % addr, "which": name}
                if name == "past_fade_store":
                    # f13 (the value just stored to this+0x18c) as the
                    # register dump's own byte offset: REG_FPR + 13*8.
                    f13_bytes = regs[REG_FPR + 13 * 8: REG_FPR + 14 * 8]
                    entry["f13"] = struct.unpack(">d", f13_bytes)[0]
                details.append(entry)
                print(entry, flush=True)
                if not stub_desync:
                    try:
                        step_off_breakpoint(dbg, addr)
                    except (TimeoutError, OSError) as exc:
                        print("stub desync stepping off %#x after %d hits: %r" %
                              (addr, len(details), exc), file=sys.stderr, flush=True)
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
