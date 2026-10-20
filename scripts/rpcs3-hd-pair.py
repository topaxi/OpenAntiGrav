#!/usr/bin/env python3
"""One original-vs-ours pair of Wipeout HD from a race RPCS3 is in, craft and camera matched.

    source data/scratch/<lane>/env.sh
    OAG_RPCS3_ATTACH=1 UV_CACHE_DIR=~/.cache/uv UV_OFFLINE=1 \\
    uv run --with evdev python3 scripts/rpcs3-hd-pair.py --out data/scratch/<lane>/talons \\
        --team harimau [--variant classic] [--ours-ticks 10]

Attached to a `serve` that sits in a Time Trial race (`scripts/emu-restore-state.sh`, then
`rpcs3-drive.py restart` so the TTY lines exist), this runs `rpcs3-drive.py capture` (two
frames, the camera from the push buffer and the player body's pose from the same pause),
then renders our frame with `oag-game --race --pose --camera-pose --camera-fov` from that
record, the way `psp-pulse-pair.py` does for Pulse PSP. Writes `theirs.png`, `ours.png`,
`side.png` and `pair.json` (the capture record plus the numbers handed to ours) under `--out`.

The body pose is carried as `--pose x,y,z,yaw`: ours takes its attitude from the nearest spline
sample plus the yaw, so pitch and roll off the spline are not carried (zero at a grid, small a
few frames after GO). The yaw is solved against `oag-game`'s own attitude at the same place,
by the rule `rpcs3_place` uses for `place`. `--json DIR/00.json` skips the capture and reuses
a record. A pair is only a pair when circuit, craft and skin match: look at `side.png`.
"""

import argparse
import json
import math
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
MAIN = Path("/home/topaxi/projects/OpenAntiGrav")
sys.path.insert(0, str(HERE))
import rpcs3_place  # noqa: E402


def run(cmd, **kw):
    print("+", " ".join(str(c) for c in cmd), file=sys.stderr)
    return subprocess.run([str(c) for c in cmd], check=True, **kw)


def solve_yaw(oag_game, image, track, pos, theirs_forward, scratch):
    """`(yaw, agreement, up)`: the `--pose` yaw (degrees) whose attitude points along
    `theirs_forward`, the dot of the two forwards, and our `up` there."""
    base, up = rpcs3_place.our_attitude(oag_game, image, track, list(pos), scratch)
    cross = (base[1] * theirs_forward[2] - base[2] * theirs_forward[1],
             base[2] * theirs_forward[0] - base[0] * theirs_forward[2],
             base[0] * theirs_forward[1] - base[1] * theirs_forward[0])
    yaw = math.degrees(math.atan2(sum(a * b for a, b in zip(up, cross)),
                                  sum(a * b for a, b in zip(base, theirs_forward))))
    best = None
    for candidate in (yaw, -yaw):
        fwd, up = rpcs3_place.our_attitude(oag_game, image, track, list(pos) + [candidate], scratch)
        agree = sum(a * b for a, b in zip(fwd, theirs_forward))
        if best is None or agree > best[1]:
            best = (candidate, agree, up)
    return best


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--team", required=True, help="our --team, e.g. harimau")
    ap.add_argument("--variant", help="our --variant (a craft's other hull file)")
    ap.add_argument("--skin", help="our --skin")
    ap.add_argument("--track", help="our --track entry; default: the circuit the TTY log named")
    ap.add_argument("--json", type=Path, help="reuse a capture record instead of capturing")
    ap.add_argument("--image", default=str(MAIN / "data/images/hdfury-ps3-eu-dec.iso"))
    ap.add_argument("--oag-game", default=str(MAIN / "target/debug/oag-game"))
    ap.add_argument("--ours-ticks", type=int, default=0,
                    help="ticks ours runs before the frame; 0 keeps the craft on the pose, but the gantry "
                         "board is then unlit (the state is after GO): see the docs")
    ap.add_argument("--size", default="1760x990")
    ap.add_argument("--load", type=float, default=0)
    ap.add_argument("--press", default="none",
                    help="the button held while the two frames are taken; `cross` is throttle")
    args = ap.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    scratch = out / "ours-profile"
    for sub in ("cfg", "data", "state"):
        (scratch / "profile" / sub).mkdir(parents=True, exist_ok=True)

    if args.json:
        record_path = args.json
    else:
        cap = [sys.executable, HERE / "rpcs3-drive.py", "capture", "--out", out / "capture",
               "--shots", 2, "--interval", 0.12, "--load", args.load, "--team", args.team,
               "--press", args.press]
        run(cap)
        record_path = out / "capture" / "00.json"
    record = json.loads(record_path.read_text())
    camera, ship = record.get("camera"), record.get("ship")
    if not camera or not ship:
        sys.exit("capture has no %s (%s)" % ("camera" if not camera else "ship pose",
                                             record.get("camera_reason")))
    track = args.track or record.get("track")
    if not track:
        sys.exit("no circuit: the TTY log named none, pass --track")
    track = track.replace(".rcsmodel", ".vex")
    pos, forward = ship["pos"], ship["rows"][2]
    env = dict(os.environ, XDG_CONFIG_HOME=str(scratch / "profile/cfg"),
               XDG_DATA_HOME=str(scratch / "profile/data"),
               XDG_STATE_HOME=str(scratch / "profile/state"))
    os.environ.update(env)
    yaw, agree, our_up = solve_yaw(args.oag_game, args.image, track, pos, forward, scratch)
    print("yaw %.3f deg, forward agreement %.6f" % (yaw, agree), file=sys.stderr)
    # What `--pose` cannot carry: the angle between the original's body up and ours (the
    # spline's), i.e. the pitch and roll off the track surface, in degrees.
    theirs_up = ship["rows"][1]
    dot = sum(a * b for a, b in zip(theirs_up, our_up)) / (
        math.dist(theirs_up, (0, 0, 0)) * math.dist(our_up, (0, 0, 0)))
    tilt = math.degrees(math.acos(max(-1.0, min(1.0, dot))))
    print("attitude not carried (body up vs spline up): %.3f deg" % tilt, file=sys.stderr)
    cam = [camera["eye"], camera["forward"], camera["up"]]
    numbers = ",".join("%.6f" % v for part in cam for v in part)
    pose = "%.4f,%.4f,%.4f,%.4f" % (pos[0], pos[1], pos[2], yaw)
    cmd = [args.oag_game, args.image, "--no-audio", "--race", "--track", track,
           "--team", args.team.upper(), "--pose=" + pose, "--camera-pose=" + numbers,
           "--camera-fov", "%.4f" % camera["fov_y_deg"], "--ticks", args.ours_ticks,
           "--size", args.size, "--screenshot", out / "ours.png"]
    if args.variant:
        cmd += ["--variant", args.variant]
    if args.skin:
        cmd += ["--skin", args.skin]
    run(cmd, stdout=subprocess.DEVNULL)
    run(["magick", record["screenshot"], out / "theirs.png"])
    run(["magick", "montage", out / "theirs.png", out / "ours.png", "-tile", "2x1",
         "-geometry", "+2+2", out / "side.png"])
    (out / "pair.json").write_text(json.dumps(
        {"capture": record, "ours": {"pose": pose, "camera_pose": numbers, "yaw_agreement": agree,
                                     "uncarried_tilt_deg": tilt,
                                     "command": [str(c) for c in cmd]}}, indent=2) + "\n")
    print("wrote", out / "side.png")


if __name__ == "__main__":
    main()
