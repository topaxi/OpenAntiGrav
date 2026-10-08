#!/usr/bin/env python3
"""Ride height of the player craft in Wipeout HD on RPCS3: flyby, grid, straight.

Companion of `scripts/rpcs3-trace.py` (same chain, same no-pause guest reads).
One boot samples the player's body, craft entry and ship raw (so a field can be
found offline) at three moments:

  1. the pre-race fly-over, BEFORE the cross tap (the craft is pinned to its
     grid slot there), `--flyby-secs` at `--flyby-step`, a screenshot every
     `--shot-every` samples;
  2. the grid at rest after the countdown, `--rest-secs`;
  3. a straight, the `--script` .inputs held by game clock, from the grid.

Writes `<out>/height.pkl` and `<out>/height.csv`; frames under `<out>/frames/`.
Every row is wall time, phase, body position + rows, and the 4 hull points and
their query points the entry holds at `+0x120..+0x190`.
"""
import argparse
import csv
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
drive, place, input_script, Debugger = tr.drive, tr.place, tr.input_script, tr.Debugger


def sample(guest, objs, phase, t0):
    s = tr.snapshot(guest, objs)
    try:
        screen = drive.current_screen()
    except Exception:
        screen = "?"
    return {"wall": time.time() - t0, "phase": phase, "raw": s, "screen": screen}


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--nav", action="append", default=[])
    ap.add_argument("--flyby-secs", type=float, default=30.0)
    ap.add_argument("--flyby-step", type=float, default=0.5)
    ap.add_argument("--shot-every", type=int, default=4)
    ap.add_argument("--countdown", type=float, default=22.0)
    ap.add_argument("--keep-skipped", action="store_true")
    ap.add_argument("--count-secs", type=float, default=0.0,
                    help="sample the countdown at --count-step for this long right after the "
                         "cross tap (phase 'count', with the screen name) instead of sleeping "
                         "--countdown")
    ap.add_argument("--count-step", type=float, default=0.1)
    ap.add_argument("--count-shot-every", type=int, default=10)
    ap.add_argument("--rest-secs", type=float, default=4.0)
    ap.add_argument("--script", default="verification/scenarios/hd-thrust.inputs")
    args = ap.parse_args()
    out = Path(args.out)
    (out / "frames").mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    plan = {}
    for item in args.nav:
        screen, _, buttons = item.partition("=")
        plan[screen] = [b.strip() for b in buttons.split(",") if b.strip()]
    rows = []
    with drive.Session(args.image, str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        (out / "rpcs3.pid").write_text("%d\n" % session.proc.pid)
        if not session.wait_for_screen_pressing("Main Menu", 240):
            sys.exit("never reached the Main Menu")
        session.settle_menu(20)
        gdb = Debugger(port=port)
        tr.set_pilot_assist(gdb, False)
        session.screens_seen = set()
        session.shot_dir = out / "screens"
        session.shot_dir.mkdir(exist_ok=True)

        def navigate(plan, session=session):
            screen = drive.current_screen()
            for n, button in enumerate(plan.get(screen, [])):
                if button == "wait":
                    time.sleep(3.0)
                elif button in ("cross", "circle"):
                    session.press_once(button)
                else:
                    session.tap(button, settle=1.5)
                time.sleep(1.5)
                print("    nav %-8s at %s" % (button, screen), flush=True)
            plan.pop(screen, None)

        session.navigate = navigate
        if session.walk_to_race(plan=plan) not in drive.RACE_ARRIVED:
            sys.exit("no race")
        print("track: %s" % drive.track_name(), flush=True)
        t0 = time.time()
        ship = body = None
        while ship is None and time.time() - t0 < 120:
            time.sleep(1.5)
            gdb.pause()
            try:
                ship, body = place.find_player(gdb)
                entry = struct.unpack(">I", gdb.read(ship + place.OFF_ENTRY, 4))[0]
                ships = struct.unpack(">8I", gdb.read(place.CRAFT_ARRAY, 32))
                rivals = [struct.unpack(">I", gdb.read(o + place.OFF_BODY, 4))[0]
                          for o in ships if o and o != ship]
            except Exception as exc:  # the array is not built yet
                ship = None
                print("  not yet: %s" % exc, flush=True)
            gdb.resume()
        if ship is None:
            sys.exit("no player craft")
        print("player found %.1f s after race arrival" % (time.time() - t0), flush=True)
        guest = tr.Guest(session.proc.pid, session.open_mem())
        world = tr.find_world(guest, body)
        objs = {"world": (world, 0x50), "body": (body, 0x210), "entry": (entry, 0x600),
                "ship": (ship, 0x200)}
        meta = {"ship": ship, "body": body, "entry": entry, "world": world,
                "rivals": rivals, "track": drive.track_name()}
        (out / "meta.txt").write_text(repr(meta) + "\n")
        t0 = time.time()
        n = 0
        h0 = struct.unpack_from(">f", tr.snapshot(guest, objs)["entry"], 0x260)[0]
        print("first height field (entry+0x260): %.3f" % h0, flush=True)
        if h0 > 3.0 and not args.keep_skipped:
            sys.exit("FLYBY SKIPPED: the craft is already at its hover height (a walk press landed on the prompt)")
        while time.time() - t0 < args.flyby_secs:
            r = sample(guest, objs, "flyby", t0)
            if n % args.shot_every == 0:
                p = out / "frames" / ("flyby-%03d.png" % n)
                drive.screenshot(p, trim=True)
                r["frame"] = str(p)
            rows.append(r)
            n += 1
            time.sleep(args.flyby_step)
        if args.count_secs > 0:
            session.tap("cross", settle=0.0)
            tc = time.time()
            c = 0
            while time.time() - tc < args.count_secs:
                r = sample(guest, objs, "count", t0)
                r["since_tap"] = time.time() - tc
                if c % args.count_shot_every == 0:
                    p = out / "frames" / ("count-%03d.png" % c)
                    drive.screenshot(p, trim=True)
                    r["frame"] = str(p)
                rows.append(r)
                c += 1
                time.sleep(args.count_step)
        else:
            session.tap("cross", settle=args.countdown)
        drive.screenshot(out / "frames" / "grid.png", trim=True)
        t1 = time.time()
        k = 0
        while time.time() - t1 < args.rest_secs:
            r = sample(guest, objs, "rest", t0)
            if k == 0:
                r["frame"] = str(out / "frames" / "rest.png")
                drive.screenshot(out / "frames" / "rest.png", trim=True)
            rows.append(r)
            k += 1
            time.sleep(0.1)
        states = input_script.parse(Path(args.script).read_text())
        raw = tr.run_script(session, guest, objs, states, dict(tr.DEFAULT_MAP), 0, rivals)
        for game_t, wt, s, rival, pressed in raw:
            rows.append({"wall": 0.0, "phase": "straight", "game_t": game_t, "raw": s})
        drive.screenshot(out / "frames" / "straight-end.png", trim=True)
        pickle.dump(rows, open(out / "height.pkl", "wb"))
        gdb.close()
    print("done: %d samples" % len(rows), flush=True)


if __name__ == "__main__":
    main()
