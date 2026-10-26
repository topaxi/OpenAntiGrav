#!/usr/bin/env python3
"""What happens when a running Autopilot's timer runs out on HD (RPCS3), row by row.

Attaches to a `serve` restored from a Time Trial or Single Race state
(`scripts/emu-restore-state.sh`, `OAG_RPCS3_ATTACH=1`), arms the player's Autopilot the
way `Autopilot_Fire` does - the pickup slot `*(ship+0x5edc)`: bit `0x800` of `+0x200`, the
timer `+0x158` - through `/proc` memory, and samples one row per new body position:

- the slot's flags and timer (the timer's fall between rows is the frame's `dt`);
- the craft's engine flags `+0x264`, multiplier `+0x2f0`, the controls' blend `+0x278`,
  the control pointer `+0x84` (the pad record `*(craft+0x44)` or the driver's record at
  `craft+0x4c`) and the five control floats it points at;
- the body's position and right, up and forward rows, from which come the heading rate in
  degrees a second and the bank.

Policies for where an expiry falls (`--policy`):

- `natural`: a stint of `--stint` seconds, re-armed after a few rows. With a stint longer than
  the lap this is a scan of the circuit's heading rates, for choosing `--corner`.
- `corner`: armed for `--stint` seconds and, the first row the heading rate passes `--corner`
  degrees a second, the timer is written down to `--force` seconds. Every expiry falls in a
  corner, which is where a hold would have to act.
- `cue`: one stint, run to the release; the `disengaging` edge is read off the timer (the call
  sits in `Autopilot_Update` and is taken when the timer crosses 1.0 from above).

`--stick` holds the pad's left stick sideways, so the pad record has a steering value to be
handed back to and an ease between the driver's and the pad's would show as intermediate values.

Not a physics reference: the Autopilot flies (maintainer, 2026-10-20). Only its timer and its
hand-back are read. Output is derived game data: keep it under `data/`, never commit.

    scripts/emu-env.sh <lane> 98 47098 ; source data/scratch/<lane>/env.sh
    scripts/emu-restore-state.sh <lane> data/saves/hd-fury/<state>.SAVESTAT.zst
    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/emu-run.py -- \\
        python3 scripts/rpcs3-hd-autopilot-expiry.py --policy corner --out data/scratch/<lane>/hd.json

`docs/ghidra/functions/ps3-hdfury-eu/autopilot.md`, "Expiry mid-corner", has what it found.
"""
import argparse
import importlib.util
import json
import math
import struct
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
_spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)
import rpcs3_place as place  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402

GUEST = 0x300000000
SLOT = 0x5EDC
CRAFT = 0x5FAC
RUNNING = 0x800


class Mem:
    def __init__(self, raw):
        self.raw = raw

    def read(self, addr, n):
        self.raw.seek(GUEST + addr)
        return self.raw.read(n)

    def u32(self, addr):
        return struct.unpack(">I", self.read(addr, 4))[0]

    def f32(self, addr):
        return struct.unpack(">f", self.read(addr, 4))[0]

    def write(self, addr, data):
        self.raw.seek(GUEST + addr)
        self.raw.write(data)


def heading_rate_dps(prev, row):
    """Degrees a second between two forward rows in the horizontal plane."""
    dt = row["t_game"] - prev["t_game"]
    if dt <= 0:
        return 0.0
    d = math.degrees(math.atan2(row["fwd"][0], row["fwd"][2]) - math.atan2(prev["fwd"][0], prev["fwd"][2]))
    return ((d + 180.0) % 360.0 - 180.0) / dt


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--image", default="/home/topaxi/projects/OpenAntiGrav/data/images/hdfury-ps3-eu-dec.iso")
    ap.add_argument("--policy", choices=("natural", "corner", "cue"), default="corner")
    ap.add_argument("--stint", type=float, default=6.0)
    ap.add_argument("--force", type=float, default=0.15)
    ap.add_argument("--corner", type=float, default=40.0, help="degrees a second that count as a corner")
    ap.add_argument("--expiries", type=int, default=20)
    ap.add_argument("--seconds", type=float, default=500.0, help="wall-clock budget")
    ap.add_argument("--stick", type=float, default=0.0)
    ap.add_argument("--no-throttle", action="store_true")
    args = ap.parse_args()
    args.out.parent.mkdir(parents=True, exist_ok=True)

    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    log = {"args": {k: str(v) for k, v in vars(args).items()}, "rows": [], "events": []}
    with drive.Session(args.image, str(args.out.parent / "logs")) as session:
        gdb = Debugger(port=port)
        gdb.pause()
        ship, body = place.find_player(gdb)
        gdb.resume()
        m = Mem(session.open_mem())
        slot = m.u32(ship + SLOT)
        craft = m.u32(ship + CRAFT)
        log["events"].append({"ship": ship, "body": body, "craft": craft, "slot": slot,
                              "slot_flags": m.u32(slot + 0x200), "slot_timer": m.f32(slot + 0x158)})
        print("ship %#x body %#x craft %#x slot %#x" % (ship, body, craft, slot), flush=True)
        if not args.no_throttle:
            session.pad.set("cross", True)
        if args.stick:
            session.pad.stick(args.stick, 0.0)
        driver_record = craft + 0x4C

        def arm(timer):
            m.write(slot + 0x200, struct.pack(">I", m.u32(slot + 0x200) | RUNNING))
            m.write(slot + 0x158, struct.pack(">f", timer))

        def set_timer(timer):
            m.write(slot + 0x158, struct.pack(">f", timer))

        def sample(t0):
            pos = struct.unpack(">3f", m.read(body + 0x200, 12))
            blk = m.read(craft + 0x264, 0xE0)
            ctl_ptr = m.u32(craft + 0x84)
            ctl = struct.unpack(">5f", m.read(ctl_ptr, 20)) if 0x10000 < ctl_ptr < 0x40000000 else None
            rows = struct.unpack(">12f", m.read(body + 0x1D0, 48))
            return {
                "t_wall": time.time() - t0,
                "finished": m.read(ship + 0x7002, 1)[0],
                "state": m.u32(craft + 0x2F8),
                "pos": pos,
                "flags": m.u32(slot + 0x200),
                "timer": m.f32(slot + 0x158),
                "engine": struct.unpack_from(">I", blk, 0)[0],
                "mult": struct.unpack_from(">f", blk, 0x2F0 - 0x264)[0],
                "blend": struct.unpack_from(">f", blk, 0x278 - 0x264)[0],
                "speed": struct.unpack_from(">f", blk, 0x340 - 0x264)[0],
                "ctl_is_driver": ctl_ptr == driver_record,
                "ctl_ptr_minus_craft": ctl_ptr - craft,
                "ctl": ctl,
                "right": rows[0:3], "up": rows[4:7], "fwd": rows[8:11],
            }

        t0 = time.time()
        deadline = t0 + args.seconds
        arm(args.stint)
        last_pos = None
        expiries = 0
        forced = False
        released_rows = 0
        total_game = 0.0
        prev = None
        cue_logged = False
        scan_started = time.time()
        while time.time() < deadline and expiries < args.expiries:
            pos_bytes = m.read(body + 0x200, 12)
            if pos_bytes == last_pos:
                time.sleep(0.002)
                continue
            last_pos = pos_bytes
            row = sample(t0)
            if prev is not None and prev["timer"] != row["timer"] and prev["flags"] & RUNNING:
                total_game += max(0.0, prev["timer"] - row["timer"])
            row["t_game"] = total_game
            row["running"] = bool(row["flags"] & RUNNING)
            if row["finished"]:
                log["events"].append({"finished_at_row": len(log["rows"])})
                print("the race is finished: PlayerStatus_Update holds blend 1.0 then; stopping", flush=True)
                break
            if prev is not None:
                row["dps"] = heading_rate_dps(prev, row)
                row["dt_timer"] = prev["timer"] - row["timer"]
            row["bank"] = math.degrees(math.asin(max(-1.0, min(1.0, row["right"][1]))))
            log["rows"].append(row)
            if row["running"]:
                if args.policy == "corner" and not forced and row["timer"] > args.force + 0.5 \
                        and abs(row.get("dps", 0.0)) > args.corner:
                    set_timer(args.force)
                    forced = True
                    log["events"].append({"forced_row": len(log["rows"]), "dps": row["dps"],
                                          "bank": row["bank"], "timer_before": row["timer"],
                                          "speed": row["speed"]})
            else:
                released_rows += 1
                if released_rows >= 12:
                    expiries += 1
                    released_rows = 0
                    forced = False
                    print("expiry %d at row %d" % (expiries, len(log["rows"])), flush=True)
                    if args.policy == "cue":
                        break
                    arm(args.stint)
            prev = row
        log["events"].append({"expiries": expiries, "rows": len(log["rows"])})
        session.pad.release_all()
        args.out.write_text(json.dumps(log))
        print("wrote %s (%d rows, %d expiries)" % (args.out, len(log["rows"]), expiries), flush=True)


if __name__ == "__main__":
    main()
