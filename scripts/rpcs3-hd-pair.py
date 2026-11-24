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
import compare_provenance as cp  # noqa: E402

#: Our race's tick at which the race clock leaves 0: `oag_race::COUNTDOWN_TICKS`, read from ours by `compare_env.go_tick` (one
#: length for every title today; it was a hard-coded 276 here until 2026-10-21, 4 ticks late).
GO_TICK = ce.go_tick()

#: The frame's HUD lap clock lags the RAM clock read in the pause by this many ticks. Measured 2026-10-21 (`rerender-all`) on the
#: 13 refreshed grid pairs: the tenths digit the frame shows against `trunc(RAM clock * 10)`: 9 of 13 one tenth low, and every
#: pair's fractional tenth brackets the lag between 0.054 s (modesto-heights shows 13.2 at RAM 13.354) and 0.055 s (anulpha-pass
#: shows 13.9 at RAM 13.955), so 0.055 s = 3.3 ticks; 3 ticks is used. (The RSX has presented what it was handed, `OAG_CAPTURE_FLUSH`;
#: this is the HUD text's own lag, as on PSP where the speed text is one tick older than the picture.)
HUD_LAG_TICKS = 3

# The RSX presents the frames it was handed after the game is paused, so a grab taken at once is older than the
# memory read (the HUD's lap clock lagged RAM's by 0.1 to 0.3 s); 3 s lets it catch up (`rpcs3-drive.py capture`).
os.environ.setdefault("OAG_CAPTURE_FLUSH", "3")
import hd_picture_lag  # noqa: E402
import ps3_pose  # noqa: E402
import rpcs3_place  # noqa: E402


#: `<Weapon type="Autopilot"><Stats time>` in HD's `weaponstats_race.xml`: the timer a player's own pickup arms with.
#: The hunt arms it at 10000 so the Autopilot keeps flying, and the HUD prints that.
AUTOPILOT_AUTHORED_S = 5.0
AUTOPILOT_TIMER_SANE_S = 600.0


def run(cmd, **kw):
    print("+", " ".join(str(c) for c in cmd), file=sys.stderr)
    return subprocess.run([str(c) for c in cmd], check=True, **kw)




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
    def geometry(wid):
        geo = subprocess.run(["xdotool", "getwindowgeometry", "--shell", wid], capture_output=True,
                             text=True, env=env).stdout
        v = dict(line.split("=") for line in geo.split() if "=" in line)
        return int(v["WIDTH"]), int(v["HEIGHT"])

    cands = []
    for wid in wins:
        gw, gh = geometry(wid)
        cands.append((gw * gh, wid, gw, gh))
    cands.sort()
    emu = cands[-2] if len(cands) > 1 else cands[-1]
    if (emu[2], emu[3]) != (w, h):
        import time
        subprocess.run(["xdotool", "windowsize", emu[1], str(w), str(h)], env=env, check=True)
        # On a loaded machine Qt applies the resize a few seconds late (one run captured a 1760x1080 window): wait
        # for the size to land, not for a fixed time.
        for _ in range(60):
            if geometry(emu[1]) == (w, h):
                break
            time.sleep(1.0)
        time.sleep(3.0)
    # The window opens at x=240 on the 2000-pixel screen: a 1920 pixel frame then runs off the screen edge and the
    # capture is clipped to 1760 wide (a refused pair, 2026-10-21). Put it at the origin.
    subprocess.run(["xdotool", "windowmove", emu[1], "0", "0"], env=env, check=True)
    import time
    time.sleep(1.0)


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


def frame_pose(pair, ship):
    """`(position, ticks)`: where the presented frame draws the player's hull, and how far behind the RAM body that is.

    An RPCS3 pause leaves RAM 1 or 2 whole ticks ahead of the frame in the push buffer, for the camera and every craft
    alike (`docs/tools/compare-page.md`, "The skew is the pause, not the original's draw order"). The camera is read off
    the push buffer, so ours' craft goes where the push buffer draws the hull, not at the body. A pause that reads no hull
    matrix, or a hull not a whole number of ticks behind the body along its velocity, refuses the pair. `rows` is the
    hull's own attitude in the same frame (unit rows, `ship["rows"]`'s layout), what the heading check reads."""
    hull = ship.get("hull_pb")
    pair.check("the push buffer's hull matrix for the player was read", hull is not None,
               "no 0.3 to 2.5 scale world matrix loaded 6 times within 15 units of the body (re-grab)")
    if hull is None:
        sys.exit("no hull in the push buffer: the pair would draw ours' craft a tick or two ahead of its camera")
    try:
        ticks, off_axis = ps3_pose.whole_ticks(hull["translation"], ship["pos"], ship["vel"])
        pair.check("the frame's hull is a whole number of ticks behind the body", True,
                   "%d tick(s), %.3f units off the velocity line, hull scale %.3f" % (ticks, off_axis, math.dist(hull["rows"][0], (0, 0, 0))))
    except ValueError as error:
        pair.check("the frame's hull is a whole number of ticks behind the body", False, str(error))
        sys.exit(str(error))
    lag = {"craft_ticks": ticks, "hull_scale": math.dist(hull["rows"][0], (0, 0, 0)), "hull_translation": hull["translation"],
           "ram_body": ship["pos"], "craft_basis": "push-buffer hull world matrix against the RAM body, one pause"}
    rows = [[v / math.dist(r, (0, 0, 0)) for v in r] for r in hull["rows"]]
    return list(hull["translation"]), rows, lag


def picture_lag(pair, camera, ship, theirs, skew, pos):
    """`(pos, eye)` ours is fed: the push buffer's hull and camera moved back the ticks the picture is older than that frame.

    The picture RPCS3 shows is older than the push buffer's frame, which is itself `skew` ticks behind RAM (`frame_pose`). The mines
    in the frame give how much older (`hd_picture_lag.fit`, `docs/tools/compare-page.md`, "The presented lag of a Detonator grab"):
    the camera and the hull move back that many ticks of the camera's own velocity (read off the rig eye in RAM), the hull
    with the camera so its place in the frame is kept. A frame with no fit is left at the push buffer's frame and says so."""
    fit = hd_picture_lag.fit(camera, ship, theirs, skew)
    lag = pair.rec["original"]["presented_lag"]
    lag["picture_fit"] = fit
    lag["picture_ticks"] = fit["ticks"]
    if fit["ticks"] is None:
        lag["picture"] = "not corrected: " + fit["reason"]
        pair.caveat("the picture's lag behind the push buffer's frame was not fitted (%s): ours is drawn at the push buffer's frame, which is "
                    "2 to 4 ticks newer than the picture on the Detonator grabs measured" % fit["reason"])
        return pos, list(camera["eye"])
    k = fit["ticks"]
    velocity = fit["camera_velocity"]
    shift = [v * k for v in velocity]
    lag["picture_total_ticks"] = skew + k
    lag["picture"] = ("fitted: the picture is %.2f ticks older than the push buffer's camera (%d mines, %.1f px rms) and %.2f older than RAM; ours' camera "
                      "and craft are moved back that far along the camera's velocity" % (k, fit["mines"], fit["rms_px"], skew + k))
    pair.check("the picture's lag was fitted from the mines", True, "%.2f ticks behind the push buffer, %d mines, %.1f px rms" % (k, fit["mines"], fit["rms_px"]))
    pair.caveat("the picture is %.2f ticks older than the push buffer's frame (fitted from %d mines, not a constant: 2 to 4 ticks on the grabs measured); "
                "ours' camera and craft are moved back that far, the mines' own bands and the gantry clock are RAM's" % (k, fit["mines"]))
    return [a - b for a, b in zip(pos, shift)], [a - b for a, b in zip(camera["eye"], shift)]


def zone_band_flags(pair, ship):
    """The original's music bands at the pause (`rpcs3-hd-detonator-shots.py`) as ours' `--zone-bands` flags.

    The floor's visualiser reads the held levels (`+0x32f8` of the Environment stage block), the mines' envelope
    the raw ones (`+0x32b8`). A shot without them (an older record) leaves ours silent and says so."""
    bands = ship.get("zone_bands")
    pair.rec["original"]["zone_bands"] = bands
    if not bands:
        pair.caveat("no music bands were read at the pause: ours draws the Detonator floor and mines on a silent spectrum")
        return []
    if ship.get("zone_bands_next_stop"):
        bands["next_stop"] = ship["zone_bands_next_stop"]
    pair.caveat("the floor, dashes, billboards and mine rims are music-driven: ours is fed the original's held bands (`+0x32f8`) and raw "
                "bands (`+0x32b8`) read in the same pause as the frame, held constant over the run; the picture was presented 0 to 3 "
                "ticks before the RAM read and the held levels fall 0.1 a tick, so each band is good to about that; a mine's envelope "
                "in ours has no history and picks its own band")
    csv = lambda values: ",".join("%.6f" % v for v in values)  # noqa: E731
    return ["--zone-bands", csv(bands["held"]), "--zone-bands-raw", csv(bands["raw"])]


def mine_flags(pair, ship):
    """The original's laid mines (`rpcs3_hd_field.detonator_mines`, read in the pause) as ours' `--detonator-mines`.

    The original numbers its track entries from the player's start cursor and ours from the spline's origin, so
    a mine's entry does not transfer; its position does, and ours snaps each to its nearest survey lane
    (`RaceSim::carry_detonator_mines`). A shot without them (an older record) keeps ours' seeded layout and says so."""
    mines = [m for m in ship.get("mines") or [] if m["flags"] & 9 == 9]
    if not mines:
        pair.caveat("no mine objects were read at the pause: ours lays its own seeded mine layout")
        return []
    pair.rec["original"]["detonator"]["mines"] = [
        {"entry": m["entry"], "lane": m["lane"], "position": m["position"], "live": bool(m["live"])} for m in mines]
    pair.caveat("Detonator mines: the original's %d mine objects were read in the pause (track entry, lane, position) and ours lays each on its "
                "survey lane nearest that position (`--detonator-mines`); its bomb and any mine laid later are ours" % len(mines))
    return ["--detonator-mines", ";".join("%.3f,%.3f,%.3f" % tuple(m["position"]) for m in mines)]


def detonator_inputs(pair, out, ship, clock):
    """Ours' `--input-script` for a Detonator frame: idle through the countdown and up to the original's press, SQUARE (our cannon
    button) over the lap-clock window the original's button was held, idle after. Records the window and what is not carried."""
    fire = ship.get("fire")
    lines = ce.detonator_input_lines(fire, GO_TICK)
    pair.rec["original"]["detonator"] = {"fire": fire, "lap_clock_s": clock}
    if fire:
        held = int(lines[1].split()[0])
        pair.rec["ours"]["detonator"] = {"fire_tick_from": int(lines[0].split()[0]), "fire_ticks_held": held,
                                         "button": "square (ours; the original's cannon fired on %s)" % fire.get("button")}
    path = out / "ours.inputs"
    path.write_text("\n".join(lines) + "\n")
    pair.caveat("Detonator: ours runs its own countdown, clock and stage laying (its own cannon rounds from the held button) with the "
                "craft held on the original's pose and camera; the mines are the original's addresses where the shot read them, "
                "else ours' own seeded layout (the placement RNG is not carried)")
    pair.caveat("Detonator: ours' craft is held at rest on the original's pose after every tick (`--pose-hold-ticks`): it never flies into a "
                "mine, its exhaust is cold, and a round leaves at 500 units/s, "
                "not 500 + 3.6 x the craft's speed")
    return ["--input-script", path]


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
    cp.add_arguments(ap)
    ap.add_argument("--ours-ticks", type=int, default=None,
                    help="ticks ours runs before the frame; the states are after GO, so ours runs past its "
                         "own countdown to show the same gantry board (0 keeps the craft exactly on the "
                         "pose but draws the pre-countdown board): see the docs")
    ap.add_argument("--size", default=None, help="default: the profile's output size")
    ap.add_argument("--load", type=float, default=0)
    ap.add_argument("--interval", type=float, default=0.12,
                    help="seconds the game runs between the two shots the camera pick compares; a slow emulator "
                         "that draws no new frame in that time answers `camera null (no frame-unique value ...)`")
    ap.add_argument("--field", action="store_true",
                    help="a race shot: the state is a mid-race frame of a Single Race, weapons off. Ours takes all eight "
                         "craft (pose, team), the player's pose and camera and the HUD's place, lap, clock, speed and "
                         "shield from the SAME pause as the screenshot (`rpcs3-drive.py capture --field --in-race`), "
                         "renders at that tick (--ticks 0), and the HUD state is imposed. Restore the state with "
                         "`emu-restore-state.sh <lane> <state> --pause` first")
    ap.add_argument("--pack-near", type=float, default=45.0,
                    help="with --field: how close to the player a craft in view must be (units); the pair is "
                         "refused when the frame is not a pack")
    ap.add_argument("--keep-autopilot", action="store_true",
                    help="with --field: leave the Autopilot armed for good (the HUD then prints its thousands of seconds) "
                         "instead of writing the authored 5 s back, for a loop that captures again from the same flight")
    ap.add_argument("--pack-min-in-view", type=int, default=2,
                    help="with --field: how many other craft the camera must see; 1 for a craft tucked behind one rival")
    ap.add_argument("--pack-max-dist", type=float, default=160.0, help="with --field: craft this far from the camera count as in view")
    ap.add_argument("--pack-half-fov", type=float, default=45.0,
                    help="with --field: half the angle of the cone around the look direction that counts as in view "
                         "(HD's 16:9 frame is about 100 degrees wide)")
    ap.add_argument("--zone", metavar="ZONE,SCORE[,STAGE]",
                    help="a Zone frame: ours renders at tick 0 on the original's pose and camera with this zone number and score "
                         "(HD's Zone counters are not located in RAM, so the caller reads them off the original's own HUD in the "
                         "paused frame; recorded as read off the frame) and, with STAGE, the colour-grade stage pinned "
                         "(`--zone-stage`)")
    ap.add_argument("--detonator", action="store_true",
                    help="a Detonator race shot (`rpcs3-hd-detonator-shots.py` record, passed with --json): ours runs its own countdown and "
                         "clock from the grid in `--mode detonator` (its auto-speed, its own mines laid at ITS placement, the original's "
                         "Libc_Rand layout not carried), holds the cannon over the same lap-clock window the original's button was held "
                         "(`ship.fire`), and the craft is put back on the original's pose for the frame (`--pose-hold`)")
    ap.add_argument("--picture-lag", choices=("fit", "off"), default="fit",
                    help="with --detonator: `fit` moves ours' camera and craft back the ticks the picture is older than the push buffer's "
                         "camera, fitted from the mines in the frame (`hd_picture_lag.py`); a frame whose mines fit no value is left "
                         "uncorrected and says so. `off` leaves ours at the push buffer's frame")
    ap.add_argument("--press", default="none",
                    help="the button held while the two frames are taken; `cross` is throttle")
    args = ap.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    pair = ce.Pair(args.platform, out)
    plat = pair.plat
    size = args.size or ce.size_text(plat)
    scratch = out / "ours-profile"
    # The original's Safe Area Setting off the save its RPCS3 reads (taken before ours' profile replaces
    # `XDG_CONFIG_HOME` below); ours is drawn at it, the profile's pin being the fallback.
    safe_area = ce.record_safe_area(pair, restored_state=bool(args.state))
    ce.write_ours_settings(plat, scratch / "profile", safe_area)
    # Whether this lane's RPCS3 draws the hull shake: `scripts/rpcs3-hd-no-hull-shake.py` patches it out, read
    # back from the lane's own patch file rather than trusted from a variable, before ours' profile replaces
    # `XDG_CONFIG_HOME` below.
    patch = Path(os.environ.get("XDG_CONFIG_HOME", "/nonexistent")) / "rpcs3" / "patches" / "patch.yml"
    pair.rec["original"]["hull_shake"] = ("patched out (rpcs3-hd-no-hull-shake.py)"
                                          if patch.exists() and "HullShake_Apply" in patch.read_text() else "on")
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
               "--shots", 2, "--interval", args.interval, "--load", args.load, "--team", args.team,
               "--press", args.press]
        if args.field:
            cap += ["--field", "--in-race", "--settle", "0", "--first-at-pause"]
            if not args.keep_autopilot:
                cap += ["--autopilot-timer", AUTOPILOT_AUTHORED_S]
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
    frame_lag, frame_rows = {}, ship["rows"]
    if args.detonator:
        pos, frame_rows, frame_lag = frame_pose(pair, ship)
        forward = frame_rows[2]
    if args.field or args.zone:
        args.ours_ticks = 0
    if args.ours_ticks is None:
        if clock is None:
            sys.exit("the capture has no race clock (ship.race_clock_s): pass --ours-ticks")
        args.ours_ticks = GO_TICK + round(clock * 60) - HUD_LAG_TICKS
    pair.rec["original"]["race_clock_s"] = clock
    pair.rec["original"]["capture_path"] = os.environ.get("OAG_HD_CAPTURE_PATH", "restored savestate (RPCS3 paints garbage on the glass of a restored state)")
    pair.rec["original"]["presented_lag"] = {"hud_clock_ticks": HUD_LAG_TICKS, "picture": "no correction to the picture; on a moving craft the presented frame was measured 0 to 3 ticks older than the RAM read "
                                              "(`hd-pair-lag-scan.py`, docs/tools/compare-page.md, \"A moving HD craft\"); ours' ticks and gantry pin take the HUD lag"}
    pair.rec["original"]["presented_lag"].update(frame_lag)
    note = ship.get("autopilot_timer")
    if args.field and args.keep_autopilot:
        pair.caveat("the player's Autopilot is armed for good (--keep-autopilot): the HUD's pickup row prints its "
                    "thousands of seconds; writing the authored 5 s back froze this game at 1.12 s left, twice")
    elif args.field:
        pair.rec["original"]["autopilot_timer"] = note
        pair.check("no armed-forever Autopilot timer at the stop (the HUD prints it)",
                   note is not None and note["after_s"] <= AUTOPILOT_TIMER_SANE_S,
                   "slot+0x158 read %s before and %s after" % ((note or {}).get("before_s"), (note or {}).get("after_s")))
        if note and note["written"]:
            pair.caveat("the state was saved with the player's Autopilot timer at %.0f s, which the HUD prints; the "
                        "pickup's authored %.1f s was written back after the load and the game ran %.1f s so the HUD "
                        "redraws it: the frame, pose, field and HUD record are from that moment" %
                        (note["before_s"], note["authored_s"], note["run_s"]))
    capture_kind = os.environ.get("OAG_HD_CAPTURE_KIND", "unknown")  # cold_boot | savestate_restore, set by compare-rerender-all
    cp.build(pair, args, "zone" if args.zone else "race" if args.field or args.detonator else "scenery", capture_kind, args.state,
             {"value": HUD_LAG_TICKS, "unit": "ticks", "basis": "measured",
              "what": "HUD clock text vs RAM; the picture is not corrected; on a moving craft it was measured 0 to 3 ticks older than the RAM read (hd-pair-lag-scan.py)"})
    gantry = ship.get("gantry_seconds")
    pair.rec["original"]["gantry_seconds"] = gantry
    world = ship.get("scenery_clock")
    ours_world_offset = None
    if world is not None and clock is not None:
        # Scenery animation time less the HUD lap clock, both read in the pause (the clock runs through the wait at START RACE, so
        # it is per capture). Ours runs `ticks / 60`, which already holds the countdown, so its flag drops `GO_TICK / 60`.
        offset = world["time_s"] - clock
        pair.rec["original"]["world_clock_offset_s"] = offset
        pair.rec["original"]["world_clock_s"] = world["time_s"]
        pair.rec["original"]["world_clock_note"] = ("scenery MeshImporter nodes' animation time (+0xc0) less the lap clock, the most "
                                                    "common value (%d of %d nodes), RAM, same pause as the frame" % (world["nodes"], world["total"]))
        ours_world_offset = offset - GO_TICK / 60 if not (args.field or args.zone) else world["time_s"]
        pair.rec["ours"]["world_clock_offset_s"] = ours_world_offset
    pair.check("the scenery's animation clock was read", world is not None, "%s" % (world,))
    pair.rec["ours"]["ticks"] = args.ours_ticks
    pair.rec["ours"]["race_clock_s"] = (args.ours_ticks - GO_TICK) / 60
    if clock is not None and not args.field and not args.zone:
        pair.check("race clock phase", abs((args.ours_ticks - GO_TICK) / 60 - (clock - HUD_LAG_TICKS / 60)) <= 0.05,
                   "original %.2f s on its lap clock, ours %.2f s (ticks %d, GO at tick %d)"
                   % (clock, (args.ours_ticks - GO_TICK) / 60, args.ours_ticks, GO_TICK))
    shutil_copy(record["screenshot"], out / "theirs.png")
    pair.check_size("original", out / "theirs.png")
    camera_eye = list(camera["eye"])
    if args.detonator and args.picture_lag == "fit":
        pos, camera_eye = picture_lag(pair, camera, ship, out / "theirs.png", frame_lag["craft_ticks"], pos)
    # The width the frame's scene was drawn at (1300 to 1920 on the 1080p path), read off the frame, with the
    # level the game held in the capture's pause beside it (`hd_scene_width`).
    import hd_scene_width
    pair.rec["original"]["scene_width"] = hd_scene_width.measure(out / "theirs.png")
    if ship.get("scene_width"):
        pair.rec["original"]["scene_width"]["at_pause"] = ship["scene_width"]
    if args.field:
        # the player goes in as slot 0 of the field file, an exact pose: no yaw solve, no tilt left uncarried
        yaw, agree, our_up = 0.0, 1.0, ship["rows"][1]
    else:
        yaw, agree, our_up = solve_yaw(args.oag_game, args.image, track, pos, forward, scratch)
    print("yaw %.3f deg, forward agreement %.6f" % (yaw, agree), file=sys.stderr)
    # What `--pose` cannot carry: the angle between the original's body up and ours (the
    # spline's), i.e. the pitch and roll off the track surface, in degrees.
    theirs_up = frame_rows[1]
    dot = sum(a * b for a, b in zip(theirs_up, our_up)) / (
        math.dist(theirs_up, (0, 0, 0)) * math.dist(our_up, (0, 0, 0)))
    tilt = math.degrees(math.acos(max(-1.0, min(1.0, dot))))
    pair.rec["ours"]["uncarried_tilt_deg"] = tilt
    cam = [camera_eye, camera["forward"], camera["up"]]
    numbers = ",".join("%.6f" % v for part in cam for v in part)
    pose = "%.4f,%.4f,%.4f,%.4f" % (pos[0], pos[1], pos[2], yaw)
    flags = ce.ours_flags(plat)
    flags[flags.index("--size") + 1] = size
    mode = "detonator" if args.detonator else pair.rec["original"].get("state", {}).get("mode", "single_race" if args.field else "time_trial")
    flags += ["--mode", mode]
    # The GO board's phase, off the gantry node's own time in the same pause: the race clock alone
    # does not give it, less the presented lag like ours' ticks (`compare_env.gantry_pin`). A board past its windows (the
    # node reads outside every one) keeps ours' own.
    if gantry is not None:
        flags += ["--gantry-seconds", "%.6f" % ce.gantry_pin(gantry, HUD_LAG_TICKS)]
    if ours_world_offset is not None:
        flags += ["--world-clock-offset", "%.3f" % ours_world_offset]
    field_flags = []
    if args.zone:
        zbits = args.zone.split(",")
        zone_n, zone_score = int(zbits[0]), int(zbits[1])
        field_flags = ["--hud-state", "1,1,0,0,0,100,0,%d,%d,0" % (zone_n, zone_score)]
        if len(zbits) > 2:
            field_flags += ["--zone-stage", zbits[2]]
        pair.rec["original"]["zone"] = {"zone": zone_n, "score": zone_score, "stage_pinned": zbits[2] if len(zbits) > 2 else None,
                                        "source": "read off the original's own HUD in the paused frame (HD's Zone counters are not located in RAM)"}
        pair.caveat("Zone state read off the original's HUD by eye (zone %d, score %d), not from RAM; ours runs no ticks, so every effect "
                    "that needs history (exhaust, trail) starts cold; speed readout shows 0 in ours" % (zone_n, zone_score))
    if args.field:
        rows = [{"slot": r["slot"], "player": r["player"], "team": r["team"], "left": r["rows"][0],
                 "up": r["rows"][1], "fwd": r["rows"][2], "pos": r["pos"], "vel": r["vel"]} for r in ship["field"]]
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
        ce.pack_check(pair, hud["place"], camera["eye"], camera["forward"], player_row["pos"],
                      [r["pos"] for r in ship["field"] if not r["player"]], near=args.pack_near,
                      max_dist=args.pack_max_dist, half_fov=args.pack_half_fov, min_in_view=args.pack_min_in_view)
        if pair.failed():
            pair.finish()
        pair.check("every craft has a team name", all(teams) and bool(player_row["team"]), ", ".join(teams))
        # The state decides the hull (`<team>` or classic `<team>_c1`); ours is told which with `--team`, and a
        # sidecar or a copied command line can name the other one.
        pair.check_equal("the player's craft (RAM) is the craft ours draws", player_row["team"].lower(), args.team.lower())
        field_flags = ["--field-from", out / "field.csv", "--field-warmup", "--hud-state", "%d,%d,%.4f,%.4f,%.4f,%.4f" % (
            hud["place"], hud["lap"], player_row["clock"] - HUD_LAG_TICKS / 60, player_row["clock"] - HUD_LAG_TICKS / 60, hud["speed"],
            100.0 * hud["shield"] / hud["shield_max"])]
        # ours races with weapons off the way the original's Racebox row was set
        ce.set_ours_weapons_off(scratch / "profile")
        pair.caveat("every craft's trail and plume in ours are an exhaust-only warm-up (`--field-warmup`): each craft flown on "
                    "a straight path to its captured pose at its captured velocity (RAM, body+0x190), full thrust for all, "
                    "the path and the heading chosen, not measured; the HUD's place, lap, "
                    "clock, speed and shield are imposed from the original's RAM (%d/8, lap %d, %.1f s), the best-lap "
                    "line, the pickup row and the sight are ours" % (hud["place"], hud["lap"], player_row["clock"]))
    detonator_flags = []
    if args.detonator:
        detonator_flags = detonator_inputs(pair, out, ship, clock)
        detonator_flags += zone_band_flags(pair, ship)
        detonator_flags += mine_flags(pair, ship)
    base = [args.oag_game, args.image, "--race", "--track", track, "--team", args.team.upper(),
            "--pose=" + pose, "--camera-pose=" + numbers, "--camera-fov", "%.4f" % camera["fov_y_deg"]] + flags + field_flags + detonator_flags
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
    # A craft resting after GO turns, in the original as in ours (bank-to-yaw, docs/physics/grid-state.md), and the
    # original's pose has already turned for the clock it was read at. Running that clock again from it turned ours
    # twice, so `--pose-hold` puts the craft back on the fed pose for the frame and the last trace row; the clock,
    # the GO board, the record panel and the exhaust still run their ticks. A race shot runs no ticks and needs none.
    ours_csv = out / "ours-trace.csv"
    cmd = base if args.field or args.zone else base + ["--pose-hold"] + (["--pose-hold-ticks"] if args.detonator else [])
    pair.rec["ours"]["pose_hold"] = not (args.field or args.zone)
    pair.rec["ours"]["pose_arg"] = "exact, slot 0 of field.csv" if args.field else pose
    pair.rec["ours"]["flags"] = [str(c) for c in cmd[2:]]
    run(cmd + ["--ticks", args.ours_ticks, "--screenshot", out / "ours.png"], stdout=subprocess.DEVNULL)
    run(cmd + ["--ticks", args.ours_ticks, "--trace-out", ours_csv], stdout=subprocess.DEVNULL)
    got, bad = ce.read_back(plat, scratch / "profile", safe_area)
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
