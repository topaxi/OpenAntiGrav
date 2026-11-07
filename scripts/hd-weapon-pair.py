#!/usr/bin/env python3
"""Ours beside the original for a weapon in flight: the frames `rpcs3-hd-weapon.py --shot-at` captured.

    python3 scripts/hd-weapon-pair.py --capture DIR --weapon missile --team feisar [--fire-clock S] [--out DIR/pair]

`DIR/NN.png` and `NN.json` are one paused frame each (the original's frame, the camera read out of the RSX
pushbuffer, the player's pose and the lap clock, plus the clock window the shot happened in). For every one this
renders ours at the same craft pose and camera (`--pose`, `--camera-pose`, `--camera-fov`, `--pose-hold`), the
weapon given (`--give`) and fired at the same lap-clock second, `ticks = GO + round(clock * 60) - HUD_LAG`, and
writes `ours_NN.png`, `theirs_NN.png` and `side_NN.png`. The craft rests on its grid slot in both, so the shot
leaves from the same place; the shot's second is only known to the press window, so `--fire-clock` places it
inside that window (default: the window's end) and a sweep over it is how the phase is chosen.
"""
import argparse
import importlib.util
import json
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402

MAIN = Path("/home/topaxi/projects/OpenAntiGrav")
HUD_LAG_TICKS = 3


def load_module(name, file):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def run(cmd):
    print("+", " ".join(str(c) for c in cmd), file=sys.stderr)
    subprocess.run([str(c) for c in cmd], check=True, stdout=subprocess.DEVNULL)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--capture", type=Path, required=True)
    ap.add_argument("--weapon", required=True, help="our --give name")
    ap.add_argument("--team", required=True)
    ap.add_argument("--out", type=Path)
    ap.add_argument("--fire-clock", type=float, help="lap-clock second the shot left; default: the press window's end")
    ap.add_argument("--button", default="square", help="ours' fire button")
    ap.add_argument("--image", default=str(MAIN / "data/images/hdfury-ps3-eu-dec.iso"))
    ap.add_argument("--oag-game", default=str(HERE.parent / "target/debug/oag-game"))
    ap.add_argument("--platform", default="hd-fury")
    ap.add_argument("--camera-from", help="a frame number whose camera stands in for frames the pushbuffer read gave none "
                    "(the craft rests, so the camera does not move between frames)")
    ap.add_argument("--only", help="comma separated frame numbers")
    ap.add_argument("--extra", nargs=argparse.REMAINDER, default=[], help="flags passed to ours as they are")
    args = ap.parse_args()
    out = (args.out or args.capture / "pair").resolve()
    out.mkdir(parents=True, exist_ok=True)
    plat = ce.platform(args.platform)
    size = ce.size_text(plat)
    go = ce.go_tick(plat)
    scratch = out / "ours-profile"
    ce.write_ours_settings(plat, scratch, None)
    os.environ.update(ce.ours_env(scratch))
    pair = load_module("rpcs3_hd_pair", "rpcs3-hd-pair.py")
    wanted = set(args.only.split(",")) if args.only else None
    for record_path in sorted(args.capture.glob("[0-9][0-9].json")):
        stem = record_path.stem
        if wanted and stem not in wanted:
            continue
        record = json.loads(record_path.read_text())
        camera, ship = record.get("camera"), record.get("ship")
        if not camera and args.camera_from:
            camera = json.loads((args.capture / ("%s.json" % args.camera_from)).read_text()).get("camera")
        if not camera or not ship:
            print("%s: no camera or pose (%s), skipped" % (stem, record.get("camera_reason")))
            continue
        track = record["track"].replace(".rcsmodel", ".vex")
        pos, forward = ship["pos"], ship["rows"][2]
        yaw, agree, _ = pair.solve_yaw(args.oag_game, args.image, track, pos, forward, scratch)
        fire_clock = args.fire_clock if args.fire_clock is not None else ship["fire_clock_after_s"]
        fire_tick = go + round(fire_clock * 60)
        ticks = go + round(ship["race_clock_s"] * 60) - HUD_LAG_TICKS
        script = out / ("fire_%s.inputs" % stem)
        script.write_text("%d none\n1 %s\n600 none\n" % (fire_tick, args.button))
        cam = [camera["eye"], camera["forward"], camera["up"]]
        numbers = ",".join("%.6f" % v for part in cam for v in part)
        flags = ce.ours_flags(plat)
        flags[flags.index("--size") + 1] = size
        cmd = [args.oag_game, args.image, "--race", "--track", track, "--team", args.team.upper(), "--opponents",
               "--mode", "single_race", "--give", args.weapon, "--input-script", script,
               "--pose=%.4f,%.4f,%.4f,%.4f" % (pos[0], pos[1], pos[2], yaw), "--camera-pose=" + numbers,
               "--camera-fov", "%.4f" % camera["fov_y_deg"], "--pose-hold"] + flags + args.extra
        run(cmd + ["--ticks", ticks, "--screenshot", out / ("ours_%s.png" % stem)])
        subprocess.run(["cp", record["screenshot"], str(out / ("theirs_%s.png" % stem))], check=True)
        run(["magick", "montage", out / ("theirs_%s.png" % stem), out / ("ours_%s.png" % stem), "-tile", "2x1",
             "-geometry", "+2+2", out / ("side_%s.png" % stem)])
        print("%s: clock %.3f, ours tick %d, shot at tick %d (clock %.3f), yaw %.3f agree %.6f"
              % (stem, ship["race_clock_s"], ticks, fire_tick, fire_clock, yaw, agree))


if __name__ == "__main__":
    main()
