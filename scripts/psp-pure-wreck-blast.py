#!/usr/bin/env python3
"""Wreck a Pure (PSP, USA) craft with Ship_SetState(entity, 5) on a running PPSSPP and capture the big blast.

    source data/scratch/<lane>/pp/env.sh      # DISPLAY, OAG_PPSSPP_PORT, PP_HOME (scripts/ppsspp-start.sh, absolute image path)
    uv run --with websocket-client python3 scripts/psp-pure-wreck-blast.py \\
        --port $OAG_PPSSPP_PORT --display $DISPLAY --out data/scratch/<lane>/run [--dump-after=-1,3,6]

Start from a loaded Pure Time Trial state with the pause menu closed and the emulator window at 960x544 at (160, 128).
`0x08926ae4` (the per-state update, `a0` the entity whose state word is `+0xfc`) is the stop; a0 of
`Ship_UpdateCraft` (`0x0892d214`) is the craft, NOT the entity (injecting there crashed PPSSPP). The call is made
the way `psp-wreck-capture.py` makes Pulse's. Pure's game loop runs at 30 Hz (the timer at `entity+0x9a8` drops 0.0333 per
stop), so the 1.5 s blast fires 45 stops after the injection. Writes one photograph per stop from the blast on and a GE dump
(`gpu.record.dump`, census with `psp-ge-dump.py`) one stop before the blast and at the offsets given. `--mode spawns`
logs `0x08871e1c` calls instead (never hit on this build: unresolved). Docs: `psp-pure-usa/wreck-big-blast.md`.
"""


import argparse, base64, json, struct, sys, time, importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))
from ppsspp_debugger import Debugger  # noqa: E402

spec = importlib.util.spec_from_file_location("pair", ROOT / "psp-weapon-pair.py")
pair = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pair)

UPDATE = 0x08926AE4
SET_STATE = 0x08928824
SPAWN = 0x08871E1C


def regs(dbg):
    c = dbg.call("cpu.getAllRegs")["categories"][0]
    return dict(zip(c["registerNames"], c["uintValues"]))


def inject(dbg, craft, state):
    saved = regs(dbg)
    for name, value in (("a0", craft), ("a1", state), ("ra", UPDATE), ("pc", SET_STATE)):
        dbg.call("cpu.setReg", name=name, value=value)
    dbg.call("cpu.status")
    dbg.pending = []
    dbg.call("cpu.resume")
    dbg.wait_for_break(UPDATE, timeout=30.0)
    for name, value in saved.items():
        if name != "zero":
            dbg.call("cpu.setReg", name=name, value=value)


def f32(dbg, a):
    return struct.unpack("<f", dbg.read(a, 4))[0]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, required=True)
    ap.add_argument("--display", required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--mode", choices=("frames", "spawns"), default="frames")
    ap.add_argument("--inject-frame", type=int, default=5)
    ap.add_argument("--dump-after", default="-1,3,6", help="GE dumps at these frames relative to the blast frame")
    ap.add_argument("--shot-from", type=int, default=-3)
    ap.add_argument("--shot-to", type=int, default=24)
    ap.add_argument("--timeout", type=float, default=60.0)
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    dumps_at = [int(v) for v in args.dump_after.split(",")]
    dbg = Debugger(args.port)
    log = []
    replies = []
    _recv = dbg._recv

    def spy():
        m = _recv()
        if m and m.get("event") == "gpu.record.dump" and "uri" in m:
            replies.append(m)
        return m

    dbg._recv = spy
    try:
        pair.WINDOW_X, pair.WINDOW_Y = 160, 128
        craft = None
        frame = 0
        injected = None
        blast = None
        tickets = {}
        def step_row(frame):
            return {"frame": frame, "state": dbg.read_u32(craft + 0xFC), "timer": f32(dbg, craft + 0x9A8)}
        for _hit in dbg.each_hit(UPDATE, 100000, timeout=args.timeout):
            if craft is None:
                craft = regs(dbg)["a0"]
                print("craft %#x state %d timer %r" % (craft, dbg.read_u32(craft + 0xFC), f32(dbg, craft + 0x9A8)), file=sys.stderr)
                if dbg.read_u32(craft + 0xFC) > 8:
                    sys.exit("a0 is not an entity (state word %#x)" % dbg.read_u32(craft + 0xFC))
            if regs(dbg)["a0"] != craft:
                continue
            if injected is None and frame >= args.inject_frame:
                inject(dbg, craft, 5)
                injected = frame
                print("injected at frame %d" % frame, file=sys.stderr)
                if args.mode == "spawns":
                    break
            row = step_row(frame)
            if injected is not None:
                row["since"] = frame - injected
                if blast is None and row["state"] == 6:
                    blast = frame
                    print("blast frame %d (since %d)" % (frame, frame - injected), file=sys.stderr)
                if blast is None and row["timer"] <= 0.05 and "pre_dump" not in tickets and -1 in dumps_at:
                    tickets["pre_dump"] = dbg.send("gpu.record.dump")
                    row["dump"] = "pre"
                if blast is not None:
                    n = frame - blast
                    if n in dumps_at:
                        tickets["dump%+d" % n] = dbg.send("gpu.record.dump")
                        row["dump"] = n
                    if args.shot_from <= n:
                        pass
                if blast is not None or row["timer"] < 0.2:
                    n = (frame - blast) if blast is not None else -99
                    name = "b%+03d_f%04d.png" % (n if n != -99 else -99, frame)
                    row["shot"] = name if pair.shoot(args.display, args.out / name) else None
                if blast is not None and frame - blast >= args.shot_to:
                    log.append(row)
                    break
            log.append(row)
            frame += 1
        if args.mode == "spawns":
            start = dbg.call("cpu.status")["ticks"]
            dbg.remove_breakpoint(UPDATE)
            for _ in dbg.each_hit(SPAWN, 100000, timeout=args.timeout):
                r = regs(dbg)
                now = (dbg.call("cpu.status")["ticks"] - start) / (222_000_000 / 59.94)
                name = dbg.read_cstring(r["a1"], 40) if pair.ram(r["a1"]) else None
                m = list(struct.unpack("<16f", dbg.read(r["a3"], 64))) if pair.ram(r["a3"]) else None
                log.append({"frame": round(now, 1), "name": name, "a0": hex(r["a0"]), "a2": hex(r["a2"]), "matrix": m})
                if now > 160:
                    break
        dbg.resume()
        end = time.time() + 180
        while len(replies) < len(tickets) and time.time() < end:
            dbg._recv()
        order = list(tickets)
        for label, m in zip(order, sorted(replies, key=lambda m: int(m["ticket"]))):
            _, b64 = m["uri"].split(",", 1)
            (args.out / (label + ".ppdmp")).write_bytes(base64.b64decode(b64))
            print("wrote", label, file=sys.stderr)
    finally:
        (args.out / (args.mode + ".json")).write_text(json.dumps(log, indent=1))
        dbg.close()


if __name__ == "__main__":
    main()
