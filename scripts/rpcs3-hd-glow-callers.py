#!/usr/bin/env python3
"""Find what fires HD's boost pulse: break on `EngineFlare_TriggerZoomGlow` (`0x0029ef40`) and read who called it.

    scripts/emu-restore-state.sh <lane> data/saves/hd-fury/grid-flyover-talons.SAVESTAT.zst --interpreter
    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-glow-callers.py --seconds 90 --start-race

`Z0` breakpoints fire only under the PPU interpreter, so the state is restored with `--interpreter` (slower
than the recompiler). Each hit prints the link register, the caller's frame and the first argument registers,
then the breakpoint is lifted for a second so the pulse's own updates run. With `--start-race` the script taps
START RACE and holds throttle, which on Talon's Junction crosses the start-line speed pad.
"""
import argparse
import importlib.util
import struct
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)
from rpcs3_debugger import REG_CTR, REG_LR, Debugger  # noqa: E402

TRIGGER = 0x0029EF40


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seconds", type=float, default=60.0)
    ap.add_argument("--start-race", action="store_true")
    args = ap.parse_args()
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    session = drive.read_session()
    pad = drive.RemotePad(session["socket"])
    with Debugger(port=port) as gdb:
        gdb.add_breakpoint(TRIGGER)
        gdb.resume()
        if args.start_race:
            for _ in range(3):
                pad.set("cross", True)
                time.sleep(0.2)
                pad.set("cross", False)
                time.sleep(1.5)
            pad.set("cross", True)
        t0 = time.time()
        while time.time() - t0 < args.seconds:
            tid, regs = gdb.wait_at(TRIGGER, tries=1, slice_seconds=0.4)
            if tid is None:
                continue
            gprs = struct.unpack(">32Q", regs[:256])
            lr = int.from_bytes(regs[REG_LR:REG_LR + 8], "big")
            ctr = int.from_bytes(regs[REG_CTR:REG_CTR + 8], "big")
            print("t=%.1f hit tid=%s LR=%#x CTR=%#x r1=%#x r3=%#x r4=%#x" % (
                time.time() - t0, tid, lr, ctr, gprs[1], gprs[3], gprs[4]), flush=True)
            stack = gdb.read(gprs[1] & 0xFFFFFFFF, 0x80)
            print("  stack words:", [hex(x) for x in struct.unpack(">32I", stack)], flush=True)
            gdb.remove_breakpoint(TRIGGER)
            gdb.resume()
            time.sleep(1.2)
            gdb.pause()
            gdb.add_breakpoint(TRIGGER)
        pad.set("cross", False)
        gdb.remove_breakpoint(TRIGGER)
        gdb.resume()


if __name__ == "__main__":
    main()
