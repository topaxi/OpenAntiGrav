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
camera and its speed-widened fov, and run a second time with `--trace-out` so the ship's pose at the rendered
tick is MEASURED from our own run and compared with the original's row. A pair that differs from the profile
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


FIELD_COLUMNS = ("slot,team,left_x,left_y,left_z,up_x,up_y,up_z,fwd_x,fwd_y,fwd_z,pos_x,pos_y,pos_z").split(",")


def write_field_csv(path, rows):
    """`oag-game --field-from`'s CSV: the opponents (not the player) in slot order, slot numbered 1..N."""
    lines = [",".join(FIELD_COLUMNS)]
    others = sorted((r for r in rows if not r["player"]), key=lambda r: r["slot"])
    for n, r in enumerate(others, 1):
        cells = [str(n), r["team"]] + [repr(v) for v in (*r["left"], *r["up"], *r["fwd"], *r["pos"])]
        lines.append(",".join(cells))
    Path(path).write_text("\n".join(lines) + "\n")
    return [r["team"] for r in others]


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
    teams = write_field_csv(out / "field.csv", rows)
    pair.rec["original"]["field"] = [{k: r[k] for k in ("slot", "team", "place", "lap", "lap_time", "pos")} for r in rows]
    lap_number = player["lap"] - 1  # PlayerStatus_Update: the HUD's lap is `entity+0xacc - 1`
    pair.rec["original"]["hud"] = {"place": player["place"], "of": len(rows), "lap_number": lap_number,
                                   "lap_time_s": player["lap_time"], "race_clock_s": clock,
                                   "best_lap_s": player["best_lap_cs"] / 100}
    pair.check_equal("original field size", len(rows), 8)
    pair.check("every craft has a team name", all(t and not t.startswith("?") for t in teams), ", ".join(teams))
    pair.rec["ours"].update({"mode": facts.get("mode", "single_race"), "class": facts.get("class", "venom"),
                             "team": args.team or "default", "layout": layout, "opponent_teams": teams,
                             "weapons": "off (settings [race] weapons)"})
    pair.rec["original"]["layout"] = layout
    entry = "Data\\Environments\\%s\\track%s.vex" % (args.track, "_reversed" if layout == "reversed" else "")
    base = [args.oag_game, image, "--race", "--track", entry, "--pose-from", csv, "--pose-tick", 0,
            "--field-from", out / "field.csv",
            "--hud-state", "%d,%d,%.4f,%.4f,%.4f,%.4f,%.4f" % (
                player["place"], lap_number, player["lap_time"], clock,
                theirs_row["speed"] * SPEED_TO_KMH, theirs_row["shield"], player["best_lap_cs"] / 100)] + flags + [
        "--camera-fov", "%.4f" % fov]
    pair.rec["ours"]["ticks"] = 0
    pair.rec["ours"]["flags"] = [str(c) for c in base[2:]] + ["--ticks", 0]
    pair.caveat("trails and exhaust history start from nothing in ours (stage 3, a frozen warm-up, is not run); the "
                "HUD's place, lap and clocks are imposed from the original's RAM (%d/%d, lap %d, %.1f s), "
                "the speed and shield bars read the original's own values; the pickup row (the Autopilot icon) and the "
                "exhaust plume are not drawn" %
                (player["place"], len(rows), lap_number, player["lap_time"]))
    ours_csv = out / "ours-trace.csv"
    run(base + ["--ticks", 1, "--trace-out", ours_csv], stdout=subprocess.DEVNULL)
    run(base + ["--ticks", 0, "--screenshot", out / "ours.png"], stdout=subprocess.DEVNULL)
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
        with open(ce.settings_path(profile_dir), "a") as f:
            f.write("\n[race]\nweapons = \"Off\"\n")
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
    fov = ce.speed_fov(theirs_row)
    pair.rec["original"]["camera"] = {"pos": [theirs_row["cam_pos_" + a] for a in "xyz"],
                                      "fwd": [theirs_row["cam_fwd_" + a] for a in "xyz"], "source": "psp-trace.py --camera"}
    pair.rec["original"]["shield"] = theirs_row.get("shield")
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
    # Ours can turn slowly while it sits (a craft that rests on a banked or skewed grid yaws, the original's does
    # not), so the row handed to `--pose-from` is turned about the craft's up axis until the heading MEASURED at the
    # rendered tick (a second, trace-only run of the same command) is the original's.
    ours_csv = out / "ours-trace.csv"
    fixed_csv = out / "trace-corrected.csv"
    lines = csv.read_text().splitlines()
    start = next(i for i, ln in enumerate(lines) if ln.startswith("tick,"))
    head = lines[start].split(",")
    data = [ln.split(",") for ln in lines[start + 1:] if ln.count(",") == len(head) - 1]
    first = dict(zip(head, data[0]))

    def with_turn(deg):
        cells = dict(first)
        up = [float(first["up_" + a]) for a in "xyz"]
        for name in ("right", "fwd"):
            v = ce.rotate_about([float(first["%s_%s" % (name, a)]) for a in "xyz"], up, deg)
            for a, x in zip("xyz", v):
                cells["%s_%s" % (name, a)] = repr(x)
        fixed_csv.write_text("\n".join([",".join(head)] + [",".join(cells[h] for h in head)]
                                       + [",".join(r) for r in data[1:]]) + "\n")
        cmd = [c if c != csv else fixed_csv for c in base]
        run(cmd + ["--camera-fov", "%.4f" % fov, "--ticks", args.ours_ticks + 1, "--trace-out", ours_csv],
            stdout=subprocess.DEVNULL)
        row = ce.csv_row(ours_csv, args.ours_ticks)
        return ce.signed_heading([row["fwd_" + a] for a in "xyz"], [row["up_" + a] for a in "xyz"], theirs["fwd"]), cmd

    turn, sign = 0.0, 1.0
    err, cmd = with_turn(turn)
    corrections = [{"turn_deg": turn, "heading_error_deg": err}]
    for _ in range(10):
        if abs(err) <= plat["tolerance"]["heading_deg"] / 4:
            break
        trial = turn + sign * err
        new_err, new_cmd = with_turn(trial)
        if abs(new_err) > abs(err) and sign == 1.0:
            sign = -1.0
            trial = turn + sign * err
            new_err, new_cmd = with_turn(trial)
        turn, err, cmd = trial, new_err, new_cmd
        corrections.append({"turn_deg": turn, "heading_error_deg": err})
    pair.rec["ours"]["heading_corrections"] = corrections
    pair.rec["ours"]["flags"] = [str(c) for c in cmd[2:]] + ["--camera-fov", "%.4f" % fov, "--ticks", args.ours_ticks]
    run(cmd + ["--camera-fov", "%.4f" % fov, "--ticks", args.ours_ticks, "--screenshot", out / "ours.png"],
        stdout=subprocess.DEVNULL)
    run(cmd + ["--camera-fov", "%.4f" % fov, "--ticks", args.ours_ticks + 1, "--trace-out", ours_csv],
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
