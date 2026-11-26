#!/usr/bin/env python3
"""HD/Fury's Ship Select step against ours: the original's frame N game frames after the selection moved, N read from RAM.

    source data/scratch/<lane>/env.sh ; export XDG_CACHE_HOME=/tmp/oag-<lane>-cache
    OAG_COMPARE_PLATFORM=hd-fury python3 scripts/rpcs3-drive.py --image data/images/hdfury-ps3-eu.iso serve   # to Main Menu
    # walk to Ship Select by hand: `right cross right cross cross`
    uv run --with evdev --with numpy python3 scripts/rpcs3-hd-ship-step.py select --hull harimau   # step to a hull
    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/emu-run.py -- python3 scripts/rpcs3-hd-ship-step.py timeline \\
        --buttons right,right,left --out data/scratch/<lane>/timeline.json
    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/emu-run.py -- python3 scripts/rpcs3-hd-ship-step.py capture \\
        --out data/scratch/<lane>/trials --aims 2,3,4,6,8,12,20,30

`select` presses until the hull RAM names as selected is `--hull` (`hd_ship_step.step_to_hull`) and exits non-zero
when the screen holds no such hull; `emu-hd-glass-boot.sh` calls it. `timeline` presses buttons and logs, without pausing, every game frame of the screen's fit (`hd_ship_step.py`): the
selection, the model's name and length, the depth target and drawn depth, both slides and the player's six input
terms. `capture` presses `right`, stops the game once the frame count since the selection moved reaches an aim, reads the
state in the stop, grabs the window through `grab_with_clock` and resumes: `<out>/NN.png` and `NN.json` (the frame
count, from and to model, state, adapted luma, the clock sidecar). The Fury backdrop is hidden for the run
(`hd_fury_backdrop.py`), as the rest pair does. `pairs` turns a capture into compare-page pairs, ours drawn with
`--menu-picker-step-seconds` at the same frame count; it needs the lane's RPCS3 config tree (`XDG_CONFIG_HOME`) for the language and the
Safe Area the save holds.
"""
import argparse
import importlib
import json
import os
import sys
import threading
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
MAIN = Path("/home/topaxi/projects/OpenAntiGrav")
sys.path.insert(0, str(HERE))
import hd_fury_backdrop as fb  # noqa: E402
import hd_ship_step as hs  # noqa: E402
import rpcs3_hd_field as field  # noqa: E402

drive = importlib.import_module("rpcs3-drive")
walk = importlib.import_module("rpcs3-hd-menu-walk")

GDB_PORT = int(os.environ.get("OAG_RPCS3_GDB", "127.0.0.1:2345").rsplit(":", 1)[1])
FRAME_SECONDS = 1.0 / 60.0
#: Seconds the window is given to show the frame the stopped game last handed the RSX (`hd_flyer_clock.PAUSE_FLUSH_S`).
FLUSH_S = 0.6
#: Game frames the grab's picture is older than the RAM read in the stop, for the craft. **Fitted, loosely**: of seven fast steps
#: (Icaras to Feisar, Feisar to Qirex, Qirex to Piranha, ...) ours at the original's frame less 3, 4, 5, 3, 3, any, 4 gives the
#: lowest craft-box error, a minimum a half unit deep, so a frame either way is not told apart; 4 is also the card clock's fitted
#: lag (`hd_flyer_clock.PRESENTED_LAG_S`, 0.07 s). The header dots (a 2D widget) are on the selected team from RAM frame 3 on
#: in 71 of 72 grabs on two boots, which reads as a lag of 3 for them if they follow the selection the frame it moves. 3 is the pairs' lag:
#: the craft's cut (`hull_swap::GAP_FRAMES`) starts with the selection in ours and with the dots in the original, so at 3 the first frame the
#: original shows the new craft (RAM frame 6) is the first frame ours does (frame 3).
PICTURE_LAG_FRAMES = 3
#: Frames before the aim at which the stop is asked for: the stub answers a few frames late.
LEAD = 1
TEAMS = ("AG_Systems", "Assegai", "Auricom", "EGX", "Feisar", "Goteki", "Harimau", "Icaras", "Mirage", "Piranha", "Qirex", "Triakis")


def team_of(model_name):
    """`Piranha` for `piranha_c1`."""
    for team in TEAMS:
        if model_name and model_name.lower().startswith(team.lower() + "_"):
            return team
    return None


def attach():
    info = drive.live_session()
    if info is None:
        sys.exit("no live `rpcs3-drive.py serve` (OAG_RPCS3_ATTACH=1, the lane's XDG_CACHE_HOME)")
    mem = drive.RemoteMem(info["socket"])
    return drive.RemotePad(info["socket"]), mem, hs.Screen(mem)


def described(screen, state):
    name = screen.model_name(state["model"])
    return dict(state, name=name, length=round(screen.length(state["model"]), 3))


def attach_when_up(tries=5, wait=2.0):
    """`attach`, retried while the screen is still sliding in (its scan wants the settled words)."""
    for left in range(tries - 1, -1, -1):
        try:
            return attach()
        except LookupError:
            if not left:
                raise
            time.sleep(wait)


def cmd_select(args):
    try:
        pad, mem, screen = attach_when_up()
        guest = field.Mem(mem)

        def read():
            model = screen.state()["model"]
            return model, field.model_hull(guest, model)

        def press(button):
            pad.press(button, 0.15)
            time.sleep(args.settle)

        presses = hs.step_to_hull(read, press, args.hull)
    except LookupError as error:
        sys.exit("select: %s" % error)
    print("Ship Select on %s after %d presses (%s)" % (read()[1], len(presses), " ".join(presses) or "none"))


def cmd_timeline(args):
    pad, mem, screen = attach()
    rows = []
    for button in args.buttons.split(","):
        time.sleep(args.rest)
        counter = hs.Counter()
        first = screen.state()
        counter.feed(first)
        presser = threading.Thread(target=lambda b=button: pad.press(b, 0.15))
        t0 = time.time()
        presser.start()
        last = None
        while time.time() - t0 < args.seconds:
            state = screen.state()
            frame = counter.feed(state)
            if hs.key(state) != last:
                last = hs.key(state)
                rows.append({"button": button, "t": round(time.time() - t0, 4), "frame": frame,
                             **described(screen, state)})
        presser.join()
    Path(args.out).write_text(json.dumps(rows, indent=1) + "\n")
    print("%d rows to %s" % (len(rows), args.out))


def frames_of(counter, post, screen):
    """The frame count of a state read in the stop: a polled row's own, else the polled count plus the frames the
    walking phase of the law needs to reach the read depth (`exact` False beyond the walk)."""
    for frame, state in counter.rows:
        if hs.key(state) == hs.key(post):
            return frame, True
    frame, last = counter.rows[-1]
    depth, target = last["depth"], last["target"]
    for n in range(1, 8):
        depth, target = hs.forward(depth, target, 1)
        if abs(depth - post["depth"]) < 2e-4 and abs(target - post["target"]) < 2e-3:
            return frame + n, True
    return frame + 1, False


def cmd_capture(args):
    pad, mem, screen = attach()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    w, h = (int(v) for v in args.size.split("x"))
    walk.window(w, h)
    if drive.current_screen() != "Team Selection":
        sys.exit("the emulator is on %r, not Team Selection" % drive.current_screen())
    widget = fb.Widget(mem)
    hidden = None if args.show_backdrop else widget.hide()
    if hidden is not None:
        widget.wait_frames(fb.FRAMES_TO_DECAY)
    aims = [int(a) for a in args.aims.split(",")]
    trials = []
    from rpcs3_debugger import Debugger
    try:
        for n in range(args.trials):
            aim = aims[n % len(aims)]
            time.sleep(args.rest)
            before = described(screen, screen.state())
            counter = hs.Counter()
            counter.feed(before)
            dbg = Debugger(port=GDB_PORT)
            dbg.resume()
            time.sleep(0.05)
            presser = threading.Thread(target=lambda: pad.press("right", 0.15))
            t0 = time.time()
            presser.start()
            polled = None
            while time.time() - t0 < 4.0:
                state = screen.state()
                frame = counter.feed(state)
                if frame is not None and frame >= aim - args.lead:
                    polled = frame
                    break
            stopped_at = time.time() - t0
            dbg.pause()
            post = screen.state()
            presser.join()
            record = {"n": n, "aim": aim, "polled_frame": polled, "stopped_after_s": round(stopped_at, 4), "before": before}
            if polled is None or not counter.rows:
                record["failed"] = "the selection never moved"
                dbg.resume()
                dbg.close()
                trials.append(record)
                continue
            frame, exact = frames_of(counter, post, screen)
            time.sleep(FLUSH_S)
            png = out / ("%02d.png" % n)
            walk.grab(png, w, h)
            again = screen.state()
            luma = widget.adapted_luma()
            still_hidden = widget.still_hidden() if hidden is not None else None
            dbg.resume()
            dbg.close()
            record.update({"frame": frame, "frame_exact": exact, "after": described(screen, post),
                           "still": hs.key(again) == hs.key(post), "adapted_luma": luma, "still_hidden": still_hidden,
                           "png": png.name, "rows": [(f, s["target"], s["depth"], s["slide"]) for f, s in counter.rows]})
            (out / ("%02d.json" % n)).write_text(json.dumps(record, indent=1) + "\n")
            trials.append(record)
            print("%02d aim %2d polled %s frame %s%s %s -> %s depth %.3f" % (
                n, aim, polled, frame, "" if exact else "?", before["name"], record["after"]["name"], post["depth"]), flush=True)
    finally:
        if hidden is not None:
            widget.restore()
    (out / "trials.json").write_text(json.dumps({"hidden": hidden, "trials": [
        {k: v for k, v in t.items() if k != "rows"} for t in trials]}, indent=1) + "\n")


def cmd_pairs(args):
    """One compare-page pair per chosen grab: the original's frame at RAM frame `k`, ours at `k - PICTURE_LAG_FRAMES`."""
    import datetime
    import compare_env as ce
    import compare_menu as cm
    import compare_provenance as cp
    import hd_fury_backdrop as fbk
    cap = Path(args.capture)
    doc = json.loads((cap / "trials.json").read_text())
    hidden = doc["hidden"]
    out = Path(args.out)
    plat = ce.platform("hd-fury")
    config = Path(os.environ["XDG_CONFIG_HOME"]) / "rpcs3"
    wanted = [int(v) for v in args.frames.split(",")]
    refused = []
    for k in wanted:
        trial = next((t for t in doc["trials"] if t.get("frame") == k and t.get("still")), None)
        if trial is None:
            print("no still grab at frame %d" % k)
            continue
        team = team_of(trial["after"]["name"])
        frames = k - PICTURE_LAG_FRAMES
        pdir = out / ("menu-ship-step-k%02d" % k)
        pdir.mkdir(parents=True, exist_ok=True)
        pair = ce.Pair("hd-fury", pdir)
        pair.rec["meta"] = {"id": "hd-menu-ship-step-k%02d" % k, "title": "Ship Select, %d game frames after a step to %s" % (k, team),
                            "track": "front end", "craft": "none (a menu)",
                            "note": "Ours is `--menu-page ship-select --menu-picker-step-seconds %.4f`, the screen's step played to frame %d; the "
                                    "original is stopped at RAM frame %d after the selection moved (`st+0x1014/0x1020/0x1018/0x101c`, a frame is a "
                                    "change of that tuple) and the grab's picture is %d frames behind RAM (`PICTURE_LAG_FRAMES`). Both sides English, "
                                    "front-end styling FURY, Fury backdrop hidden on both." % (frames / 60.0, frames, k, PICTURE_LAG_FRAMES),
                            "state": "cold boot, walked to Ship Select, one lap of every team to load the hulls, then right presses"}
        pair.rec["original"]["emulator"] = dict(plat["emulator"])
        grabbed = json.loads((cap / (Path(trial["png"]).stem + ".clock.json")).read_text())["grabbed_at"]
        pair.rec["original"]["captured_at"] = datetime.datetime.strptime(grabbed, "%Y-%m-%dT%H:%M:%S").astimezone().isoformat(timespec="seconds")
        pair.rec["original"]["capture_path"] = "cold boot, walked (no savestate)"
        profile = pdir / "ours-profile" / "profile"
        safe_area = ce.record_safe_area(pair)
        ce.write_ours_settings(plat, profile, safe_area)
        race = {"team": team}
        cm.extra_settings(profile, "FURY", race, locked=True)
        pair.rec["original"]["selection_basis"] = "read from RAM: `st+0x100` and the model's hull entry name (%s)" % trial["after"]["name"]
        pair.rec["original"]["step"] = {
            "frame": k, "frame_exact": trial["frame_exact"], "from": trial["before"]["name"], "to": trial["after"]["name"],
            "to_length": trial["after"]["length"], "state_in_the_stop": {key: trial["after"][key] for key in ("target", "depth", "slide_target", "slide", "inputs", "idle")},
            "picture_lag_frames": PICTURE_LAG_FRAMES, "ours_step_frames": frames,
            "basis": "read: the screen object found by `hd_ship_step.find`, its fit state at the stop (game stopped through the grab, read again after it: %s)"
                     % ("unchanged" if trial["still"] else "CHANGED")}
        args.accept_provenance = list(args.accept_provenance)
        cp.build(pair, args, "menu", "cold_boot", None, {"value": PICTURE_LAG_FRAMES, "unit": "game frames", "basis": "fitted on the craft box of seven fast steps, see `PICTURE_LAG_FRAMES`"})
        extra = ["--menu-picker-seconds", "5", "--menu-picker-step-seconds", "%.5f" % (frames / 60.0)]
        hid = dict(hidden, still_hidden=trial.get("still_hidden", False))
        pair.check("the Fury backdrop is hidden at the grab", bool(hid["still_hidden"]) and hid["frames_waited"] >= fbk.FRAMES_TO_DECAY,
                   "row %s zero at the grab: %s, %d widget frames since the write" % (hid["row"], hid["still_hidden"], hid["frames_waited"]))
        extra += fbk.record(pair.rec, hid, trial["adapted_luma"])
        pair.check("the screen stood still through the grab", bool(trial["still"]), "the fit's state read in the stop and again after the grab")
        try:
            cm.menu_pair(pair, theirs_png=cap / trial["png"], oag_game=args.oag_game, image=args.image, page="ship-select",
                         drawn_from="xml", xml_file="Team_Selection_Definition.xml", screen_seen="Team Selection",
                         screen_wanted="Team Selection", profile_dir=profile, size=ce.size_text(plat), front_end_style="FURY",
                         language="English", original_language=original_language(config), selection={"original": team, "ours": team},
                         extra_args=extra, race=race, safe_area=safe_area)
            pair.finish()
        except SystemExit as exc:
            if exc.code == 3:
                refused.append(k)
                continue
            raise
        print("pair k=%d ok" % k)
    return 3 if refused else 0


def original_language(config_dir):
    for line in (Path(config_dir) / "config.yml").read_text().splitlines():
        if line.strip().startswith("Language:"):
            return line.split(":", 1)[1].strip().split(" ")[0]
    return "unknown"


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("select", help="press until the selected hull RAM names is --hull, or refuse")
    s.add_argument("--hull", required=True, help="the pair's --team, e.g. harimau or Assegai_c1")
    s.add_argument("--settle", type=float, default=1.2, help="seconds after a press before the read")
    s.set_defaults(run=cmd_select)
    t = sub.add_parser("timeline", help="log every game frame of the fit across button presses, unpaused")
    t.add_argument("--buttons", required=True)
    t.add_argument("--out", required=True)
    t.add_argument("--seconds", type=float, default=2.5)
    t.add_argument("--rest", type=float, default=1.5)
    t.set_defaults(run=cmd_timeline)
    c = sub.add_parser("capture", help="stop the game at aimed frames after a step and grab the window")
    c.add_argument("--out", required=True)
    c.add_argument("--aims", default="2,3,4,6,8,12,20,30")
    c.add_argument("--trials", type=int, default=8)
    c.add_argument("--rest", type=float, default=3.0)
    c.add_argument("--lead", type=int, default=LEAD, help="frames before the aim the stop is asked for")
    c.add_argument("--size", default="1920x1080")
    c.add_argument("--show-backdrop", action="store_true")
    c.set_defaults(run=cmd_capture)
    p = sub.add_parser("pairs", help="compare-page pairs from a capture, ours at the same frame less the picture's lag")
    p.add_argument("--capture", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--frames", default="6,10,19")
    p.add_argument("--oag-game", default=str(ROOT / "target/debug/oag-game"))
    p.add_argument("--image", default=str(MAIN / "data/images/hdfury-ps3-eu.iso"))
    import compare_provenance as cp
    cp.add_arguments(p)
    p.set_defaults(run=cmd_pairs)
    args = ap.parse_args()
    return args.run(args)


if __name__ == "__main__":
    sys.exit(main())
