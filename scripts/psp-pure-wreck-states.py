#!/usr/bin/env python3
"""Wreck a Pure (PSP, USA) craft with Ship_SetState(entity, 4) on a running PPSSPP and log the state chain.

    source data/scratch/<lane>/pp/env.sh      # DISPLAY, OAG_PPSSPP_PORT (scripts/ppsspp-start.sh)
    uv run --with websocket-client python3 scripts/psp-pure-wreck-states.py \\
        --port $OAG_PPSSPP_PORT --display $DISPLAY --out data/scratch/<lane>/run [--shots 10,40,60]

Start from a loaded Pure Time Trial state (`data/saves/pure-psp/`). The stop is the per-state update `0x08926ae4`
(`a0` the entity). Logs per 30 Hz stop: the state word `+0xfc`, the timer `+0x9a8`, which model is live
(`+0x9ec` against `+0x9f0`/`+0x9f4`), the destroyed byte `+0xb95`, the live model's first matrix row length
(`+0x80`) and flag word (`+0x2c`); photographs at the given stops since the call.

`Ship_SetState` loads its timer constants through `f12`, which is the update's `dt`: the injection restores
`f12` as well as the integer registers, or the first update after the call runs with `dt` = the state's timer.
Docs: `docs/ghidra/functions/psp-pure-usa/ship-destroy.md`, "Measured on PPSSPP".
"""
import argparse, json, math, struct, sys, importlib.util
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS))
from ppsspp_debugger import Debugger  # noqa: E402

spec = importlib.util.spec_from_file_location("pair", SCRIPTS / "psp-weapon-pair.py")
pair = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pair)

UPDATE = 0x08926AE4
SET_STATE = 0x08928824


def regs(dbg):
    c = dbg.call("cpu.getAllRegs")["categories"][0]
    return dict(zip(c["registerNames"], c["uintValues"]))


def inject(dbg, craft, state):
    saved = regs(dbg)
    fpu = dbg.call("cpu.getAllRegs")["categories"][1]
    f12 = dict(zip(fpu["registerNames"], fpu["uintValues"]))["f12"]
    for name, value in (("a0", craft), ("a1", state), ("ra", UPDATE), ("pc", SET_STATE)):
        dbg.call("cpu.setReg", name=name, value=value)
    dbg.call("cpu.status")
    dbg.pending = []
    dbg.call("cpu.resume")
    dbg.wait_for_break(UPDATE, timeout=30.0)
    for name, value in saved.items():
        if name != "zero":
            dbg.call("cpu.setReg", name=name, value=value)
    dbg.call("cpu.setReg", category=1, register=12, value=f12)
    fpu = dbg.call("cpu.getAllRegs")["categories"][1]
    print("f12 restored", hex(f12), hex(dict(zip(fpu["registerNames"], fpu["uintValues"]))["f12"]), file=sys.stderr)


def f32(dbg, a):
    return struct.unpack("<f", dbg.read(a, 4))[0]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, required=True)
    ap.add_argument("--display", required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--inject-frame", type=int, default=5)
    ap.add_argument("--frames", type=int, default=110)
    ap.add_argument("--shots", default="10,40,60,75,85")
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    shots = {int(v) for v in args.shots.split(",")}
    dbg = Debugger(args.port)
    log = []
    try:
        pair.WINDOW_X, pair.WINDOW_Y = 160, 128
        craft, frame, injected = None, 0, None
        for _hit in dbg.each_hit(UPDATE, 100000, timeout=60.0):
            a0 = regs(dbg)["a0"]
            if craft is None:
                craft = a0
                if dbg.read_u32(craft + 0xFC) > 8:
                    sys.exit("a0 is not an entity")
            if a0 != craft:
                continue
            if injected is None and frame >= args.inject_frame:
                inject(dbg, craft, 4)
                injected = frame
            live, hull, wreck = (dbg.read_u32(craft + o) for o in (0x9EC, 0x9F0, 0x9F4))
            row = {"frame": frame, "state": dbg.read_u32(craft + 0xFC), "timer": round(f32(dbg, craft + 0x9A8), 4),
                   "live": "wreck" if live == wreck and wreck else ("hull" if live == hull else hex(live)),
                   "destroyed_b95": dbg.read(craft + 0xB95, 1)[0]}
            if pair.ram(live):
                r = struct.unpack("<3f", dbg.read(live + 0x80, 12))
                row["live_row0_len"] = round(math.sqrt(sum(x * x for x in r)), 6)
                row["live_flags"] = hex(dbg.read_u32(live + 0x2C))
            if injected is not None:
                row["since"] = frame - injected
                if row["since"] in shots:
                    name = "s%03d_state%d.png" % (row["since"], row["state"])
                    row["shot"] = name if pair.shoot(args.display, args.out / name) else None
                if row["since"] >= args.frames:
                    log.append(row)
                    break
            log.append(row)
            frame += 1
        dbg.resume()
    finally:
        (args.out / "states.json").write_text(json.dumps(log, indent=1))
        dbg.close()


if __name__ == "__main__":
    main()
