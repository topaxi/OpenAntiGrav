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
from rpcs3_hd_field import MINE_DRAW_WORDS  # noqa: E402

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


def picture_lag(pair, camera, ship, theirs, skew, pos, mode="fit", render=None):
    """`(pos, eye)` ours is fed: the push buffer's hull and camera moved back the ticks the picture is older than that frame.

    The picture RPCS3 shows is older than the push buffer's frame, which is itself `skew` ticks behind RAM (`frame_pose`). The mines
    in the frame give how much older (`hd_picture_lag.fit`, `docs/tools/compare-page.md`, "The presented lag of a Detonator grab"):
    the camera and the hull move back that many ticks of the camera's own velocity (read off the rig eye in RAM), the hull
    with the camera so its place in the frame is kept. A frame with no fit is left at the push buffer's frame and says so."""
    fit = hd_picture_lag.fit(camera, ship, theirs, skew)
    lag = pair.rec["original"]["presented_lag"]
    lag["picture_fit"] = fit
    k, velocity, basis = fit["ticks"], fit["camera_velocity"], "mines"
    static = None
    if mode == "static" or k is None:
        reason = fit["reason"] or "the static scene was asked for"
        if velocity is not None and render is not None:
            static = hd_picture_lag.static_fit(theirs, lambda at: render(at, velocity))
            lag["picture_static_fit"] = static
            k, basis = static["ticks"], "static scene"
            if k is None:
                reason = "%s; %s" % (reason, static["reason"])
        else:
            k = None
            if velocity is not None:
                reason += "; no static fit without the scenery's animation clock"
    if k is None:
        lag["picture_ticks"] = None
        lag["picture"] = "not corrected: " + reason
        pair.caveat("the picture's lag behind the push buffer's frame was not fitted (%s): ours is drawn at the push buffer's frame, which is "
                    "2 to 4 ticks newer than the picture on the Detonator grabs measured" % reason)
        return pos, list(camera["eye"])
    shift = [v * k for v in velocity]
    lag["picture_ticks"] = k
    lag["picture_basis"] = basis
    lag["picture_total_ticks"] = skew + k
    if basis == "mines":
        how = "%d mines, %.1f px rms" % (fit["mines"], fit["rms_px"])
    else:
        how = "the static scene's edges at their best of %d lags, read 0.25 to 0.6 ticks low where the mines also fit" % len(static["curve"])
    lag["picture"] = ("fitted: the picture is %.2f ticks older than the push buffer's camera (%s) and %.2f older than RAM; ours' camera "
                      "and craft are moved back that far along the camera's velocity" % (k, how, skew + k))
    pair.check("the picture's lag was fitted", True, "%.2f ticks behind the push buffer (%s)" % (k, how))
    pair.caveat("the picture is %.2f ticks older than the push buffer's frame (fitted from %s, not a constant: 2 to 4 ticks on the grabs "
                "measured); ours' camera and craft are moved back that far and its tick is the picture's; the music bands are RAM's" % (k, how))
    return [a - b for a, b in zip(pos, shift)], [a - b for a, b in zip(camera["eye"], shift)]


def static_renderer(args, camera, ship, skew, pos, rows, track, flags, clock, scratch):
    """`render(k, velocity)` for `hd_picture_lag.static_fit`: ours' frame with camera and craft `k` ticks behind the push
    buffer's, at that picture's tick, on the original's mines, with no fire (the score leaves out what moves)."""
    def render(k, velocity):
        eye = [a - k * v for a, v in zip(camera["eye"], velocity)]
        at = [a - k * v for a, v in zip(pos, velocity)]
        field = scratch / "lag-field.csv"
        ce.write_field_csv(field, [{"player": True, "team": args.team, "left": rows[0], "up": rows[1], "fwd": rows[2], "pos": at,
                                    "vel": [0.0, 0.0, 0.0]}], with_player=True)
        shot = scratch / ("lag-%.2f.png" % k)
        numbers = ",".join("%.6f" % v for part in (eye, camera["forward"], camera["up"]) for v in part)
        mines = [m for m in ship.get("mines") or [] if m["flags"] & 9 == 9]
        extra = ["--detonator-mines", ";".join("%.3f,%.3f,%.3f" % tuple(m["position"]) for m in mines)] if mines else []
        bands = ship.get("zone_bands")
        if bands:
            extra += ["--zone-bands", ",".join("%.6f" % v for v in bands["held"]),
                      "--zone-bands-raw", ",".join("%.6f" % v for v in bands["raw"])]
        run([args.oag_game, args.image, "--race", "--track", track, "--team", args.team.upper(), "--camera-pose=" + numbers,
             "--camera-fov", "%.4f" % camera["fov_y_deg"], *flags, "--mode", "detonator", "--field-from", field, "--pose-hold",
             "--pose-hold-ticks", *extra, "--ticks", ce.picture_tick(GO_TICK, clock, skew + k), "--screenshot", shot],
            stdout=subprocess.DEVNULL)
        return shot
    return render


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


#: How far a mine read's RAM clock may sit from the picture's clock and still stand for it: an envelope steps once a tick.
PICTURE_READ_TOLERANCE_TICKS = 1.5


def picture_read(pair, ship):
    """Each mine's draw words (by object address) from the shot's read nearest the picture's clock, `{}` when none is near.

    The picture is `picture_total_ticks` older than the RAM read of its own pause, and a mine's envelope steps once a tick, so
    an attack between the two (every spike `t` 0.57 -> 1.0 in one tick, `detonator-mine-draw.md`, "The envelope at the
    picture's tick") would put the RAM's envelope on a picture drawn before it. Records which read stood for the picture."""
    reads = ship.get("mine_reads") or []
    total = (pair.rec["original"].get("presented_lag") or {}).get("picture_total_ticks")
    det = pair.rec["original"].setdefault("detonator", {})
    if not reads or total is None:
        det["mine_read"] = {"source": "the pause's own read: %s" % ("no earlier reads in the shot record" if not reads
                                                                    else "the picture's lag was not fitted")}
        return {}
    at = ship["race_clock_s"] - total / 60.0
    read = min(reads, key=lambda r: abs(r["race_clock_s"] - at))
    off = (read["race_clock_s"] - at) * 60.0
    near = abs(off) <= PICTURE_READ_TOLERANCE_TICKS
    det["mine_read"] = {"picture_clock_s": at, "read_clock_s": read["race_clock_s"], "ticks_off": off,
                        "source": "the read nearest the picture's clock" if near else
                                  "the pause's own read: no read within %.1f ticks of the picture's clock" % PICTURE_READ_TOLERANCE_TICKS}
    if not near:
        pair.caveat("no mine read lies within %.1f ticks of the picture's clock (%.3f s, nearest %.3f s): the mines' envelopes are the "
                    "pause's own, %.1f ticks newer than the picture" % (PICTURE_READ_TOLERANCE_TICKS, at, read["race_clock_s"], total))
        return {}
    pair.caveat("each mine's envelope and draw words are the read at %.3f s, %.1f ticks from the picture's clock %.3f s (the pause's own "
                "RAM is %.1f ticks newer than the picture)" % (read["race_clock_s"], off, at, total))
    return read["mines"]


def mine_flags(pair, ship):
    """The original's laid mines (`rpcs3_hd_field.detonator_mines`, read in the pause) as ours' `--detonator-mines`.

    The original numbers its track entries from the player's start cursor and ours from the spline's origin, so
    a mine's entry does not transfer; its position does, and ours snaps each to its nearest survey lane
    (`RaceSim::carry_detonator_mines`). A shot without them (an older record) keeps ours' seeded layout and says so."""
    mines = [m for m in ship.get("mines") or [] if m["flags"] & 9 == 9]
    if not mines:
        pair.caveat("no mine objects were read at the pause: ours lays its own seeded mine layout")
        return []
    at_picture = picture_read(pair, ship)
    mines = [dict(m, **at_picture.get(str(m.get("address")), {})) for m in mines]
    # A mine's age is a clock, so with no read at the picture's clock the pause's age less the measured lag is the picture's.
    total = (pair.rec["original"].get("presented_lag") or {}).get("picture_total_ticks")
    if not at_picture and total is not None and any(m.get("age_s") is not None for m in mines):
        mines = [m if m.get("age_s") is None else dict(m, age_s=m["age_s"] - total / 60) for m in mines]
        pair.caveat("each mine's age is the pause's less the picture's %.1f ticks of lag (the age is a clock); its envelope is the pause's own" % total)
    pair.rec["original"]["detonator"]["mines"] = [
        {"entry": m["entry"], "lane": m["lane"], "position": m["position"], "live": bool(m["live"]),
         "football_s": m.get("football_s"), "spikes_t": m.get("spikes_t"), "age_s": m.get("age_s"),
         **{name: m[name] for name, _ in MINE_DRAW_WORDS if name in m}} for m in mines]
    pair.caveat("Detonator mines: the original's %d mine objects were read in the pause (track entry, lane, position) and ours lays each on its "
                "survey lane nearest that position (`--detonator-mines`); its bomb and any mine laid later are ours" % len(mines))
    flags = ["--detonator-mines", ";".join("%.3f,%.3f,%.3f" % tuple(m["position"]) for m in mines)]
    # Without the envelopes ours' near mines draw dark, unspiked and about half the original's size: quiet raw bands never
    # attack an envelope with no history (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine-draw.md`, "The spikes in a pair").
    carried = all(m.get("football_s") is not None and m.get("spikes_t") is not None for m in mines)
    pair.check("each live mine's music envelope was read", carried or not any(m["live"] for m in mines),
               "football s and spikes t of %d mines" % len(mines) if carried
               else "a shot record without `football_s`/`spikes_t`: re-grab it with `rpcs3-hd-detonator-shots.py`")
    if carried:
        # A third value is the ring's `PitchOffset` (`+0x108`), an accumulator ours has no start for; an older record has none.
        flags += ["--detonator-mine-envelopes", ";".join(
            "%.4f,%.4f" % (m["football_s"], m["spikes_t"]) + ("" if m.get("pitch_offset") is None else ",%.4f" % m["pitch_offset"])
            for m in mines)]
        pair.caveat("each mine's music envelope (`+0xf4`, `+0xf8`) and, where read, its ring's `PitchOffset` (`+0x108`) are held in ours "
                    "(`--detonator-mine-envelopes`): ours' own have no history of the music before the frame")
    # A mine turns about its up by 4 x its age (`detonator-mine-draw.md`, "The spin"); ours' own age counts from its own laying.
    if all(m.get("age_s") is not None for m in mines):
        flags += ["--detonator-mine-ages", ";".join("%.4f" % m["age_s"] for m in mines)]
        pair.caveat("each mine's age (`+0xc0`), which turns it about its up axis, is held in ours (`--detonator-mine-ages`)")
    return flags


def detonator_inputs(pair, out, ship, clock):
    """Ours' `--input-script` for a Detonator frame: idle through the countdown and up to the original's press, SQUARE (our cannon
    button) over the lap-clock window the original's button was held, idle after. Records the window and what is not carried."""
    fire = ship.get("fire")
    lines = ce.detonator_input_lines(fire, GO_TICK)
    pair.rec["original"]["detonator"] = {"fire": fire, "lap_clock_s": clock}
    if fire:
        held = sum(int(line.split()[0]) for line in lines if line.endswith("square"))
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
    flags = ["--input-script", path]
    if fire and fire.get("volley_clocks_s"):
        pair.caveat("each volley is placed so ours' round has the original's age (lap clock less the round's age: first at %.3f s, "
                    "%.1f ticks after the harness sent the press; the game's own frame that fired it is up to a few ticks later)"
                    % (fire["first_launch_clock_s"], fire["volley_after_send_ticks"]))
    vel = ship.get("vel")
    if vel:
        speed = sum(c * c for c in vel) ** 0.5
        flags += ["--detonator-gauge-speed", "%.3f" % speed]
        pair.caveat("Detonator: the cannon gauge's speed gate reads the original's speed at the frame (`--detonator-gauge-speed`, "
                    "%.1f): ours' craft is held at rest, where the gauge never updates and stays the bind's full red" % speed)
    return flags


#: A Zone race flies the mode's own hull whatever team was picked: RAM names its directory `Data\Ships\Zone`, and ours draws it
#: under `--mode zone`.
ZONE_HULL = "zone"


def hull_check(pair, ship, team, live):
    """Refuse a pair whose original flies another hull than `--team` names. The parity checks pass on a wrong hull (the
    pose, camera and clocks do not depend on it), so the player's hull directory read in the capture's pause
    (`ship.team`: `Harimau`, or the Fury `Harimau_c1`) is compared with it. A live capture must have read it; a reused
    `--json` record made before the field existed is checked only when it holds one."""
    ram = ship.get("team")
    pair.rec["original"]["team"] = ram
    if ram is None:
        return pair.check("the player's hull was read from RAM", not live, "no `team` in the capture record")
    return pair.check_equal("the player's hull (RAM) is the hull ours draws", ram.lower(), team.lower())


def pilot_assist_check(pair, ship, profile_value, live):
    """Refuse a pair whose original draws the Pilot Assist indicator ours is not told to draw, or the reverse. The option is
    a save setting (`ship.pilot_assist`, the byte read in the capture's pause, 0 is off): a save with it on puts the
    indicator's bracket under the craft of every race frame. Checked like the hull: a live capture must have read it."""
    byte = ship.get("pilot_assist")
    pair.rec["original"]["pilot_assist_byte"] = byte
    if profile_value is None:
        return True
    if byte is None:
        return pair.check("the original's Pilot Assist option was read from RAM", not live, "no `pilot_assist` in the capture record")
    return pair.check_equal("the original's Pilot Assist option is ours'", "off" if byte == 0 else "on", profile_value)


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
    ap.add_argument("--picture-lag", choices=("fit", "static", "off"), default="fit",
                    help="with --detonator: `fit` moves ours' camera and craft back the ticks the picture is older than the push buffer's "
                         "camera, and runs ours to the picture's tick: fitted from the mines in the frame, else from the static scene "
                         "(`hd_picture_lag.py`); a frame that fits neither is left uncorrected and says so. `static` uses the static scene "
                         "only. `off` leaves ours at the push buffer's frame")
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
    # A field pair checks the hull against its field row below; a wrong hull or Pilot Assist is refused before ours renders.
    if not args.field and not hull_check(pair, ship, ZONE_HULL if args.zone else args.team, live=not args.json):
        pair.finish()
    if not pilot_assist_check(pair, ship, plat["ours"].get("pilot_assist"), live=not args.json):
        pair.finish()
    gantry = ship.get("gantry_seconds")
    pair.rec["original"]["gantry_seconds"] = gantry
    flame = ship.get("flame_clock_s")
    pair.rec["original"]["flame_clock_s"] = flame
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
    shutil_copy(record["screenshot"], out / "theirs.png")
    pair.check_size("original", out / "theirs.png")
    camera_eye = list(camera["eye"])
    total_lag = None
    if args.detonator and args.picture_lag != "off":
        if clock is None:
            sys.exit("the capture has no race clock (ship.race_clock_s)")
        lag_flags = ce.ours_flags(plat)
        lag_flags[lag_flags.index("--size") + 1] = size
        if ours_world_offset is not None:
            lag_flags += ["--world-clock-offset", "%.3f" % ours_world_offset]
        skew = frame_lag["craft_ticks"]
        # The static scene's floor dashes and billboards run on the world clock: without it the static fit has read 2.4 ticks off.
        render = None if ours_world_offset is None else static_renderer(args, camera, ship, skew, pos, frame_rows, track,
                                                                         lag_flags, clock, scratch)
        pos, camera_eye = picture_lag(pair, camera, ship, out / "theirs.png", skew, pos, args.picture_lag, render)
        total_lag = pair.rec["original"]["presented_lag"].get("picture_total_ticks")
        if total_lag is not None:
            args.ours_ticks = ce.picture_tick(GO_TICK, clock, total_lag)
    clock_lag = HUD_LAG_TICKS if total_lag is None else total_lag
    pair.rec["ours"]["ticks"] = args.ours_ticks
    pair.rec["ours"]["race_clock_s"] = (args.ours_ticks - GO_TICK) / 60
    if clock is not None and not args.field and not args.zone:
        pair.check("race clock phase", abs((args.ours_ticks - GO_TICK) / 60 - (clock - clock_lag / 60)) <= 0.05,
                   "original %.2f s on its lap clock, ours %.2f s (ticks %d, GO at tick %d)"
                   % (clock, (args.ours_ticks - GO_TICK) / 60, args.ours_ticks, GO_TICK))
    # The width the frame's scene was drawn at (1300 to 1920 on the 1080p path), read off the frame, with the
    # level the game held in the capture's pause beside it (`hd_scene_width`).
    import hd_scene_width
    pair.rec["original"]["scene_width"] = hd_scene_width.measure(out / "theirs.png")
    if ship.get("scene_width"):
        pair.rec["original"]["scene_width"]["at_pause"] = ship["scene_width"]
    if args.field or args.detonator:
        # the player goes in as slot 0 of a field file, an exact pose: no yaw solve, no tilt left uncarried
        yaw, agree, our_up = 0.0, 1.0, (frame_rows if args.detonator else ship["rows"])[1]
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
        flags += ["--gantry-seconds", "%.6f" % ce.gantry_pin(gantry, clock_lag)]
    if ours_world_offset is not None:
        flags += ["--world-clock-offset", "%.3f" % ours_world_offset]
    # The engine flame's noise phase, off the frame context's boot clock in the same pause (`compare_env.flame_pin`).
    if flame is not None:
        flags += ["--flame-clock", "%.4f" % ce.flame_pin(flame)]
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
    if args.detonator and not args.field:
        # `--pose` carries a yaw only, so ours' hull sat on the spline's attitude: 0.7 degrees flatter than the original's body
        # (forward y 0.030 against 0.041 to 0.047) while ours' own free flight reads 0.041 at the same place
        # (`docs/tools/compare-page.md`, "The pitch the heading check refused"). The push buffer's hull attitude goes in as slot 0 of
        # a field file: an exact pose, nothing left uncarried.
        row = {"player": True, "team": args.team, "left": frame_rows[0], "up": frame_rows[1], "fwd": frame_rows[2],
               "pos": pos, "vel": [0.0, 0.0, 0.0]}
        ce.write_field_csv(out / "field.csv", [row], with_player=True)
        pair.rec["ours"]["pose_row"] = {k: row[k] for k in ("team", "left", "up", "fwd", "pos")}
        pair.rec["ours"]["pose_row"]["source"] = ("the push buffer's hull attitude (`original.ship.hull_pb`), at the position ours' craft is "
                                                  "put (lag-fitted when the picture lag was)")
        base.remove("--pose=" + pose)
        base += ["--field-from", out / "field.csv"]
    if args.variant:
        base += ["--variant", args.variant]
    if args.skin:
        base += ["--skin", args.skin]
    pair.rec["ours"].update({"mode": mode, "team": args.team, "variant": args.variant, "flags": [str(c) for c in base[2:]],
                             "pose_arg": pose, "camera_pose": numbers, "yaw_agreement": agree})
    pair.rec["original"]["camera"] = camera
    # The shot's every mine read and each object's bytes stay in the shot record: the pair keeps the read it used
    # (`original.detonator.mine_read`, `original.detonator.mines`).
    pair.rec["original"]["ship"] = dict(ship, mine_reads=len(ship.get("mine_reads") or []),
                                        mines=[{k: v for k, v in m.items() if k != "raw"} for m in ship.get("mines") or []])
    # A craft resting after GO turns, in the original as in ours (bank-to-yaw, docs/physics/grid-state.md), and the
    # original's pose has already turned for the clock it was read at. Running that clock again from it turned ours
    # twice, so `--pose-hold` puts the craft back on the fed pose for the frame and the last trace row; the clock,
    # the GO board, the record panel and the exhaust still run their ticks. A race shot runs no ticks and needs none.
    ours_csv = out / "ours-trace.csv"
    cmd = base if args.field or args.zone else base + ["--pose-hold"] + (["--pose-hold-ticks"] if args.detonator else [])
    pair.rec["ours"]["pose_hold"] = not (args.field or args.zone)
    pair.rec["ours"]["pose_arg"] = "exact, slot 0 of field.csv" if args.field or args.detonator else pose
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
