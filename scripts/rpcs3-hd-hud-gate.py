#!/usr/bin/env python3
"""HD's whole-HUD gate around the start, read live on RPCS3.

Cold boots into a Racebox Zone race (the walk of `rpcs3-hd-zone-palette.py`),
then samples through `/proc/<pid>/mem`, no pause, from the pre-race flyby
through the countdown to a few seconds after GO:

- the race manager (`*0x0095ae78`): phase `+0x1970`, sub-state `+0x1974`,
  countdown length `+0x1958` and elapsed `+0x1968`;
- the HUD object (`*0x0098767c`, the global `RaceManager_UpdateCountdown`
  writes): its vtable, node flags `+0x34` (bit `0x4` visible), `+0x44`, the
  timer `+0x630`, the countdown bytes `*(hud+0x40) + 0x68/0x69`, and the whole
  0x6d0-byte object for diffing;
- the flags `+0x34` of the HUD's bound group parents (`hud+0x4ec..+0x5e4`).

Writes `DIR/samples.pkl` and frames at every change of phase or HUD bits.
`docs/ghidra/functions/ps3-hdfury-eu/hud-start-gate.md`.
"""

import argparse
import importlib.util
import pickle
import struct
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
_spec = importlib.util.spec_from_file_location("rpcs3_trace", HERE / "rpcs3-trace.py")
tr = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(tr)
drive = tr.drive

RACE_MANAGER_SLOT = 0x0095AE78
HUD_SLOT = 0x0098767C
HUD_SIZE = 0x6D0
GROUPS = {
    "ForwardHUD": 0x4EC, "SpeedClassParent": 0x5B0, "DamageBarParent": 0x5B4,
    "SpeedBarParent": 0x5C8, "LapCountersParent": 0x5CC, "PositionsParent": 0x5D0,
    "TotalTimeParent": 0x5D4, "ZoneParent": 0x5D8, "ScoreFrame": 0x5E4,
}


def u32(b, o=0):
    return struct.unpack(">I", b[o:o + 4])[0]


def sample(guest):
    row = {"t": time.time()}
    rm = u32(guest.read(RACE_MANAGER_SLOT, 4))
    row["rm"] = rm
    if rm:
        blk = guest.read(rm + 0x1950, 0x28)
        row["phase"] = u32(blk, 0x20)
        row["sub"] = u32(blk, 0x24)
        row["cd_len"] = struct.unpack(">f", blk[8:12])[0]
        row["cd_elapsed"] = struct.unpack(">f", blk[0x18:0x1C])[0]
    hud = u32(guest.read(HUD_SLOT, 4))
    row["hud"] = hud
    if hud:
        raw = guest.read(hud, HUD_SIZE)
        row["raw"] = raw
        row["vtable"] = u32(raw, 0)
        row["flags34"] = u32(raw, 0x34)
        row["flags44"] = u32(raw, 0x44)
        row["t630"] = struct.unpack(">f", raw[0x630:0x634])[0]
        sub = u32(raw, 0x40)
        if sub:
            b = guest.read(sub + 0x68, 2)
            row["cd_bytes"] = (b[0], b[1])
        groups = {}
        for name, off in GROUPS.items():
            node = u32(raw, off)
            groups[name] = u32(guest.read(node + 0x34, 4)) if node else None
        row["groups"] = groups
    return row


def key(row):
    g = row.get("groups") or {}
    return (row.get("phase"), row.get("sub"), (row.get("flags34") or 0) & 0x6,
            tuple((v or 0) & 0x6 for v in g.values()), row.get("cd_bytes"))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--rights", type=int, default=5)
    ap.add_argument("--flyby-secs", type=float, default=4.0)
    ap.add_argument("--count-secs", type=float, default=14.0)
    ap.add_argument("--drive-secs", type=float, default=6.0)
    ap.add_argument("--boot-secs", type=float, default=400.0)
    ap.add_argument("--any-mode", action="store_true",
                    help="record a race of whatever type the Racebox remembers, not only Zone")
    ap.add_argument("--poke", action="store_true",
                    help="in the countdown, write hud+0x115 = 1, photograph, write 0, photograph")
    ap.add_argument("--step", type=float, default=0.1)
    args = ap.parse_args()
    out = Path(args.out)
    (out / "frames").mkdir(parents=True, exist_ok=True)
    plan = {"Main Menu": ["right"],
            "Single Player": ["right"] * args.rights + ["cross"]}
    rows = []
    with drive.Session(args.image, str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        (out / "rpcs3.pid").write_text("%d\n" % session.proc.pid)
        if session.guard is not None:
            # A cold PPU cache under load compiles the EBOOT for minutes.
            session.guard.stage("boot", limit=args.boot_secs)
        if not session.wait_for_screen_pressing("Main Menu", 240):
            sys.exit("never reached the Main Menu")
        session.settle_menu(20)
        if session.walk_to_race(plan=plan) not in drive.RACE_ARRIVED:
            sys.exit("no race")
        tty = drive.tty_text()
        modes = [line for line in tty.splitlines() if "GetMode()" in line]
        print("track: %s; mode lines: %s" % (drive.track_name(), modes[-2:]), flush=True)
        if not args.any_mode and not any("GetMode()==6" in line for line in modes[-1:]):
            sys.exit("not a Zone race: %s" % modes[-1:])
        session.wait_for_load(120)
        guest = tr.Guest(session.proc.pid, session.open_mem())
        t0 = time.time()
        shots = [0]
        poked = [not args.poke]

        def poke(hud, value, tag):
            with open("/proc/%d/mem" % session.proc.pid, "r+b", 0) as mem:
                mem.seek(tr.GUEST_BASE + hud + 0x115)
                mem.write(bytes([value]))
            time.sleep(1.5)
            back = guest.read(hud + 0x115, 1)[0]
            r = sample(guest)
            shots[0] += 1
            p = out / "frames" / ("%03d-poke-%s.png" % (shots[0], tag))
            drive.screenshot(p, trim=True)
            r.update(wall=r["t"] - t0, stage="poke-" + tag, frame=str(p), byte115=back)
            rows.append(r)
            print("  poke %s: wrote %d, read back %d, phase %s" % (tag, value, back, r.get("phase")), flush=True)

        def run(stage, secs, shot_every_s):
            t = time.time()
            last_key, last_shot = None, 0.0
            while time.time() - t < secs:
                r = sample(guest)
                r["wall"] = r["t"] - t0
                r["stage"] = stage
                k = key(r)
                if k != last_key or r["wall"] - last_shot >= shot_every_s:
                    shots[0] += 1
                    p = out / "frames" / ("%03d-%s-p%s.png" % (shots[0], stage, r.get("phase")))
                    drive.screenshot(p, trim=True)
                    r["frame"] = str(p)
                    last_shot = r["wall"]
                    print("  %6.2fs %-6s phase %s sub %s cd %.2f/%.2f hud %#x bits34 %#x cdb %s groups %s" % (
                        r["wall"], stage, r.get("phase"), r.get("sub"), r.get("cd_elapsed", -1),
                        r.get("cd_len", -1), r.get("hud", 0), r.get("flags34", 0), r.get("cd_bytes"),
                        {n: (v or 0) & 0x6 for n, v in (r.get("groups") or {}).items()}), flush=True)
                last_key = k
                rows.append(r)
                if not poked[0] and r.get("phase") == 1 and r.get("hud"):
                    poked[0] = True
                    poke(r["hud"], 1, "on")
                    poke(r["hud"], 0, "off")
                time.sleep(args.step)

        run("flyby", args.flyby_secs, 2.0)
        session.tap("cross", settle=0.0)
        run("count", args.count_secs, 1.0)
        session.pad.set("cross", True)
        run("drive", args.drive_secs, 2.0)
        session.pad.set("cross", False)
        pickle.dump({"rows": rows, "track": drive.track_name(), "modes": modes},
                    open(out / "samples.pkl", "wb"))
    print("done: %d samples" % len(rows), flush=True)


if __name__ == "__main__":
    main()
