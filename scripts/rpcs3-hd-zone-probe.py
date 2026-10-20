#!/usr/bin/env python3
"""HD Zone race on RPCS3: dump the player's craft entry, class block and team block.

Cold boot straight into Zone (so the team's
stats block is loaded from Data\\Ships\\Zone\\handlingstats.xml, never a racing file
first), then sample entry (0x600), *(entry+0x7c) (class block, 0x200) and
*(entry+0x78) (team block, 0x200) through /proc mem, no pause.
Run from the repository root (imports scripts/). Source a private env first
(scripts/emu-env.sh) and wrap it in scripts/emu-run.py; see
docs/ghidra/functions/ps3-hdfury-eu/hover-four-point.md, "Zone, read live".
"""
import argparse
import importlib.util
import pickle
import struct
import sys
import time
from pathlib import Path

ROOT = Path.cwd()
sys.path.insert(0, str(ROOT / "scripts"))
_spec = importlib.util.spec_from_file_location("rpcs3_trace", ROOT / "scripts" / "rpcs3-trace.py")
tr = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(tr)
drive, place, Debugger = tr.drive, tr.place, tr.Debugger


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--rights", type=int, default=5)
    ap.add_argument("--drive-secs", type=float, default=60.0)
    ap.add_argument("--step", type=float, default=0.1)
    ap.add_argument("--steer", default="")
    args = ap.parse_args()
    out = Path(args.out)
    (out / "frames").mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    plan = {"Main Menu": ["right"],
            "Single Player": ["right"] * args.rights + ["cross"]}
    rows = []
    with drive.Session(args.image, str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        (out / "rpcs3.pid").write_text("%d\n" % session.proc.pid)
        if not session.wait_for_screen_pressing("Main Menu", 240):
            sys.exit("never reached the Main Menu")
        session.settle_menu(20)
        gdb = Debugger(port=port)
        tr.set_pilot_assist(gdb, False)

        def navigate(plan, session=session):
            screen = drive.current_screen()
            for button in plan.get(screen, []):
                if button in ("cross", "circle"):
                    session.press_once(button)
                else:
                    session.tap(button, settle=1.5)
                time.sleep(1.5)
                print("    nav %-8s at %s" % (button, screen), flush=True)
            if screen == "Single Player":
                drive.screenshot(out / "frames" / "single-player.png", trim=True)
            plan.pop(screen, None)

        session.navigate = navigate
        if session.walk_to_race(plan=plan) not in drive.RACE_ARRIVED:
            sys.exit("no race")
        tty = drive.tty_text()
        modes = [l for l in tty.splitlines() if "GetMode()" in l]
        print("track: %s; mode lines: %s" % (drive.track_name(), modes[-2:]), flush=True)
        t0 = time.time()
        ship = None
        while ship is None and time.time() - t0 < 120:
            time.sleep(1.5)
            gdb.pause()
            try:
                ship, body = place.find_player(gdb)
                entry = struct.unpack(">I", gdb.read(ship + place.OFF_ENTRY, 4))[0]
            except Exception as exc:
                ship = None
                print("  not yet: %s" % exc, flush=True)
            gdb.resume()
        if ship is None:
            sys.exit("no player craft")
        gdb.close()
        guest = tr.Guest(session.proc.pid, session.open_mem())
        cls = guest.u32(entry + 0x7c)
        team = guest.u32(entry + 0x78)
        objs = {"entry": (entry, 0x600), "class": (cls, 0x200), "team": (team, 0x200),
                "body": (body, 0x210)}
        meta = {"ship": ship, "body": body, "entry": entry, "class": cls, "team": team,
                "track": drive.track_name(), "modes": modes}
        (out / "meta.txt").write_text(repr(meta) + "\n")
        print("meta %r" % meta, flush=True)

        def run(phase, secs, shot_every=50):
            t = time.time()
            k = 0
            while time.time() - t < secs:
                r = {"wall": time.time() - t0, "phase": phase, "raw": tr.snapshot(guest, objs)}
                if k % shot_every == 0:
                    p = out / "frames" / ("%s-%04d.png" % (phase, k))
                    drive.screenshot(p, trim=True)
                    r["frame"] = str(p)
                rows.append(r)
                k += 1
                time.sleep(args.step)

        run("flyby", 3.0, shot_every=10)
        session.tap("cross", settle=0.0)
        run("count", 12.0, shot_every=20)
        session.pad.set("cross", True)
        run("drive", args.drive_secs)
        session.pad.set("cross", False)
        pickle.dump(rows, open(out / "zone.pkl", "wb"))
        drive.screenshot(out / "frames" / "end.png", trim=True)
    print("done: %d samples" % len(rows), flush=True)


if __name__ == "__main__":
    main()
