#!/usr/bin/env python3
"""Log what Wipeout HD's hover escape gate reads, under player input (RPCS3).

`Craft_HoverFourPoint` (`0x000ede88`) branches to `0x000ee628` for every probe whose height
along up is under `1.0`. There it loads the hit record's `+0x28` and compares it with `1`,
tests the magnet lock `craft+0x2c0 == 1.0`, then the hit object's class (`+0x74`, through
`*(craft+0x35c)` and the record's `+0x20`) against `2`, and only then moves the body
(`0x000ee6ac`). This parks a breakpoint on `0x000ee628` and, at each stop, reads:

- `+0x28` and `+0x20` of the record (`r9`), the height (`f13`), the magnet lock;
- the hit object's class `+0x74` and shape type `+0xc`, resolved as the loop does.

With `--contacts` it parks on `0x000ee294` instead, the contact test's own class compare,
and tallies the class (`r0`) of every probe hit the loop reaches.

Player input only: cross is held and the stick swings left and right every `--swing`
seconds, so the craft meets walls; the Autopilot is never armed. Attach to a running
`rpcs3-drive.py serve` (`OAG_RPCS3_ATTACH=1`) sitting in a race after GO, for example a
restored Time Trial state, served with `--interpreter` (the only decoder where a `Z0`
breakpoint fires). Writes `<out>/escape-gate.json`.
"""
import argparse
import importlib.util
import json
import struct
import sys
import time
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parent
sys.path.insert(0, str(REPO))
_spec = importlib.util.spec_from_file_location("rpcs3_drive", REPO / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)
from rpcs3_debugger import REG_FPR, REG_GPR, REG_PC, Debugger  # noqa: E402

ESCAPE_GATE = 0x000EE628
CONTACT_CLASS = 0x000EE294


def gpr(regs, i):
    return int.from_bytes(regs[REG_GPR + i * 8:REG_GPR + i * 8 + 8], "big") & 0xFFFFFFFF


def fpr(regs, i):
    return struct.unpack_from(">d", regs, REG_FPR + i * 8)[0]


def u32(dbg, addr):
    return struct.unpack(">I", dbg.read(addr, 4))[0]


def f32(dbg, addr):
    return struct.unpack(">f", dbg.read(addr, 4))[0]


def hit_object(dbg, craft, triangle):
    """The object the loop resolves for a hit: `*(*(craft+0x35c) + (id << 3) + 0xf760 + 4)`."""
    table = u32(dbg, craft + 0x35C)
    off = ((triangle << 3) & 0xFFFFFFF8) + 0x10000 - 0x8A0
    if off & 0x80000000:
        off -= 1 << 32
    return u32(dbg, (table + off + 4) & 0xFFFFFFFF)


def next_hit(dbg, address, timeout):
    deadline = time.time() + timeout
    while time.time() < deadline:
        dbg.resume()
        if dbg.wait_for_stop(min(5.0, max(deadline - time.time(), 0.5))) is None:
            dbg.pause()
        dbg.drain(0.1)
        for tid in dbg.threads():
            regs = dbg.registers(tid)
            if regs is not None and int.from_bytes(regs[REG_PC:REG_PC + 8], "big") == address:
                return tid, regs
    return None, None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--samples", type=int, default=150)
    ap.add_argument("--seconds", type=float, default=300.0)
    ap.add_argument("--swing", type=float, default=2.0)
    ap.add_argument("--contacts", action="store_true")
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    address = CONTACT_CLASS if args.contacts else ESCAPE_GATE
    log = {"address": hex(address), "samples": []}
    tally = Counter()
    with drive.Session(args.image, str(out / "logs"), interpreter=True) as session:
        if session.guard is not None:
            session.guard.stage("capture:escape-gate", limit=args.seconds + 120)
        session.pad.set("cross", True)
        dbg = Debugger(port=port)
        dbg.pause()
        dbg.add_breakpoint(address)
        t0 = time.time()
        side = 1.0
        last_swing = t0
        try:
            while len(log["samples"]) < args.samples and time.time() - t0 < args.seconds:
                if time.time() - last_swing > args.swing:
                    side = -side
                    session.pad.stick(side, 0.0)
                    last_swing = time.time()
                tid, regs = next_hit(dbg, address, 20.0)
                if tid is None:
                    continue
                craft = gpr(regs, 31)
                if args.contacts:
                    # `r6` is the hit record at the contact test (`0x000ee270`).
                    record = gpr(regs, 6)
                    obj = hit_object(dbg, craft, u32(dbg, record + 0x20))
                    sample = {
                        "t": round(time.time() - t0, 2),
                        "class": gpr(regs, 0),
                        "tag_28": u32(dbg, record + 0x28),
                        "shape_c": u32(dbg, obj + 0xC),
                    }
                    tally[(sample["tag_28"], sample["class"])] += 1
                else:
                    record = gpr(regs, 9)
                    triangle = u32(dbg, record + 0x20)
                    obj = hit_object(dbg, craft, triangle)
                    sample = {
                        "t": round(time.time() - t0, 2),
                        "craft": hex(craft),
                        "tag_28": u32(dbg, record + 0x28),
                        "id_20": triangle,
                        "h": fpr(regs, 13),
                        "maglock": f32(dbg, craft + 0x2C0),
                        "class_74": u32(dbg, obj + 0x74),
                        "shape_c": u32(dbg, obj + 0xC),
                    }
                    tally[(sample["tag_28"], sample["class_74"])] += 1
                log["samples"].append(sample)
                print(json.dumps(sample), flush=True)
                dbg.remove_breakpoint(address)
                dbg.step(tid)
                dbg.pause()
                dbg.add_breakpoint(address)
            dbg.remove_breakpoint(address)
        finally:
            log["tally"] = {str(k): v for k, v in tally.items()}
            (out / "escape-gate.json").write_text(json.dumps(log, indent=1))
            session.pad.set("cross", False)
            session.pad.stick(0.0, 0.0)
            try:
                dbg.resume()
            except Exception:
                pass
            dbg.close()
    print("tally", dict(tally))


if __name__ == "__main__":
    main()
