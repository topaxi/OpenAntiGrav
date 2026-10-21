#!/usr/bin/env python3
"""One original-vs-ours pair of Wipeout Pulse (PSP) from a race the emulator is in, environment-checked.

    OAG_PPSSPP_PORT=47897 source <lane>/pp/env.sh       # DISPLAY, PP_HOME (scripts/ppsspp-start.sh)
    DISPLAY=127.0.0.1:97 OAG_SHOT_DISPLAY=127.0.0.1:97 HOME=<instance HOME> XDG_CONFIG_HOME=<scratch>/profile/cfg \\
    uv run --with websocket-client python3 scripts/psp-pulse-pair.py \\
        --port 47897 --out data/scratch/<lane>/moa --track 03_Track --state data/saves/pulse-psp/<point>.ppst \\
        [--reversed] [--team Feisar]

The environment is `scripts/compare-environment.toml`, platform `pulse-psp`: the PPSSPP instance must have
been started with its `ppsspp-ini` text (`PPSSPP_EXTRA_INI`, see ppsspp-start.sh); the 960x544 window the pause
menu needs to load a state is resized to the native 480x272 afterwards, so the frame is the console's own
size with nothing scaled. Ours is rendered at 480x272 with the profile's flags and settings file, the original's
camera and its speed-widened fov, after the countdown and the original's race clock, with `--pose-hold` putting
the craft back on the original's pose for the frame (a resting craft turns in both, so running the clock from
an already-turned pose turned ours twice). A second, `--trace-out` run checks that pose landed: an import check
of the basis, convention and fov, not of the simulation (`scripts/compare-resting-yaw.py` measures that). A pair that differs from the profile
(image size, settings read back, layout file, mode, class, pose, fov) is refused with the mismatch named, and
`pair.json` records both sides.
"""

import argparse
import os
import subprocess
import sys
import time
from pathlib import Path

#: The race clock in seconds (f32) in PPSSPP's Pulse USA RAM: 40.07 at a state whose HUD read 0:40.0, and it
#: counts 1:1 with ours (read 2026-10-20 by scanning RAM for a float that advances with the emulation).
CLOCK_ADDRESS = 0x08B344B0
#: Our race's tick at which its race clock leaves 0 (the countdown's length): ours reads 0:00.4 at tick 300 and
#: 0:40.4 at tick 2700, so GO is at 276.
GO_TICK = 276

#: `g_weapons_enabled` (byte, 0 = off), docs/ghidra/functions/psp-pulse-usa/shield.md.
WEAPONS_ENABLED = 0x08B30FA8
#: `g_camera_fov_degrees` (f32), the fov the frame is drawn with: the view's authored fov (`<ExternalCameraClose>` 60,
#: `<InternalCamera>` 65 on these ships) plus `0.075 * dot(fwd, vel)`, docs/ghidra/functions/psp-pulse-usa/camera.md.
CAMERA_FOV_ADDRESS = 0x08B34310
#: `<InternalCamera fov>` on every Pulse ship read so far (our log: `<InternalCamera>: fov 65`); a check below
#: compares it with the fov in RAM, so a ship that authors another value is refused rather than mis-posed.
INTERNAL_FOV = 65.0
#: `|velocity| * 3.6`, the HUD's speed readout (PlayerStatus_Update, docs/ghidra/functions/psp-pulse-usa/race-progress.md).
SPEED_TO_KMH = 3.6

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402


def run(cmd, **kw):
    print("+", " ".join(str(c) for c in cmd), file=sys.stderr)
    return subprocess.run([str(c) for c in cmd], check=True, **kw)


def xdo(*args, capture=False):
    return subprocess.run(["xdotool", *args], capture_output=capture, text=True, check=True)


def emulator_window():
    """`(id, x, y, w, h)` of PPSSPP's own window: the largest visible one that is not the root."""
    best = None
    for wid in xdo("search", "--onlyvisible", "--name", "", capture=True).stdout.split():
        geo = xdo("getwindowgeometry", "--shell", wid, capture=True).stdout
        v = dict(line.split("=") for line in geo.split() if "=" in line)
        x, y, w, h = (int(v[k]) for k in ("X", "Y", "WIDTH", "HEIGHT"))
        if (w, h) in ((960, 544), (480, 272)):
            best = (wid, x, y, w, h)
    if best is None:
        sys.exit("no PPSSPP window of 960x544 or 480x272 on DISPLAY")
    return best


def set_window(w, h):
    wid, x, y, cw, ch = emulator_window()
    if (cw, ch) != (w, h):
        xdo("windowsize", wid, str(w), str(h))
        time.sleep(2.0)
    return emulator_window()


def freeze(port):
    """Stop the CPU straight after the load, so the race clock stays at the state's own value while the
    "State loaded" toast fades (the toast runs on wall time, not emulated time)."""
    from ppsspp_debugger import Debugger
    Debugger(port).call("cpu.stepping")


def resume(port):
    """A previous `psp-trace.py` leaves the CPU stopped at its breakpoint; the pause-menu logic reads that as
    "the menu is open", so run it first."""
    from ppsspp_debugger import Debugger
    dbg = Debugger(port)
    if dbg.is_stepping():
        dbg.call("cpu.resume")
        time.sleep(1.0)


def race_field_module():
    """`psp-race-field.py` (a hyphenated name cannot be imported): the original's whole field out of RAM."""
    import importlib.util
    spec = importlib.util.spec_from_file_location("psp_race_field", HERE / "psp-race-field.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


#: `Mesh_UpdateTextureTransforms` (a0 = mesh): the world meshes' texture animation updater, texture-animation.md.
UPDATE_TEXTURE_TRANSFORMS = 0x0890E160


def world_clock_offset(port, hits=40):
    """The world meshes' animation clock minus the HUD's race clock, in seconds, read live.

    Every world mesh is fed one clock that began when the circuit loaded, so it stands at the HUD clock plus the
    flyby and countdown before GO (16.2 s on Fort Gale's state: `scripts/psp-world-anim-clock.py`), not at the HUD
    clock. The most common non-zero `mesh+0x40` over `hits` updater calls is that clock (other meshes hold 0 or a
    clock of their own), read with the HUD clock at the same stop. Runs the CPU for the hits, so call it after
    everything else has been captured from the state.
    """
    from collections import Counter
    from ppsspp_debugger import Debugger
    dbg = Debugger(port)
    seen = Counter()
    for _ in dbg.each_hit(UPDATE_TEXTURE_TRANSFORMS, hits, timeout=20.0):
        regs = dbg.call("cpu.getAllRegs")
        a0 = dict(zip(*[next(c for c in regs["categories"] if c["name"] == "GPR")[k] for k in ("registerNames", "uintValues")]))["a0"]
        t = dbg.read_f32(a0 + 0x40)
        if t > 0.0:
            seen[round(t - dbg.read_f32(CLOCK_ADDRESS), 3)] += 1
    if not seen:
        return None
    (offset, n), = seen.most_common(1)
    return offset, n, sum(seen.values())


def camera_craft_distance(row):
    return ce.math.dist([row["cam_pos_" + a] for a in "xyz"], [row["pos_" + a] for a in "xyz"])


def camera_view_of(row):
    """`internal` when the original's camera sits inside the craft (3.0 units from it at its own height), else
    the close chase view, the one every other pair is in (11.64 units back, 3 up)."""
    return "internal" if camera_craft_distance(row) < 6.0 else "close"


def detect_layout(oag, image, track, theirs_row, flags, tol):
    """Which of `track.vex` / `track_reversed.vex` the original loaded: our own start on each layout against
    the original's grid pose (position and heading). Returns (layout, report) or exits naming the circuit."""
    report = {}
    for layout, name in (("forward", "track.vex"), ("reversed", "track_reversed.vex")):
        csv = Path(tol["scratch"]) / ("layout-%s.csv" % layout)
        entry = "Data\\Environments\\%s\\%s" % (track, name)
        run([oag, image, "--race", "--track", entry, "--ticks", 1, "--trace-out", csv] + flags,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        ours = ce.pose_of(ce.csv_row(csv, 0))
        theirs = ce.pose_of(theirs_row)
        d = ce.math.dist(ours["pos"], theirs["pos"])
        a = ce.vec_angle(ours["fwd"], theirs["fwd"])
        report[layout] = {"start_distance": d, "heading_deg": a}
    best = min(report, key=lambda k: report[k]["start_distance"] + report[k]["heading_deg"])
    return best, report


def field_pair(args, pair, plat, out, image, facts, flags, csv, theirs_row, theirs, fov, layout, profile_dir, clock):
    """Ours for a race shot: every craft of the original's field, the player's pose and the camera from the
    one stop (`field.json`, `trace.csv`), rendered at that tick with no ticks run."""
    import json
    rows = json.loads((out / "field.json").read_text())["rows"]
    player = next(r for r in rows if r["player"])
    teams = ce.write_field_csv(out / "field.csv", rows, with_player=True)
    pair.rec["original"]["field"] = [{k: r[k] for k in ("slot", "team", "place", "lap", "lap_time", "pos")} for r in rows]
    lap_number = player["lap"] - 1  # PlayerStatus_Update: the HUD's lap is `entity+0xacc - 1`
    pair.rec["original"]["hud"] = {"place": player["place"], "of": len(rows), "lap_number": lap_number,
                                   "lap_time_s": player["lap_time"], "race_clock_s": clock,
                                   "shield": player["shield"], "shield_max": player["shield_max"],
                                   "best_lap_s": player["best_lap_cs"] / 100}
    pair.check_equal("original field size", len(rows), 8)
    ce.pack_check(pair, player["place"], [theirs_row["cam_pos_" + a] for a in "xyz"],
                  [-theirs_row["cam_right_z"], -theirs_row["cam_up_z"], -theirs_row["cam_fwd_z"]], player["pos"],
                  [r["pos"] for r in rows if not r["player"]], near=args.pack_near)
    pair.check("every craft has a team name", all(t and not t.startswith("?") for t in teams), ", ".join(teams))
    pair.rec["ours"].update({"mode": facts.get("mode", "single_race"), "class": facts.get("class", "venom"),
                             "team": args.team or "default", "layout": layout, "opponent_teams": teams,
                             "weapons": "off (settings [race] weapons)"})
    pair.rec["original"]["layout"] = layout
    entry = "Data\\Environments\\%s\\track%s.vex" % (args.track, "_reversed" if layout == "reversed" else "")
    base = [args.oag_game, image, "--race", "--track", entry, "--pose-from", csv, "--pose-tick", 0,
            "--field-from", out / "field.csv", "--field-warmup",
            "--hud-state", "%d,%d,%.4f,%.4f,%.4f,%.4f,%.4f" % (
                player["place"], lap_number, player["lap_time"], clock,
                theirs_row["speed"] * SPEED_TO_KMH, 100.0 * player["shield"] / player["shield_max"],
                player["best_lap_cs"] / 100)] + flags + [
        "--camera-fov", "%.4f" % fov]
    pair.rec["ours"]["ticks"] = 0
    pair.rec["ours"]["flags"] = [str(c) for c in base[2:]] + ["--ticks", 0]
    pair.caveat("every craft's trail and plume in ours are an exhaust-only warm-up (`--field-warmup`): each craft "
                "flown on a straight path to its captured pose at its captured velocity (RAM, body+0x140), full "
                "thrust for all (the player's reads 100, an opponent's is not read), the path and the heading chosen, "
                "not measured; the HUD's place, lap and clocks are imposed from the original's RAM (%d/%d, lap %d, "
                "%.1f s), the speed and shield bars read the original's own values; the pickup row (the Autopilot icon) "
                "is not drawn" %
                (player["place"], len(rows), lap_number, player["lap_time"]))
    ours_csv = out / "ours-trace.csv"
    run(base + ["--ticks", 1, "--trace-out", ours_csv], stdout=subprocess.DEVNULL)
    run(base + ["--ticks", 0] + view_flags + ["--screenshot", out / "ours.png"], stdout=subprocess.DEVNULL)
    got, bad = ce.read_back(plat, profile_dir)
    pair.rec["ours"]["settings_readback"] = got
    for b in bad:
        pair.check("ours settings", False, b)
    pair.check("ours settings read back", not bad, "%d keys equal the profile" % (len(got) - len(bad)))
    ours_row = ce.csv_row(ours_csv, 0)
    pair.check_pose(theirs, ce.pose_of(ours_row), fov, fov)
    pair.rec["ours"]["shield"] = ours_row.get("shield")
    pair.check_size("ours", out / "ours.png")
    run(["magick", "montage", out / "theirs.png", out / "ours.png", "-tile", "2x1", "-geometry", "+2+2", out / "side.png"])
    if args.meta:
        pair.rec["meta"] = json.loads(args.meta)
    pair.finish()
    print("wrote", out / "side.png")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--port", type=int, default=int(os.environ.get("OAG_PPSSPP_PORT", 47810)))
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--track", required=True, help="circuit directory, e.g. 03_Track")
    ap.add_argument("--layout", choices=("auto", "forward", "reversed"), default="auto",
                    help="auto: the layout whose own start is on the original's grid pose; forward/reversed asserts it")
    ap.add_argument("--team", help="our --team, when the original's is not the default")
    ap.add_argument("--state", required=True, help="a .ppst to restore into slot 2 first")
    ap.add_argument("--home", default=os.environ.get("PP_HOME"), help="the instance's own HOME (PP_HOME)")
    ap.add_argument("--platform", default="pulse-psp")
    ap.add_argument("--oag-game", default=str(ROOT / "target/debug/oag-game"))
    ap.add_argument("--ours-ticks", type=int, default=None,
                    help="ticks our race runs before the frame. Default: GO_TICK + the original's race clock, read "
                         "out of its RAM (%#x, seconds as a float) after the load, so both sides are the same "
                         "number of seconds after GO and show the same GO board and record panel" % CLOCK_ADDRESS)
    ap.add_argument("--field", action="store_true",
                    help="a race shot: the state is a mid-race frame of a Single Race. Ours takes ALL eight craft "
                         "(pose, team) and the camera from the same stop of the original as the shot, renders at that "
                         "tick (--ticks 0) and the layout must be asserted with --layout forward|reversed (the grid "
                         "probe is meaningless mid-race); the HUD race state is recorded as a caveat")
    ap.add_argument("--pack-near", type=float, default=45.0,
                    help="with --field: how close to the player a craft in view must be (units); the pair is "
                         "refused when the frame is not a pack")
    ap.add_argument("--toast-wait", type=float, default=6.0)
    ap.add_argument("--meta", default="", help="JSON object for the manifest: id, title, track, craft, state, note")
    args = ap.parse_args()
    if "OAG_SHOT_DISPLAY" not in os.environ or "DISPLAY" not in os.environ:
        sys.exit("DISPLAY and OAG_SHOT_DISPLAY must name the instance's own Xvfb")
    if not args.home:
        sys.exit("--home (or PP_HOME) must name the instance's own HOME")
    scratch_root = str(ce.MAIN / "data" / "scratch")
    if not os.environ.get("XDG_CONFIG_HOME", "").startswith(scratch_root):
        sys.exit("XDG_CONFIG_HOME must be a directory under data/scratch/<lane>/: oag-game persists settings there")
    pair = ce.Pair(args.platform, args.out)
    plat = pair.plat
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    pair.out = out
    image = ce.MAIN / plat["image"]
    facts = ce.state_facts(args.state)
    pair.rec["original"]["state"] = {"file": args.state, **facts}
    pair.rec["original"]["emulator"] = dict(plat["emulator"])
    pair.rec["ours"]["binary"] = ce.binary_record(args.oag_game)

    profile_dir = Path(os.environ["XDG_CONFIG_HOME"]).parent
    ce.write_ours_settings(plat, profile_dir)
    if args.field:
        # ours races with weapons off the way the original's Racebox row was set (checked below against
        # `g_weapons_enabled` in the state's own RAM); the saved WEAPONS row is what `Mode::SingleRace` reads.
        ce.set_ours_weapons_off(profile_dir)
    flags = ce.ours_flags(plat)
    mode = facts.get("mode", "single_race" if args.field else "time_trial")
    klass = facts.get("class", "venom")
    flags += ["--mode", mode, "--class", klass]
    if args.team:
        flags += ["--team", args.team]

    uv = ["uv", "run", "--with", "websocket-client", "python3"]
    set_window(960, 544)
    resume(args.port)
    run(uv + [HERE / "psp-state.py", "--port", args.port, "--home", args.home, "load", "2", "--file", args.state])
    freeze(args.port)
    time.sleep(args.toast_wait)
    from ppsspp_debugger import Debugger
    clock = Debugger(args.port).read_f32(CLOCK_ADDRESS)
    ram_fov = Debugger(args.port).read_f32(CAMERA_FOV_ADDRESS)
    if not 0.0 <= clock <= 3600.0:
        sys.exit("the race clock at %#x reads %r: not a clock, the address is wrong for this state" % (CLOCK_ADDRESS, clock))
    pair.rec["original"]["race_clock_s"] = clock
    if args.ours_ticks is None:
        args.ours_ticks = GO_TICK + round(clock * 60)
    pair.rec["ours"]["ticks"] = args.ours_ticks
    pair.rec["ours"]["race_clock_s"] = (args.ours_ticks - GO_TICK) / 60
    pair.check("race clock phase", abs((args.ours_ticks - GO_TICK) / 60 - clock) <= 0.05,
               "original %.2f s after GO, ours %.2f s (ticks %d, GO at tick %d)"
               % (clock, (args.ours_ticks - GO_TICK) / 60, args.ours_ticks, GO_TICK))
    trace_extra = []
    if args.field:
        rf = race_field_module()
        from ppsspp_debugger import Debugger as _D
        probe = _D(args.port)
        player_craft = rf.Field(probe).player["craft"]
        wp = probe.read(WEAPONS_ENABLED, 1)[0]
        pair.rec["original"]["weapons_enabled_byte"] = wp
        pair.check("weapons off in the original's own RAM", wp == 0,
                   "g_weapons_enabled byte at %#x reads %d (0 = off)" % (WEAPONS_ENABLED, wp))
        trace_extra = ["--craft", hex(player_craft), "--field-out", out / "field.json"]
    wid, wx, wy, ww, wh = set_window(*plat["output"])
    pair.check_equal("original window size", [ww, wh], list(plat["output"]))
    shots = out / "shots"
    csv = out / "trace.csv"
    with open(csv, "w") as f:
        run(uv + [HERE / "psp-trace.py", "--port", args.port, "--ticks", 4, "--camera", "--shot-every", 2,
                  "--shot-dir", shots] + trace_extra, stdout=f)
    set_window(960, 544)
    run(["magick", shots / "tick00000.png", "-crop", "%dx%d+%d+%d" % (ww, wh, wx, wy), "+repage", out / "theirs.png"])
    theirs_row = ce.csv_row(csv, 0)
    theirs = ce.pose_of(theirs_row)
    view = camera_view_of(theirs_row)
    fov = ce.speed_fov(theirs_row, INTERNAL_FOV if view == "internal" else 60.0)
    pair.rec["original"]["fov_source"] = ("the view's authored fov (%s) plus 0.075 * dot(fwd, vel), checked against "
                                          "g_camera_fov_degrees (%#x) read at the clock's stop" % (
                                              INTERNAL_FOV if view == "internal" else 60.0, CAMERA_FOV_ADDRESS))
    pair.rec["original"]["fov_ram_at_stop"] = ram_fov
    pair.check("the original's fov in RAM at the stop is the view's authored fov plus the speed term (4 ticks of drift allowed)",
               abs(ram_fov - fov) < 1.0, "RAM %.3f, law %.3f" % (ram_fov, fov))
    world = world_clock_offset(args.port)
    pair.check("the world meshes' animation clock was read", world is not None and world[1] * 2 >= world[2],
               "%s" % (world,))
    if world is not None:
        offset, n, total = world
        pair.rec["original"]["world_clock_offset_s"] = offset
        pair.rec["original"]["world_clock_note"] = ("world meshes' animation clock minus the HUD race clock, the most common "
                                                    "non-zero value (%d of %d updater hits), live" % (n, total))
        ours_offset = offset - GO_TICK / 60
        flags += ["--world-clock-offset", "%.3f" % ours_offset]
        pair.rec["ours"]["world_clock_offset_s"] = ours_offset
    pair.rec["original"]["camera"] = {"pos": [theirs_row["cam_pos_" + a] for a in "xyz"],
                                      "fwd": [theirs_row["cam_fwd_" + a] for a in "xyz"], "source": "psp-trace.py --camera"}
    pair.rec["original"]["shield"] = theirs_row.get("shield")
    pair.rec["original"]["camera_view"] = {"view": view, "source": "camera-to-craft distance (%.2f units; the chase "
                                           "views sit 11.64 away, the cockpit view 3.0)" % camera_craft_distance(theirs_row)}
    pair.rec["ours"]["camera_view"] = view
    view_flags = ["--camera-view", view]
    pair.check_size("original", out / "theirs.png")

    if args.field:
        if args.layout == "auto":
            sys.exit("--field needs --layout forward|reversed: the grid probe is a start-line test")
        layout = args.layout
        declared = facts.get("layout")
        pair.check("layout asserted matches the state's declared layout", declared in (None, layout),
                   "asserted %s, sidecar declares %s" % (layout, declared))
        return field_pair(args, pair, plat, out, image, facts, flags, csv, theirs_row, theirs, fov, layout, profile_dir,
                          clock)
    layout, rep = detect_layout(args.oag_game, image, args.track, theirs_row, flags, {"scratch": str(out)})
    pair.rec["ours"]["layout_probe"] = rep
    asserted = {"forward": False, "reversed": True}.get(args.layout)
    pair.check("layout file matches the start position", asserted is None or (layout == "reversed") == asserted,
               "probe says %s (%s); %s" % (
                   layout, ", ".join("%s %.1f units / %.1f deg" % (k, v["start_distance"], v["heading_deg"])
                                     for k, v in rep.items()),
                   "asserted %s" % args.layout if asserted is not None else "nothing asserted, the probe chooses"))
    # the heading is only a layout discriminator here (the wrong layout points 178 degrees away); the ship may have
    # yawed on the grid since the start, and the real pose parity is checked at the rendered tick below
    pair.check("circuit start within 25 units of the original's grid pose, facing the same way (under 90 deg)",
               rep[layout]["start_distance"] <= 25 and rep[layout]["heading_deg"] <= 90,
               "%.1f units, %.1f deg" % (rep[layout]["start_distance"], rep[layout]["heading_deg"]))
    pair.rec["ours"].update({"mode": mode, "class": klass, "team": args.team or "default", "layout": layout})
    pair.rec["original"]["layout"] = layout
    entry = "Data\\Environments\\%s\\track%s.vex" % (args.track, "_reversed" if layout == "reversed" else "")
    base = [args.oag_game, image, "--race", "--track", entry, "--pose-from", csv, "--pose-tick", 0] + flags
    pair.rec["ours"]["flags"] = [str(c) for c in base[2:]] + ["--camera-fov", "%.4f" % fov, "--ticks", args.ours_ticks]
    if pair.failed():
        pair.finish()
    # A craft resting after GO turns, in the original as in ours (bank-to-yaw, docs/physics/grid-state.md), and the
    # original's pose has already turned for the clock it was read at. Running that clock again from it turned ours
    # twice, so `--pose-hold` puts the craft back on the fed pose for the frame and the last trace row; the clock,
    # the GO board, the record panel and the exhaust still run their ticks. The pose check below is then an import
    # check (basis, convention, fov); `scripts/compare-resting-yaw.py` measures the turn itself against the pairs.
    ours_csv = out / "ours-trace.csv"
    cmd = base + ["--pose-hold"]
    pair.rec["ours"]["pose_hold"] = True
    pair.rec["ours"]["flags"] = [str(c) for c in cmd[2:]] + ["--camera-fov", "%.4f" % fov, "--ticks", args.ours_ticks]
    run(cmd + ["--camera-fov", "%.4f" % fov, "--ticks", args.ours_ticks] + view_flags + ["--screenshot", out / "ours.png"],
        stdout=subprocess.DEVNULL)
    run(cmd + ["--camera-fov", "%.4f" % fov, "--ticks", args.ours_ticks, "--trace-out", ours_csv],
        stdout=subprocess.DEVNULL)
    got, bad = ce.read_back(plat, profile_dir)
    pair.rec["ours"]["settings_readback"] = got
    for b in bad:
        pair.check("ours settings", False, b)
    pair.check("ours settings read back", not bad, "%d keys equal the profile" % (len(got) - len(bad)))
    ours_row = ce.csv_row(ours_csv, args.ours_ticks)
    pair.check_pose(theirs, ce.pose_of(ours_row), fov, fov)
    pair.rec["ours"]["shield"] = ours_row.get("shield")
    pair.check_size("ours", out / "ours.png")
    run(["magick", "montage", out / "theirs.png", out / "ours.png", "-tile", "2x1", "-geometry", "+2+2", out / "side.png"])
    if args.meta:
        import json
        pair.rec["meta"] = json.loads(args.meta)
    pair.finish()
    print("wrote", out / "side.png")


if __name__ == "__main__":
    main()
