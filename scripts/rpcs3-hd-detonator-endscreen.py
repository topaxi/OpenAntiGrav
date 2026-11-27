#!/usr/bin/env python3
"""Film a Detonator run on RPCS3 from START RACE through the craft's destruction to the end screens.

    source data/scratch/<lane>/env.sh
    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/emu-run.py --child-status <log-dir>/status.json -- \\
        python3 scripts/rpcs3-hd-detonator-endscreen.py --out data/scratch/<lane>/end --kill-clock 20

Attached to a `serve` restored from a Detonator Racebox grid state, this taps START RACE until the lap
clock runs, optionally writes the shield (`ship+0x5fa0`) down at `--kill-clock` so the next mine destroys
the craft (the end screen's filler has no branch on how the run ended), then grabs the display (through
`grab_with_clock`) every `--period` wall seconds. `--fire-period` taps the cannon so the run scores. Each grab is logged to `OUT/film.json` with the TTY
screen, the lap clock, the shield, the out-of-race bit (`ship+0x6a64 & 0x1000`) and the results record
`RaceManager + 0x1980` (`docs/ghidra/functions/ps3-hdfury-eu/detonator-endrace.md`). Memory is read
without pausing. Screen changes and the out-of-race bit are logged to `OUT/events.json` with their wall
second. `--tail` seconds after the first screen that is not the race, it stops.
"""
import argparse
import importlib.util
import json
import struct
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
_spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)
import compare_env  # noqa: E402
import rpcs3_hd_field  # noqa: E402
import rpcs3_place as place  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402

SHIELD = 0x5FA0
OUT_FLAGS = 0x6A64
LAP_CLOCK = 0x7014
RACE_MANAGER = 0x008A6814
RECORD = 0x1980


def record(mem, rm):
    """The results record the end screen draws from, `rec = rm + 0x1980`."""
    blob = mem.rd(rm + RECORD + 0x1314, 0x48)
    at = lambda off, fmt: struct.unpack_from(">" + fmt, blob, off - 0x1314)[0]
    return {"score": at(0x1314, "i"), "hud_stage": at(0x1318, "H"), "mines_percent": at(0x131c, "f"),
            "bombs": at(0x1320, "H"), "chain5": at(0x1328, "H"), "chain10": at(0x132a, "H"),
            "chain15": at(0x132c, "H"), "laps": at(0x1330, "i"), "new_record": blob[0x1357 - 0x1314],
            "record_value": at(0x1358, "I"), "raw": blob.hex()}


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--image", default="/home/topaxi/projects/OpenAntiGrav/data/images/hdfury-ps3-eu-dec.iso")
    ap.add_argument("--kill-clock", type=float, help="lap-clock second at which the shield is written down to --kill-shield")
    ap.add_argument("--kill-shield", type=float, default=0.5)
    ap.add_argument("--period", type=float, default=0.6, help="wall seconds between grabs once near the end")
    ap.add_argument("--tail", type=float, default=40.0, help="wall seconds kept after the craft is out of the race")
    ap.add_argument("--limit", type=float, default=900.0, help="wall seconds overall")
    ap.add_argument("--pre-wait", type=float, default=30.0)
    ap.add_argument("--fire-period", type=float, help="hold TRIANGLE (the pulse cannon) for 0.4 s every this many wall seconds")
    args = ap.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    pair = importlib.util.spec_from_file_location("rpcs3_hd_pair", HERE / "rpcs3-hd-pair.py")
    pair_mod = importlib.util.module_from_spec(pair)
    pair.loader.exec_module(pair_mod)
    window = compare_env.platform("hd-fury")["output"]
    pair_mod.set_window(*window)
    crop = "%dx%d+0+0" % tuple(window)
    with drive.Session(args.image, str(out / "logs")) as session:
        gdb = Debugger(port=port)
        time.sleep(args.pre_wait)
        ship = None
        for _ in range(60):
            gdb.pause()
            try:
                ship, _body = place.find_player(gdb)
                clock = struct.unpack(">f", gdb.read(ship + LAP_CLOCK, 4))[0]
            except RuntimeError:
                clock = 0.0
            gdb.resume()
            if clock > 0.5:
                break
            session.tap("cross", settle=6)
        else:
            sys.exit("the race never started")
        mem = rpcs3_hd_field.Mem(session.open_mem())
        rm = mem.u32(mem.u32(RACE_MANAGER))
        print("ship %#x rm %#x" % (ship, rm), flush=True)
        film = []
        killed = False
        out_since = None
        started = time.time()
        index = 0
        last_grab = 0.0
        last_screen = None
        events = []
        seen = (None, None)
        held = None
        next_fire = 0.0
        while time.time() - started < args.limit:
            clock = mem.f32(ship + LAP_CLOCK)
            shield = mem.f32(ship + SHIELD)
            flags = mem.u32(ship + OUT_FLAGS)
            screen = drive.current_screen()
            if args.kill_clock is not None and not killed and clock >= args.kill_clock:
                mem.wr(ship + SHIELD, struct.pack(">f", args.kill_shield))
                killed = True
                print("shield written %.1f -> %.1f at lap clock %.2f" % (shield, args.kill_shield, clock), flush=True)
            now_seen = (screen, bool(flags & 0x1000))
            if now_seen != seen:
                events.append({"wall_s": round(time.time() - started, 2), "screen": screen, "out": now_seen[1],
                               "lap_clock": clock, "shield": shield})
                (out / "events.json").write_text(json.dumps(events, indent=1) + "\n")
                print("event %s" % events[-1], flush=True)
                seen = now_seen
            if args.fire_period and not now_seen[1] and screen in ("?", "HUD", "InGame"):
                if held is None and time.time() >= next_fire:
                    session.pad.set("triangle", True)
                    held = time.time()
                elif held is not None and time.time() - held >= 0.4:
                    session.pad.set("triangle", False)
                    held, next_fire = None, time.time() + args.fire_period
            elif held is not None:
                session.pad.set("triangle", False)
                held = None
            gone = bool(flags & 0x1000) or screen not in ("?", "HUD", "InGame")
            if gone and out_since is None:
                out_since = time.time()
                print("out of race at lap clock %.2f, shield %.1f, screen %s" % (clock, shield, screen), flush=True)
            near = out_since is not None or shield < 40.0 or screen != last_screen
            now = time.time()
            if now - last_grab >= (args.period if near else 6.0):
                name = "f%04d.png" % index
                drive.grab_with_clock(out / name, crop=crop)
                row = {"frame": name, "wall_s": round(now - started, 2), "screen": screen, "lap_clock": clock,
                       "shield": shield, "flags": flags, "out": bool(flags & 0x1000), "record": record(mem, rm)}
                film.append(row)
                (out / "film.json").write_text(json.dumps(film, indent=1) + "\n")
                print("%s wall %.1f screen %r clock %.2f shield %.1f out %d score %d" % (
                    name, row["wall_s"], screen, clock, shield, row["out"], row["record"]["score"]), flush=True)
                index += 1
                last_grab = now
                last_screen = screen
            if out_since is not None and time.time() - out_since > args.tail:
                break
            time.sleep(0.1)
        print("done: %d frames" % index, flush=True)


if __name__ == "__main__":
    main()
