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
import compare_env as ce  # noqa: E402
import rpcs3_place  # noqa: E402


def run(cmd, **kw):
    print("+", " ".join(str(c) for c in cmd), file=sys.stderr)
    return subprocess.run([str(c) for c in cmd], check=True, **kw)


#: Our race's tick at which the lap clock leaves 0 (the countdown's length), measured on HD (see the docs).
GO_TICK = 276


def shutil_copy(src, dst):
    import shutil
    shutil.copyfile(src, dst)


def set_window(w, h):
    """Resize RPCS3's window on the instance's own Xvfb to the profile's output size: the presented
    frame is then the render target 1:1 (the default window is 1760x1072 with black bars)."""
    display = os.environ.get("OAG_RPCS3_DISPLAY")
    if not display:
        sys.exit("OAG_RPCS3_DISPLAY must name the instance's own Xvfb (source its env.sh)")
    env = dict(os.environ, DISPLAY="127.0.0.1:%s" % display)
    wins = subprocess.run(["xdotool", "search", "--onlyvisible", "--name", ""], capture_output=True,
                          text=True, env=env).stdout.split()
    # the largest is the root; the emulator is the largest one that is not the whole screen
    cands = []
    for wid in wins:
        geo = subprocess.run(["xdotool", "getwindowgeometry", "--shell", wid], capture_output=True,
                             text=True, env=env).stdout
        v = dict(line.split("=") for line in geo.split() if "=" in line)
        cands.append((int(v["WIDTH"]) * int(v["HEIGHT"]), wid, int(v["WIDTH"]), int(v["HEIGHT"])))
    cands.sort()
    emu = cands[-2] if len(cands) > 1 else cands[-1]
    if (emu[2], emu[3]) != (w, h):
        subprocess.run(["xdotool", "windowsize", emu[1], str(w), str(h)], env=env, check=True)
        import time
        time.sleep(3.0)


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
    ap.add_argument("--oag-game", default=str(ROOT / "target/debug/oag-game"),
                    help="the build that renders ours; its hash is recorded in pair.json")
    ap.add_argument("--platform", default="hd-fury")
    ap.add_argument("--state", help="the save state the emulator was restored from (mode/circuit facts)")
    ap.add_argument("--meta", default="", help="JSON object for the manifest: id, title, track, craft, state, note")
    ap.add_argument("--ours-ticks", type=int, default=None,
                    help="ticks ours runs before the frame; the states are after GO, so ours runs past its "
                         "own countdown to show the same gantry board (0 keeps the craft exactly on the "
                         "pose but draws the pre-countdown board): see the docs")
    ap.add_argument("--size", default=None, help="default: the profile's output size")
    ap.add_argument("--load", type=float, default=0)
    ap.add_argument("--field", action="store_true",
                    help="a race shot: the state is a mid-race frame of a Single Race, weapons off. Ours takes all eight "
                         "craft (pose, team), the player's pose and camera and the HUD's place, lap, clock, speed and "
                         "shield from the SAME pause as the screenshot (`rpcs3-drive.py capture --field --in-race`), "
                         "renders at that tick (--ticks 0), and the HUD state is imposed. Restore the state with "
                         "`emu-restore-state.sh <lane> <state> --pause` first")
    ap.add_argument("--press", default="none",
                    help="the button held while the two frames are taken; `cross` is throttle")
    args = ap.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    pair = ce.Pair(args.platform, out)
    plat = pair.plat
    size = args.size or ce.size_text(plat)
    scratch = out / "ours-profile"
    ce.write_ours_settings(plat, scratch / "profile")
    os.environ.update(ce.ours_env(scratch / "profile"))
    pair.rec["ours"]["binary"] = ce.binary_record(args.oag_game)
    pair.rec["original"]["emulator"] = dict(plat["emulator"])
    if args.state:
        pair.rec["original"]["state"] = {"file": args.state, **ce.state_facts(args.state)}

    if args.json:
        record_path = args.json
    else:
        set_window(*plat["output"])
        cap = [sys.executable, HERE / "rpcs3-drive.py", "capture", "--out", out / "capture",
               "--shots", 2, "--interval", 0.12, "--load", args.load, "--team", args.team,
               "--press", args.press]
        if args.field:
            cap += ["--field", "--in-race", "--settle", "0"]
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
    clock = ship.get("race_clock_s")
    if args.field:
        args.ours_ticks = 0
    if args.ours_ticks is None:
        if clock is None:
            sys.exit("the capture has no race clock (ship.race_clock_s): pass --ours-ticks")
        args.ours_ticks = GO_TICK + round(clock * 60)
    pair.rec["original"]["race_clock_s"] = clock
    pair.rec["ours"]["ticks"] = args.ours_ticks
    pair.rec["ours"]["race_clock_s"] = (args.ours_ticks - GO_TICK) / 60
    if clock is not None and not args.field:
        pair.check("race clock phase", abs((args.ours_ticks - GO_TICK) / 60 - clock) <= 0.05,
                   "original %.2f s on its lap clock, ours %.2f s (ticks %d, GO at tick %d)"
                   % (clock, (args.ours_ticks - GO_TICK) / 60, args.ours_ticks, GO_TICK))
    shutil_copy(record["screenshot"], out / "theirs.png")
    pair.check_size("original", out / "theirs.png")
    if args.field:
        # the player goes in as slot 0 of the field file, an exact pose: no yaw solve, no tilt left uncarried
        yaw, agree, our_up = 0.0, 1.0, ship["rows"][1]
    else:
        yaw, agree, our_up = solve_yaw(args.oag_game, args.image, track, pos, forward, scratch)
    print("yaw %.3f deg, forward agreement %.6f" % (yaw, agree), file=sys.stderr)
    # What `--pose` cannot carry: the angle between the original's body up and ours (the
    # spline's), i.e. the pitch and roll off the track surface, in degrees.
    theirs_up = ship["rows"][1]
    dot = sum(a * b for a, b in zip(theirs_up, our_up)) / (
        math.dist(theirs_up, (0, 0, 0)) * math.dist(our_up, (0, 0, 0)))
    tilt = math.degrees(math.acos(max(-1.0, min(1.0, dot))))
    pair.rec["ours"]["uncarried_tilt_deg"] = tilt
    cam = [camera["eye"], camera["forward"], camera["up"]]
    numbers = ",".join("%.6f" % v for part in cam for v in part)
    pose = "%.4f,%.4f,%.4f,%.4f" % (pos[0], pos[1], pos[2], yaw)
    flags = ce.ours_flags(plat)
    flags[flags.index("--size") + 1] = size
    mode = pair.rec["original"].get("state", {}).get("mode", "single_race" if args.field else "time_trial")
    flags += ["--mode", mode]
    field_flags = []
    if args.field:
        rows = [{"slot": r["slot"], "player": r["player"], "team": r["team"], "left": r["rows"][0],
                 "up": r["rows"][1], "fwd": r["rows"][2], "pos": r["pos"]} for r in ship["field"]]
        teams = ce.write_field_csv(out / "field.csv", rows, with_player=True)
        hud = ship["hud"]
        player_row = next(r for r in ship["field"] if r["player"])
        pair.rec["original"]["field"] = [{k: r[k] for k in ("slot", "team", "pos", "clock")} for r in ship["field"]]
        pair.rec["original"]["hud"] = {"place": hud["place"], "lap": hud["lap"], "laps": hud["laps"],
                                       "lap_time_s": player_row["clock"], "speed_kmh": hud["speed"], "shield": hud["shield"]}
        pair.check_equal("original field size", len(ship["field"]), 8)
        pair.check("HUD record decodes to one place and lap", hud["place"] is not None,
                   "words %s: place %s lap %s of %d" % (hud["words"], hud["place"], hud["lap"], hud["laps"]))
        if hud["place"] is None:
            pair.finish()
        pair.check("every craft has a team name", all(teams) and bool(player_row["team"]), ", ".join(teams))
        field_flags = ["--field-from", out / "field.csv", "--hud-state", "%d,%d,%.4f,%.4f,%.4f,%.4f" % (
            hud["place"], hud["lap"], player_row["clock"], player_row["clock"], hud["speed"],
            100.0 * hud["shield"] / hud["shield_max"])]
        # ours races with weapons off the way the original's Racebox row was set
        ce.set_ours_weapons_off(scratch / "profile")
        pair.caveat("trails and exhaust history start from nothing in ours (a craft at rest); the HUD's place, lap, "
                    "clock, speed and shield are imposed from the original's RAM (%d/8, lap %d, %.1f s), the best-lap "
                    "line, the pickup row and the sight are ours" % (hud["place"], hud["lap"], player_row["clock"]))
    base = [args.oag_game, args.image, "--race", "--track", track, "--team", args.team.upper(),
            "--pose=" + pose, "--camera-pose=" + numbers, "--camera-fov", "%.4f" % camera["fov_y_deg"]] + flags + field_flags
    if args.field:
        base.remove("--pose=" + pose)
    if args.variant:
        base += ["--variant", args.variant]
    if args.skin:
        base += ["--skin", args.skin]
    pair.rec["ours"].update({"mode": mode, "team": args.team, "variant": args.variant, "flags": [str(c) for c in base[2:]],
                             "pose_arg": pose, "camera_pose": numbers, "yaw_agreement": agree})
    pair.rec["original"]["camera"] = camera
    pair.rec["original"]["ship"] = ship
    # Ours drifts in heading while it sits (about 0.26 deg/s on Talon's, a simulation difference recorded as
    # a lead, the original's craft does not turn), so the pose handed to ours is corrected until the
    # heading MEASURED at the rendered tick (a second, trace-only run of the same command) is the original's.
    ours_csv = out / "ours-trace.csv"

    def trace_at(yaw_deg):
        cmd = [c for c in base]
        cmd[cmd.index("--pose=" + pose)] = "--pose=%.4f,%.4f,%.4f,%.4f" % (pos[0], pos[1], pos[2], yaw_deg)
        run(cmd + ["--ticks", args.ours_ticks + 1, "--trace-out", ours_csv], stdout=subprocess.DEVNULL)
        row = ce.csv_row(ours_csv, args.ours_ticks)
        theirs_f = list(forward)
        ours_f = [row["fwd_x"], row["fwd_y"], row["fwd_z"]]
        ours_u = [row["up_x"], row["up_y"], row["up_z"]]
        cr = (ours_f[1] * theirs_f[2] - ours_f[2] * theirs_f[1], ours_f[2] * theirs_f[0] - ours_f[0] * theirs_f[2],
              ours_f[0] * theirs_f[1] - ours_f[1] * theirs_f[0])
        signed = math.degrees(math.atan2(sum(a * b for a, b in zip(ours_u, cr)), sum(a * b for a, b in zip(ours_f, theirs_f))))
        return signed, cmd

    yaw_now = yaw
    sign = 1.0
    if args.field:
        cmd, signed, corrections = base, 0.0, []
    else:
        signed, cmd = trace_at(yaw_now)
        corrections = [{"yaw": yaw_now, "heading_error_deg": signed}]
    for _ in range(0 if args.field else 4):
        if abs(signed) <= pair.plat["tolerance"]["heading_deg"] / 4:
            break
        trial = yaw_now + sign * signed
        new_signed, new_cmd = trace_at(trial)
        if abs(new_signed) > abs(signed) and sign == 1.0:
            sign = -1.0
            trial = yaw_now + sign * signed
            new_signed, new_cmd = trace_at(trial)
        yaw_now, signed, cmd = trial, new_signed, new_cmd
        corrections.append({"yaw": yaw_now, "heading_error_deg": signed})
    pair.rec["ours"]["yaw_corrections"] = corrections
    pair.rec["ours"]["pose_arg"] = ("exact, slot 0 of field.csv" if args.field
                                    else "%.4f,%.4f,%.4f,%.4f" % (pos[0], pos[1], pos[2], yaw_now))
    pair.rec["ours"]["flags"] = [str(c) for c in cmd[2:]]
    run(cmd + ["--ticks", args.ours_ticks, "--screenshot", out / "ours.png"], stdout=subprocess.DEVNULL)
    run(cmd + ["--ticks", args.ours_ticks + 1, "--trace-out", ours_csv], stdout=subprocess.DEVNULL)
    got, bad = ce.read_back(plat, scratch / "profile")
    pair.rec["ours"]["settings_readback"] = got
    for b in bad:
        pair.check("ours settings", False, b)
    pair.check("ours settings read back", not bad, "%d keys equal the profile" % (len(got) - len(bad)))
    ours_row = ce.csv_row(ours_csv, args.ours_ticks)
    pair.check_pose({"pos": list(pos), "fwd": list(forward)}, ce.pose_of(ours_row),
                    camera["fov_y_deg"], camera["fov_y_deg"])
    pair.check_size("ours", out / "ours.png")
    run(["magick", "montage", out / "theirs.png", out / "ours.png", "-tile", "2x1",
         "-geometry", "+2+2", out / "side.png"])
    if args.meta:
        pair.rec["meta"] = json.loads(args.meta)
    pair.finish()
    print("wrote", out / "side.png")


if __name__ == "__main__":
    main()
