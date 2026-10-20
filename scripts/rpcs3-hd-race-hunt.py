#!/usr/bin/env python3
"""Wipeout HD (RPCS3): fly a Racebox Single Race on the original's Autopilot and stop at a mid-pack moment.

Attached to the `serve` that `scripts/emu-hd-race-walk.sh` leaves on the grid of a weapons-off Single Race
(stub off, Compatible Savestate Mode: the only flavour that can write a state). It

1. presses START RACE until the craft leaves its grid state,
2. arms the Autopilot by writing the pickup slot's running bit and timer through `serve`'s read-write memory
   (`RemoteMem.write`, 2026-10-20): `*(ship+0x5edc)+0x200 |= 0x800`, `+0x158 = 10000`
   (docs/ghidra/functions/ps3-hdfury-eu/autopilot.md),
3. finds the HUD's per-player record by scanning guest memory for it (`lap, laps, place` at `+0xc..+0x14`, shield at
   `+0x48`, its maximum `+0x50` = 140, docs/ghidra/functions/ps3-hdfury-eu/hud-readouts.md; the heap moves boot to boot),
4. polls `place`, every craft's position and the player's forward axis, and
5. when the player's place is in `--place` and at least `--min-near` other craft are within `--near-dist` of the
   player and in front of it, presses the PS menu's "Save Emulation State And Exit" (`emu-save-state.sh`'s steps), which
   writes the state a second or so later and ends the emulator.

There is no GDB stub in this flavour, so nothing stops the game at the match: the state is taken a few frames after it, and
the pair script reads the field again from the restored state. Raw captures are derived game data: keep them under `data/`.

    source data/scratch/<lane>/env.sh
    OAG_RPCS3_ATTACH=1 UV_CACHE_DIR=~/.cache/uv UV_OFFLINE=1 uv run --with evdev python3 scripts/rpcs3-hd-race-hunt.py \\
        --lane <lane> --name hd-race-talons-venom-pack --place 3-6
"""
import argparse
import importlib.util
import json
import math
import struct
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
_spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)

import rpcs3_hd_field  # noqa: E402
from rpcs3_hd_field import CRAFT_ARRAY, Mem, find_record  # noqa: E402

SLOT = 0x5EDC


def parse_range(text):
    lo, _, hi = text.partition("-")
    return int(lo), int(hi or lo)


def ships_of(m):
    ships = struct.unpack(">8I", m.rd(CRAFT_ARRAY, 32))
    out = []
    for ship in ships:
        entry = m.u32(ship + 0x5FAC)
        body = m.u32(entry + 0x270)
        out.append({"ship": ship, "body": body, "player": m.u32(ship + 0x7A60) == 0})
    return out


def pose_of(m, body):
    blob = m.rd(body + 0x1D0, 0x40)
    fwd = struct.unpack_from(">3f", blob, 0x20)
    pos = struct.unpack_from(">3f", blob, 0x30)
    return pos, fwd


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--lane", required=True)
    ap.add_argument("--name", required=True, help="state name under data/saves/hd-fury/")
    ap.add_argument("--place", type=parse_range, default=(3, 6))
    ap.add_argument("--near-dist", type=float, default=45.0)
    ap.add_argument("--min-near", type=int, default=1)
    ap.add_argument("--min-front", type=int, default=2, help="craft within --front-dist in front of the player")
    ap.add_argument("--front-dist", type=float, default=110.0)
    ap.add_argument("--seconds", type=float, default=420.0)
    ap.add_argument("--image", default=str(HERE.parent / "data/images/hdfury-ps3-eu-dec.iso"))
    ap.add_argument("--log", type=Path)
    args = ap.parse_args()
    t0 = time.time()
    with drive.Session(args.image, str(Path("/home/topaxi/projects/OpenAntiGrav/data/scratch") / args.lane / "hunt-logs")) as session:
        m = Mem(session.open_mem())
        ships = ships_of(m)
        player = next(s for s in ships if s["player"])
        craft = m.u32(player["ship"] + 0x5FAC)
        deadline = time.time() + 120
        while time.time() < deadline:
            flags = m.u32(craft + 0x264)
            if not flags & 0x2:
                break
            session.pad.set("cross", True)
            time.sleep(0.15)
            session.pad.set("cross", False)
            time.sleep(4)
        time.sleep(2)
        slot = m.u32(player["ship"] + SLOT)
        m.wr(slot + 0x200, struct.pack(">I", m.u32(slot + 0x200) | 0x800))
        m.wr(slot + 0x158, struct.pack(">f", 10000.0))
        print("autopilot armed: slot flags %#x timer %.0f" % (m.u32(slot + 0x200), m.f32(slot + 0x158)), flush=True)
        records = find_record(m, m.f32(player["ship"] + 0x5FA0))
        print("records", [hex(r) for r in records], flush=True)
        if len(records) != 1:
            sys.exit("expected exactly one HUD record, found %d" % len(records))
        record = records[0]
        lo, hi = args.place
        n = 0
        last = 0
        while time.time() - t0 < args.seconds:
            n += 1
            h = rpcs3_hd_field.hud(m, record)
            place, lap = h["place"], h["lap"]
            if place is None:
                time.sleep(0.05)
                continue
            if time.time() - last > 5:
                last = time.time()
                print("t %.0f lap %d place %d" % (time.time() - t0, lap, place), flush=True)
            if not lo <= place <= hi:
                time.sleep(0.05)
                continue
            ppos, pfwd = pose_of(m, player["body"])
            near, front = [], []
            for s in ships:
                if s["player"]:
                    continue
                pos, _ = pose_of(m, s["body"])
                d = [a - b for a, b in zip(pos, ppos)]
                dist = math.sqrt(sum(x * x for x in d))
                if dist <= args.near_dist:
                    near.append(round(dist, 1))
                if dist <= args.front_dist and sum(a * b for a, b in zip(d, pfwd)) > 0.6 * dist:
                    front.append(round(dist, 1))
            if len(near) >= args.min_near and len(front) >= args.min_front:
                print("MATCH t %.0f lap %d place %d near %s front %s" % (time.time() - t0, lap, place, near, front), flush=True)
                if args.log:
                    args.log.write_text(json.dumps({"lap": lap, "place": place, "near": near, "front": front}))
                break
        else:
            sys.exit("no match in %.0f s" % args.seconds)
    # the PS menu's save, as scripts/emu-save-state.sh does it (it ends the emulator)
    done = subprocess.run([str(HERE / "emu-save-state.sh"), args.lane, args.name], capture_output=True, text=True)
    print(done.stdout + done.stderr)
    sys.exit(done.returncode)


if __name__ == "__main__":
    main()
