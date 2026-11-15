#!/usr/bin/env python3
"""Log what Wipeout HD/Fury's collision sound is computed from, next to the cue it starts.

`Ship_UpdateCollisionCues` (`0x000d8bd0`) reads four craft fields the contact-ring walk in
`Ship_UpdateEntity` fills each frame, and starts `.COLLISIONS` (or `.COLLSHIELD`) and `.GRIND`:

    ship+0x6904  i32  frames until the next collision cue may start (set from tuning +0x4c)
    ship+0x6908  i32  the same for the grind cue (player only)
    ship+0x690c  f32  the frame's largest contact impulse (zeroed before the ring walk)
    ship+0x6910  u32  record word 0 of that contact: zero for a wall, non-zero for a craft
    ship+0x6920  vec  the impulse's direction
    ship+0x6930  f32  dot(body forward, contact normal)

This polls those fields on all eight craft and the SCREAM handler list (as
`rpcs3-hd-cue-handlers.py` does) through the `serve`'s memory channel, without stopping the game,
and writes one JSON line per event:

    {"kind": "arm", ...}      a craft's collision or grind countdown was seen risen: a cue started
    {"kind": "handler", ...}  a new SBlk handler: bank, cue index, the four cue variables, volume

`--arm 5` gives the player a Shield every `--arm-every` seconds (`*(ship + 0x5edc) + 0x204`), so
contacts land with the slot's shield bit (`+0x200 & 0x10`) set and the `.COLLSHIELD` branch runs.

`--analyse FILE` then pairs each arm with the collision handler that started beside it and prints
how often the handler's variables are what the law predicts from the impulse.

Evidence page: docs/ghidra/functions/ps3-hdfury-eu/collision-cues.md.

    scripts/emu-env.sh <lane> <display> <gdb-port>; source data/scratch/<lane>/env.sh
    scripts/emu-restore-state.sh <lane> data/saves/hd-fury/hd-racebox-talons-venom-weapons-grid.SAVESTAT.zst
    python3 scripts/rpcs3-drive.py press cross        # START RACE
    export OAG_RPCS3_ATTACH=1
    uv run --with evdev python3 scripts/emu-run.py -- \\
        python3 -u scripts/rpcs3-hd-collision-cue.py --out <dir> --seconds 120 --weave 2.5
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

GUEST = 0x300000000
CRAFT_ARRAY = 0x0098D7C0
LIST_SLOT = 0x008C00FC
TUNING = 0x008C15E0
ROLE = 0x628C
SLOT = 0x5EDC
SLOT_STATE = 0x204
PERSPECTIVE = 0x5F14
BLOCK = 0x6904
BLOCK_LEN = 0x30
HANDLER_LEN = 0x190

# The law read at 0x000d8f90..0x000d9004 (collision) and 0x000d8d78..0x000d8ddc (grind).
ANGLE_LIMIT = 0.7
HIT_FLOOR, HIT_FLOOR_SIDE = 3.0, 18.0
HIT_MEDIUM, HIT_LARGE = 25.0, 50.0
GRIND_FLOOR, GRIND_MEDIUM, GRIND_LARGE = 0.5, 4.0, 8.0


def severity(impulse, medium, large):
    return 2 if impulse > large else 1 if impulse > medium else 0


def hit_volume(impulse):
    return min(impulse / 28.0 + 0.2, 1.0)


def grind_volume(impulse):
    return min(impulse * 0.125 + 0.25, 1.0)


def capture(args):
    args.out.mkdir(parents=True, exist_ok=True)
    rows, banks, seen = [], {}, set()
    out = args.out / "collision-cues.jsonl"

    def emit(row):
        rows.append(row)
        print(json.dumps(row), flush=True)

    with drive.Session("x", str(args.out / "logs")) as session:
        mem = session.open_mem()

        def rd(address, n):
            mem.seek(GUEST + address)
            return mem.read(n)

        def u32(address):
            return struct.unpack(">I", rd(address, 4))[0]

        root = u32(LIST_SLOT)
        ships = [s for s in struct.unpack(">8I", rd(CRAFT_ARRAY, 32)) if s]
        tuning = rd(TUNING, 0x60)
        emit({"kind": "tuning", "angle": struct.unpack_from(">f", tuning, 0x48)[0],
              "hit_frames": struct.unpack_from(">i", tuning, 0x4C)[0],
              "grind_frames": struct.unpack_from(">i", tuning, 0x50)[0]})
        roles = {ship: struct.unpack(">i", rd(ship + ROLE, 4))[0] for ship in ships}
        emit({"kind": "ships", "roles": {"%#x" % s: r for s, r in roles.items()}})
        last = {ship: (0, 0) for ship in ships}
        recent = {ship: [] for ship in ships}
        if not args.no_pad:
            session.pad.set("cross", True)
        started = time.time()
        polls = 0
        armed_at = None
        player = next((ship for ship in ships if roles[ship] == 0), None)
        while time.time() - started < args.seconds:
            now = time.time() - started
            polls += 1
            if args.arm is not None and player and (armed_at is None or now - armed_at > args.arm_every):
                armed_at = now
                slot = u32(player + SLOT)
                mem.seek(GUEST + slot + SLOT_STATE)
                mem.write(struct.pack(">ii", args.arm, args.arm))
                session.pad.press("triangle", 0.15)
                emit({"kind": "armed", "t": round(now, 4), "state": args.arm})
            if args.weave and not args.no_pad:
                session.pad.stick(args.amplitude * math.sin(2 * math.pi * now / args.weave), 0.0)
            try:
                for ship in ships:
                    raw = rd(ship + BLOCK, BLOCK_LEN)
                    hit_left, grind_left = struct.unpack_from(">ii", raw, 0)
                    impulse = struct.unpack_from(">f", raw, 8)[0]
                    other = struct.unpack_from(">I", raw, 0xC)[0]
                    dot = struct.unpack_from(">f", raw, 0x2C)[0]
                    history = recent[ship]
                    history.append((now, impulse, other, dot))
                    del history[:-4]
                    before = last[ship]
                    for which, left, was in (("hit", hit_left, before[0]), ("grind", grind_left, before[1])):
                        if left > was:
                            slot = u32(ship + SLOT)
                            emit({"kind": "arm", "which": which, "t": round(now, 4), "ship": "%#x" % ship,
                                  "role": roles[ship], "left": left, "impulse": impulse, "other": other,
                                  "dot": dot, "recent": [[round(t, 4), i, o, d] for t, i, o, d in history],
                                  "slot_flags": u32(slot + 0x200) if slot else None,
                                  "perspective": u32(ship + PERSPECTIVE)})
                    last[ship] = (hit_left, grind_left)
                handler = u32(root + 0x24)
                hops = 0
                while handler and hops < 200:
                    hops += 1
                    raw = rd(handler, HANDLER_LEN)
                    following = struct.unpack_from(">I", raw, 0x70)[0]
                    key = (handler, struct.unpack_from(">I", raw, 0x184)[0])
                    if key not in seen and raw[0] & 0x1F == 5:
                        seen.add(key)
                        cue = struct.unpack_from(">I", raw, 4)[0]
                        bank = struct.unpack_from(">I", raw, 0xB0)[0]
                        name = "%#x" % bank
                        if name not in banks:
                            head = rd(bank, 0x40)
                            banks[name] = {"cues": struct.unpack_from(">H", head, 0x16)[0],
                                           "cue_table": struct.unpack_from(">I", head, 0x1C)[0]}
                        table = banks[name]["cue_table"]
                        emit({"kind": "handler", "t": round(now, 4), "bank_cues": banks[name]["cues"],
                              "index": (cue - (table if table > 0x10000 else bank + table)) // 12,
                              "vars": list(struct.unpack("4b", raw[0xA4:0xA8])),
                              "vol16": struct.unpack_from(">h", raw, 0x16)[0]})
                    handler = following
            except (OSError, struct.error, KeyError) as error:
                print("poll error %r" % error, flush=True)
                time.sleep(0.2)
        out.write_text("".join(json.dumps(row) + "\n" for row in rows))
        print("%d polls in %.1f s, %d rows -> %s" % (polls, time.time() - started, len(rows), out), flush=True)
        if not args.no_pad:
            session.pad.stick(0.0, 0.0)
            session.pad.set("cross", False)
    return 0


def analyse(path, ship_cues, window):
    """Pairs each `arm` with the nearest collision-family handler and scores the law's prediction.

    `ship_cues` maps a cue index of the ship bank (the bank with `SHIP_BANK_CUES` cues) to
    `(family, severity)`; the poll sees the child cue because opcode 0x08 replaces the handler's.
    """
    rows = [json.loads(line) for line in Path(path).read_text().splitlines() if line.strip()]
    handlers = [r for r in rows if r["kind"] == "handler" and r["index"] in ship_cues
                and r["bank_cues"] == SHIP_BANK_CUES]
    used = set()
    tally = {}
    for arm in (r for r in rows if r["kind"] == "arm"):
        impulses = [i for _, i, _, _ in arm["recent"] if i > 0.0]
        if arm["which"] == "hit":
            predicted = {severity(i, HIT_MEDIUM, HIT_LARGE) for i in impulses}
        else:
            predicted = {severity(i, GRIND_MEDIUM, GRIND_LARGE) for i in impulses}
        best = None
        for n, handler in enumerate(handlers):
            family, _ = ship_cues[handler["index"]]
            wanted = "grind" if arm["which"] == "grind" else "hit"
            if n in used or abs(handler["t"] - arm["t"]) > window or family.endswith("grind") != (wanted == "grind"):
                continue
            if best is None or abs(handler["t"] - arm["t"]) < abs(handlers[best]["t"] - arm["t"]):
                best = n
        key = (arm["which"], "player" if arm["role"] == 0 else "other")
        entry = tally.setdefault(key, {"arms": 0, "paired": 0, "var0": 0, "var1": 0})
        entry["arms"] += 1
        if best is None:
            continue
        used.add(best)
        handler = handlers[best]
        entry["paired"] += 1
        entry["var0"] += handler["vars"][0] in predicted
        surface = 0 if arm["other"] else 1
        entry["var1"] += handler["vars"][1] == surface
        # The player's cue is unplaced, so its volume word is the request's times the cue's own
        # authored volume: the ratio printed here is that authored volume when the law holds.
        law = hit_volume if arm["which"] == "hit" else grind_volume
        requested = int(law(arm["impulse"]) * 1024.0)
        print(json.dumps({"which": arm["which"], "role": arm["role"], "impulse": arm["impulse"],
                          "other": bool(arm["other"]), "dot": arm["dot"], "shield": bool((arm["slot_flags"] or 0) & 0x10),
                          "cue": ship_cues[handler["index"]], "vars": handler["vars"], "vol16": handler["vol16"],
                          "vol_ratio": round(handler["vol16"] / requested, 4) if arm["role"] == 0 else None,
                          "dt": round(handler["t"] - arm["t"], 4)}))
    for key, entry in sorted(tally.items()):
        print(key, entry)
    return 0


SHIP_BANK_CUES = 84


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", type=Path, help="directory for collision-cues.jsonl")
    ap.add_argument("--seconds", type=float, default=120.0)
    ap.add_argument("--weave", type=float, default=0.0, help="swing the left stick with this period, seconds")
    ap.add_argument("--amplitude", type=float, default=1.0, help="the swing's size, 0..1")
    ap.add_argument("--no-pad", action="store_true", help="leave the pad alone (no held cross)")
    ap.add_argument("--arm", type=int, help="give the player this pickup state and fire it (5 is the Shield)")
    ap.add_argument("--arm-every", type=float, default=10.0, help="seconds between two --arm firings")
    ap.add_argument("--analyse", type=Path, help="score a capture instead of taking one")
    ap.add_argument("--cues", type=Path, help="JSON: ship-bank cue index -> [family, severity], for --analyse")
    ap.add_argument("--window", type=float, default=0.08, help="seconds an arm and its handler may be apart")
    args = ap.parse_args()
    if args.analyse:
        cues = {int(k): tuple(v) for k, v in json.loads(args.cues.read_text()).items()}
        return analyse(args.analyse, cues, args.window)
    if not args.out:
        ap.error("--out is required for a capture")
    return capture(args)


if __name__ == "__main__":
    sys.exit(main())
