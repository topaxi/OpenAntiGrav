#!/usr/bin/env python3
"""Film HD's lock-on sight on RPCS3 and log the HUD fields that drive it (player input).

`Hud_UpdateMissileSight` (`0x0008a5b8`) and `Hud_UpdateLeachBeamSight` (`0x00090d30`) read and
write fields of the HUD object (`docs/ghidra/functions/ps3-hdfury-eu/hud-sight.md`): the lock byte
`hud+0xdc`, the accumulator `hud+0xec`, the Leach Beam's loop clock `hud+0x604`, the Missile's spin
angle `hud+0x634`, the cue latch `hud+0x6b8`, and ten widget pointers `hud+0x578..0x59c`. This
finds the HUD object by those ten pointers, hands the player a weapon (the pickup slot's state
word, `docs/reverse-engineering/rpcs3-capture.md`, "Giving the player a weapon"), holds cross, and
polls the object through `/proc` memory without pausing, logging a row per poll and a screenshot
every `--shot-every` seconds. Wall clock is not game time (RPCS3 runs HD at a fraction of real
time): read time off `hud+0xec` and `hud+0x604`, which are accumulated frame deltas.

    OAG_RPCS3_ATTACH=1 ... uv run --with evdev --with numpy python3 scripts/rpcs3-hd-lock-sight.py \\
        --image data/images/hdfury-ps3-eu-dec.iso --out <dir> --state 1 --seconds 25

Attach to a `rpcs3-drive.py serve` parked on a race grid; without `--in-race` it presses START RACE first.
"""
import argparse
import importlib.util
import struct
import sys
import time
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
_spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)
import rpcs3_place as place  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402

GUEST = 0x300000000
SLOT_OFFSET = 0x5EDC
STATE = 0x204

WIDGETS = 0x578
WIDGET_NAMES = [
    "MissileSightBG", "MissileSightOuter", "MissileSightInner", "MissileSightMiddle",
    "MissileSightLockedOnLines", "MissileSightLockedOnMiddle",
    "LeachBeamSightBG", "LeachBeamSightOuter", "LeachBeamSightMiddle", "LeachBeamSightInner",
]
WIDGET_SPAN = 0x140


def u32(mem, addr):
    mem.seek(GUEST + addr)
    return struct.unpack(">I", mem.read(4))[0]


def find_hud(mem, lo, hi, chunk=0x1000000):
    """Guest address of the HUD object: ten consecutive heap pointers at `+0x578`, six of which
    share one vtable word, and the "already bound" byte at `+0x161` set."""
    hits = []
    for base in range(lo, hi, chunk):
        mem.seek(GUEST + base)
        raw = mem.read(chunk + 0x600)
        if len(raw) < 0x700:
            continue
        words = np.frombuffer(raw[: len(raw) & ~3], dtype=">u4").astype(np.uint32)
        ok = ((words >= 0x10000000) & (words < 0x80000000) & ((words & 3) == 0)).astype(np.int8)
        run = np.convolve(ok, np.ones(10, dtype=np.int8), mode="valid")
        for i in np.nonzero(run == 10)[0]:
            at = base + int(i) * 4
            ptrs = words[i : i + 10]
            if len(set(ptrs.tolist())) != 10:
                continue
            hits.append((at - WIDGETS, ptrs.tolist()))
    good = []
    for hud, ptrs in hits:
        if hud % 16 or hud < lo:
            continue
        try:
            vt = [u32(mem, p) for p in ptrs]
            flag = mem_byte(mem, hud + 0x161)
        except struct.error:
            continue
        # The Leach Beam's four are authored at 176/176/128/80 px (`hud hud_art`, hud-sight.md).
        sizes = []
        for p in ptrs[6:]:
            mem.seek(GUEST + p + 0xAC)
            sizes.append(struct.unpack(">f", mem.read(4))[0])
        if sizes == [176.0, 176.0, 128.0, 80.0] and flag:
            good.append((hud, ptrs, vt))
    return good


def mem_byte(mem, addr):
    mem.seek(GUEST + addr)
    return mem.read(1)[0]


def speed_of(mem, body):
    mem.seek(GUEST + body + place.B_VELOCITY)
    v = struct.unpack(">3f", mem.read(12))
    return sum(c * c for c in v) ** 0.5


def teleport_behind(session, gdb, mem, body, distance, timeout=240.0):
    """Wait for GO (the player moving), then put the player `distance` units behind the nearest
    craft ahead of it, on that craft's heading and at its speed, so a lock has a target."""
    began = time.time()
    while speed_of(mem, body) < 30.0:
        if time.time() - began > timeout:
            sys.exit("the player never moved: GO did not come")
        time.sleep(0.1)
    gdb.pause()
    me = place.read_pose(gdb, body)
    best = None
    ships = struct.unpack(">8I", gdb.read(place.CRAFT_ARRAY, 32))
    for ship in ships:
        if not ship or struct.unpack(">I", gdb.read(ship + place.OFF_ROLE, 4))[0] == 0:
            continue
        other_body = struct.unpack(">I", gdb.read(ship + place.OFF_BODY, 4))[0]
        pose = place.read_pose(gdb, other_body)
        ahead = sum((pose["pos"][i] - me["pos"][i]) * me["rows"][2][i] for i in range(3))
        if ahead > 0 and (best is None or ahead < best[0]):
            best = (ahead, pose)
    if best is None:
        gdb.resume()
        sys.exit("no craft ahead of the player")
    ahead, pose = best
    fwd = pose["rows"][2]
    speed = sum(c * c for c in pose["vel"]) ** 0.5
    pos = tuple(pose["pos"][i] - distance * fwd[i] for i in range(3))
    place.write_pose(gdb, body, pos, pose["rows"], speed)
    gdb.resume()
    print("teleported %.0f units behind a craft that was %.0f ahead, speed %.0f" % (distance, ahead, speed), flush=True)


def snapshot(mem, hud, widgets):
    mem.seek(GUEST + hud + 0xD0)
    a = mem.read(0x40)
    mem.seek(GUEST + hud + 0x600)
    b = mem.read(0x40)
    mem.seek(GUEST + hud + 0x6B0)
    c = mem.read(0x10)
    rows = []
    for w in widgets:
        mem.seek(GUEST + w)
        rows.append(mem.read(WIDGET_SPAN))
    return a, b, c, rows


def f32(blob, at):
    return struct.unpack_from(">f", blob, at)[0]


def row_text(t, snap):
    a, b, c, rows = snap
    bits = "".join("1" if (struct.unpack_from(">I", r, 0x34)[0] & 4) else "0" for r in rows)
    wid = " ".join(
        "%d:%.1f,%.1f,r%.2f" % (i, f32(r, 0xAC), f32(r, 0xB0), f32(r, 0x12C))
        for i, r in enumerate(rows)
    )
    return (
        "%.3f dc=%08x ec=%.4f e0=%.4f e4=%.4f t604=%.4f r634=%.4f c6b8=%08x vis=%s %s"
        % (
            t,
            struct.unpack_from(">I", a, 0xDC - 0xD0)[0],
            f32(a, 0xEC - 0xD0),
            f32(a, 0xE0 - 0xD0),
            f32(a, 0xE4 - 0xD0),
            f32(b, 0x4),
            f32(b, 0x34),
            struct.unpack_from(">I", c, 0x8)[0],
            bits,
            wid,
        )
    )


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--state", type=int, default=1, help="pickup state: 1 Missile; Leach Beam is read off the HUD icon")
    ap.add_argument("--seconds", type=float, default=25.0, help="polled after the weapon is given")
    ap.add_argument("--shot-every", type=float, default=0.0, help="seconds between periodic screenshots, 0 for none")
    ap.add_argument("--shot-on-change", action="store_true", help="a screenshot (paused) whenever the lock byte, cue latch or a widget bit changes")
    ap.add_argument("--in-race", action="store_true", help="do not press START RACE")
    ap.add_argument("--restart", action="store_true", help="Restart Race from the pause menu first")
    ap.add_argument("--hold", default="cross", help="button held while polling")
    ap.add_argument("--scan-lo", type=lambda s: int(s, 0), default=0x10000000)
    ap.add_argument("--scan-hi", type=lambda s: int(s, 0), default=0x50000000)
    ap.add_argument("--hud", type=lambda s: int(s, 0), help="skip the scan")
    ap.add_argument("--behind", type=float, default=0.0,
                    help="after GO, put the player this many units behind the nearest craft ahead, at its pose and speed")
    ap.add_argument("--countdown", type=float, default=3.0, help="seconds after START RACE before polling")
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    with drive.Session(args.image, str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        if args.restart:
            session.restart_race()
        gdb = Debugger(port=port)
        gdb.pause()
        ship, body = place.find_player(gdb)
        slot = struct.unpack(">I", gdb.read(ship + SLOT_OFFSET, 4))[0]
        gdb.resume()
        print("ship %#x slot %#x" % (ship, slot), flush=True)
        mem = session.open_mem()
        hud = args.hud
        if hud is None:
            found = find_hud(mem, args.scan_lo, args.scan_hi)
            for hud_at, ptrs, vt in found:
                print("hud candidate %#x widgets %s vtables %s" % (hud_at, [hex(p) for p in ptrs], [hex(v) for v in vt]))
            if not found:
                sys.exit("no HUD object found in %#x..%#x" % (args.scan_lo, args.scan_hi))
            hud = found[0][0]
        mem.seek(GUEST + hud + WIDGETS)
        widgets = list(struct.unpack(">10I", mem.read(40)))
        print("hud %#x" % hud, flush=True)
        if not args.in_race:
            # The race sits on the grid at its START RACE prompt (walk it there with the Racebox
            # route in `scripts/emu-hd-race-walk.sh`); cross starts it. The HUD object is found
            # first, while the grid waits.
            session.tap("cross", settle=args.countdown)

        gdb.pause()
        gdb.write(slot + STATE, struct.pack(">ii", args.state, args.state))
        gdb.resume()
        session.pad.set(args.hold, True)
        if args.behind:
            teleport_behind(session, gdb, mem, body, args.behind)
        began = time.time()
        last_shot = -1e9
        last_key = None
        shots = 0
        log = open(out / "poll.log", "w")
        try:
            while time.time() - began < args.seconds:
                now = time.time() - began
                snap = snapshot(mem, hud, widgets)
                line = row_text(now, snap)
                a, _b, c, rows = snap
                bits = tuple(struct.unpack_from(">I", r, 0x34)[0] & 4 for r in rows)
                key = (a[0xDC - 0xD0], c[8], bits)
                due = args.shot_every > 0 and now - last_shot >= args.shot_every
                changed = args.shot_on_change and key != last_key and last_key is not None
                last_key = key
                if due or changed:
                    # Paused, so the picture and the row are the same frame.
                    gdb.pause()
                    snap = snapshot(mem, hud, widgets)
                    line = row_text(time.time() - began, snap)
                    name = "shot%03d.png" % shots
                    drive.screenshot(str(out / name), trim=True)
                    (out / (name + ".widgets")).write_text("\n".join(r.hex() for r in snap[3]))
                    gdb.resume()
                    last_shot = time.time() - began
                    shots += 1
                    line += " shot=" + name
                log.write(line + "\n")
                log.flush()
                time.sleep(0.01)
        finally:
            session.pad.set(args.hold, False)
            log.close()
    print("done, %d shots, log %s" % (shots, out / "poll.log"))


if __name__ == "__main__":
    main()
