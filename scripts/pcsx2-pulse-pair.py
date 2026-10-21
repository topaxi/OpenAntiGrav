#!/usr/bin/env python3
"""One original-vs-ours pair of Wipeout Pulse (PS2) from a savestate, environment-checked.

    source <scratch>/ps2/env.sh      # OAG_PCSX2_DISPLAY, OAG_PCSX2_DATA, private XDG_*, OAG_COMPARE_PLATFORM=pulse-ps2
    uv run --with python-xlib python3 scripts/pcsx2-pulse-pair.py \\
        --slot 28098 --state data/saves/pulse-ps2/<point>.p2s --track 03_Track --out <dir> [--allow-own-camera]

Needs a booted PCSX2 on the private data directory started with `OAG_COMPARE_PLATFORM=pulse-ps2` (so its ini
carries the profile's upscale and screenshot size, `pcsx2-drive.py boot`). Loads the state, takes PCSX2's own
GS screenshot while the VM stands still on the state's frame, and resizes it ONCE (Lanczos) to the profile's
output size; ours is rendered natively at that size. The ship pose, the race clock and the camera node are
read out of the state file's own EE RAM (`pcsx2_state.py`), so what is compared is the state's frame, not
the frame some ticks later. Ours is run a second time with `--trace-out` to MEASURE its pose at the rendered
tick. The camera is the original's own: its eye, orientation and vertical field are read out of the state's EE RAM
(`pcsx2_state.camera`, the object `Camera_SubmitScene` runs on) and fed to ours (`cam_*` columns of the pose row,
`--camera-fov`), as the PSP and HD pairs do. The state must have the game's own `Aspect Ratio` option on `16:9`
(`DAT_00284fe8` = 1, `docs/ghidra/functions/ps2-pulse-eu/camera.md`): a pair whose state is on `4:3` is refused,
because the profile is the PS2's widescreen and the projection aspect is read back and checked.
"""

import argparse
import json
import math
import os
import shutil
import struct
import subprocess
import sys
import time
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402

#: Our race's tick at which the race clock leaves 0: `oag_race::COUNTDOWN_TICKS`, read from ours by `compare_env.go_tick` (one
#: length for every title today; it was a hard-coded 276 here until 2026-10-21, 4 ticks late).
GO_TICK = ce.go_tick()
import pcsx2_pine  # noqa: E402
import pcsx2_state  # noqa: E402

#: EE address of the race clock (f32 seconds): 23.39 at a state whose HUD read 0:23.2 and 26.97 after about
#: 3.6 s more of emulation (two states, 2026-10-20). The HUD runs about 0.2 s behind it, so the check is loose.
CLOCK_ADDRESS = 0x70EE9C
CLOCK_TOLERANCE_S = 0.1
#: The HUD's clock reads 0.19 s under the RAM float (0:23.2 against 23.39, one state), so ours runs to the HUD's.
CLOCK_HUD_OFFSET_S = 0.19


def run(cmd, **kw):
    print("+", " ".join(str(c) for c in cmd), file=sys.stderr)
    return subprocess.run([str(c) for c in cmd], check=True, **kw)


def resume(slot):
    """Run the VM: a previous trace or state save leaves it paused, and the GS screenshot hotkey does nothing
    on a paused one (`screenshot FAILED`). The load that follows leaves the VM standing on the state's frame."""
    import importlib.util
    spec = importlib.util.spec_from_file_location("pcsx2_drive", HERE / "pcsx2-drive.py")
    drive = importlib.util.module_from_spec(spec)
    sys.path.insert(0, str(HERE))
    spec.loader.exec_module(drive)
    pine = drive.Pine(slot=slot)
    drive.set_paused(pine, drive.Keyboard(), False)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--slot", type=int, required=True, help="the instance's PINE slot")
    ap.add_argument("--state", type=Path, required=True)
    ap.add_argument("--track", required=True, help="circuit directory, e.g. 03_Track")
    ap.add_argument("--reversed", action="store_true")
    ap.add_argument("--team")
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--platform", default="pulse-ps2")
    ap.add_argument("--oag-game", default=str(ROOT / "target/debug/oag-game"))
    ap.add_argument("--serial", default="SCES-54748 (F8AE6FF2)",
                    help="the state file name's game part in PCSX2's sstates directory")
    ap.add_argument("--meta", default="")
    args = ap.parse_args()
    data = os.environ.get("OAG_PCSX2_DATA")
    if not data or "OAG_PCSX2_DISPLAY" not in os.environ:
        sys.exit("source the lane's own env: OAG_PCSX2_DATA and OAG_PCSX2_DISPLAY must be set")
    if os.environ.get("OAG_COMPARE_PLATFORM") != args.platform:
        sys.exit("start PCSX2 with OAG_COMPARE_PLATFORM=%s so its ini carries the profile's settings" % args.platform)
    if not os.environ.get("XDG_CONFIG_HOME", "").startswith(str(ce.MAIN / "data" / "scratch")):
        sys.exit("XDG_CONFIG_HOME must be under data/scratch/<lane>/ (oag-game writes its profile there)")
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    pair = ce.Pair(args.platform, out)
    plat = pair.plat
    image = ce.MAIN / plat["image"]
    emu = plat["emulator"]
    ini = Path(data) / "PCSX2" / "inis" / "PCSX2.ini"
    text = ini.read_text()
    for key, want in (("upscale_multiplier", emu["upscale_multiplier"]), ("ScreenshotSize", emu["screenshot_size"]),
                      ("AspectRatio", emu.get("aspect_ratio", "Stretch"))):
        line = next((ln for ln in text.splitlines() if ln.startswith(key)), "")
        got = line.split("=")[-1].strip()
        pair.check("PCSX2 %s" % key, got == str(want), "ini says %r, profile %r" % (got, want))
    pair.rec["original"]["emulator"] = dict(emu)
    pair.rec["ours"]["binary"] = ce.binary_record(args.oag_game)
    sidecar = pcsx2_state.read_sidecar(args.state) or {}
    pair.rec["original"]["state"] = {"file": str(args.state), "sidecar_circuit": sidecar.get("circuit"),
                                     "mode": "single_race", "player_slot": "8 of 8 (sidecar)", "class": "venom (assumed)"}

    states = Path(data) / "PCSX2" / "sstates"
    states.mkdir(parents=True, exist_ok=True)
    shutil.copy(args.state, states / ("%s.01.p2s" % args.serial))
    uv = ["uv", "run", "--with", "python-xlib", "python3"]
    drive = uv + [HERE / "pcsx2-drive.py", "--slot", args.slot]
    resume(args.slot)
    run(drive + ["state", "load", "1"])
    time.sleep(1.5)
    # The VM runs after the load (about a third of real time under Xvfb), so the grab is a few tenths of a second
    # past the state's frame: read the clock before and after it, and let ours run to what the picture shows.
    pine = pcsx2_pine.Pine(slot=args.slot)
    live_before = struct.unpack("<f", struct.pack("<I", pine.read32(CLOCK_ADDRESS)))[0]
    raw = out / "theirs-gs.png"
    run(drive + ["shot", raw])
    live_after = struct.unpack("<f", struct.pack("<I", pine.read32(CLOCK_ADDRESS)))[0]
    pair.rec["original"]["gs_screenshot_size"] = ce.image_size(raw)
    w, h = plat["output"]
    run(["magick", raw, "-filter", "Lanczos", "-resize", "%dx%d!" % (w, h), out / "theirs.png"])
    pair.rec["original"]["resize"] = "one Lanczos resize from %dx%d to %dx%d" % (*pair.rec["original"]["gs_screenshot_size"], w, h)
    pair.check_size("original", out / "theirs.png")
    gs_aspect = pair.rec["original"]["gs_screenshot_size"]
    pair.check("GS screenshot aspect equals the output aspect", abs(gs_aspect[0] / gs_aspect[1] - w / h) < 0.01,
               "%dx%d against %dx%d" % (gs_aspect[0], gs_aspect[1], w, h))

    craft, body, pos = pcsx2_state.player_craft(args.state)
    ram = zipfile.ZipFile(args.state).read("eeMemory.bin")
    vec = lambda a: struct.unpack_from("<3f", ram, a)
    right, up, fwd = vec(body), vec(body + 0x10), vec(body + 0x20)
    cam = pcsx2_state.camera(args.state, pos)
    if cam is None:
        pair.check("camera", False, "no camera object (fov, perspective and world matrix agreeing, within 25 units of the craft) in the state")
        pair.finish()
    pair.rec["original"]["camera"] = {k: (list(v) if isinstance(v, tuple) else v) for k, v in cam.items()}
    pair.rec["original"]["camera"]["source"] = "state EE RAM, camera object at %#x" % cam["address"]
    wide = 10 / 7 * 4 / 3
    pair.check("the state's game option is Aspect Ratio 16:9 (projection aspect 10/7 * 4/3)", abs(cam["aspect"] - wide) < 0.01,
               "projection aspect %.4f, the 4:3 option reads %.4f, 16:9 reads %.4f" % (cam["aspect"], 10 / 7, wide))
    clock = struct.unpack_from("<f", ram, CLOCK_ADDRESS)[0]
    if not 0.0 <= clock <= 3600.0:
        pair.check("race clock", False, "%r at %#x is not a clock" % (clock, CLOCK_ADDRESS))
        pair.finish()
    row = {"dt": 0.02, "grounded": 1, "speed": 0.0}
    for name, v in (("right", right), ("up", up), ("fwd", fwd), ("pos", pos)):
        for a, x in zip("xyz", v):
            row["%s_%s" % (name, a)] = x
    # The trace's camera columns are the VIEW matrix's rows (columns = the camera's axes, `camera_orientation_of`:
    # right = column 0, up = column 1, backward = column 2); the camera object holds those axes as rows.
    axes = (cam["right"], cam["up"], cam["back"])
    for r, name in enumerate(("right", "up", "fwd")):
        for c, a in enumerate("xyz"):
            row["cam_%s_%s" % (name, a)] = axes[c][r]
    for i, a in enumerate("xyz"):
        row["cam_pos_" + a] = cam["eye"][i]
    to_craft = [p - e for p, e in zip(pos, cam["eye"])]
    look = [-x for x in cam["back"]]
    cosine = sum(a * b for a, b in zip(to_craft, look)) / math.hypot(*to_craft)
    pair.check("the camera's look direction (minus its backward axis) points at the craft", cosine > 0.97,
               "cosine %.4f, eye %.2f units from the craft" % (cosine, math.hypot(*to_craft)))
    csv = out / "state-row.csv"
    ce.write_trace_row(csv, row)
    theirs = {"pos": list(pos), "fwd": list(fwd), "vel": [0, 0, 0]}
    pair.rec["original"]["pose"] = theirs
    pair.rec["original"]["race_clock_s"] = clock
    pair.rec["original"]["craft"] = hex(craft)

    grab_clock = (live_before + live_after) / 2
    pair.rec["original"]["race_clock_live_s"] = {"before_grab": live_before, "after_grab": live_after, "state_file": clock}
    pair.check("the grab is of a running VM near the state's frame", 0.0 <= grab_clock - clock <= 3.0,
               "RAM clock %.2f s before and %.2f s after the grab, %.2f s in the state file" % (live_before, live_after, clock))
    pair.caveat("the original's frame is the GS screenshot of the VM running a moment after the load (race clock %.2f to %.2f s, "
                "the state file's is %.2f s), so its HUD clock and any animated surface are a few tenths of a second "
                "past the state; ours is rendered at the grab's clock" % (live_before, live_after, clock))
    shown = grab_clock - CLOCK_HUD_OFFSET_S
    ticks = GO_TICK + round(shown * 60)
    pair.rec["ours"].update({"ticks": ticks, "race_clock_s": (ticks - GO_TICK) / 60})
    pair.check("race clock phase", abs((ticks - GO_TICK) / 60 - shown) <= CLOCK_TOLERANCE_S,
               "original HUD %.2f s (RAM %.2f s), ours %.2f s (ticks %d)" % (shown, clock, (ticks - GO_TICK) / 60, ticks))

    scratch = Path(os.environ["XDG_CONFIG_HOME"]).parent
    ce.write_ours_settings(plat, scratch)
    flags = ce.ours_flags(plat) + ["--mode", "single_race", "--class", "venom"]
    if args.team:
        flags += ["--team", args.team]
    entry = "Data\\Environments\\%s\\track%s.vex" % (args.track, "_reversed" if args.reversed else "")
    # `--pose-hold`: the original's pose has already turned for the clock it was read at (a resting craft turns, in
    # the original as in ours, docs/physics/grid-state.md), so the frame and the last trace row put ours back on it.
    base = [args.oag_game, image, "--race", "--track", entry, "--pose-from", csv, "--pose-tick", 0, "--camera-fov", "%.4f" % cam["fov_deg"], "--pose-hold"] + flags
    pair.rec["ours"].update({"mode": "single_race", "class": "venom", "layout": "reversed" if args.reversed else "forward",
                             "flags": [str(c) for c in base[2:]], "camera": "the original's own eye, orientation and field (state RAM)"})
    probe = {}
    for layout, name in (("forward", "track.vex"), ("reversed", "track_reversed.vex")):
        probe_csv = out / ("layout-%s.csv" % layout)
        run([args.oag_game, image, "--race", "--track", "Data\\Environments\\%s\\%s" % (args.track, name),
             "--ticks", 1, "--trace-out", probe_csv] + flags, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        ours_start = ce.pose_of(ce.csv_row(probe_csv, 0))
        probe[layout] = {"heading_deg": ce.vec_angle(ours_start["fwd"], list(fwd)),
                         "start_distance": math.dist(ours_start["pos"], list(pos))}
    pair.rec["ours"]["layout_probe"] = probe
    best = min(probe, key=lambda k: probe[k]["heading_deg"])
    pair.check("layout file by start heading", best == pair.rec["ours"]["layout"] and probe[best]["heading_deg"] <= 5,
               "heading to the grid: %s (the player's slot differs from ours, so the distance is not used)" % ", ".join(
                   "%s %.1f deg, %.1f units" % (k, v["heading_deg"], v["start_distance"]) for k, v in probe.items()))
    if pair.failed():
        pair.finish()
    run(base + ["--ticks", ticks, "--screenshot", out / "ours.png"], stdout=subprocess.DEVNULL)
    ours_csv = out / "ours-trace.csv"
    run(base + ["--ticks", ticks, "--trace-out", ours_csv], stdout=subprocess.DEVNULL)
    got, bad = ce.read_back(plat, scratch)
    pair.rec["ours"]["settings_readback"] = got
    for b in bad:
        pair.check("ours settings", False, b)
    pair.check("ours settings read back", not bad, "%d keys equal the profile" % (len(got) - len(bad)))
    pair.check_pose(theirs, ce.pose_of(ce.csv_row(ours_csv, ticks)))
    pair.check_size("ours", out / "ours.png")
    run(["magick", "montage", out / "theirs.png", out / "ours.png", "-tile", "2x1", "-geometry", "+2+2", out / "side.png"])
    if args.meta:
        pair.rec["meta"] = json.loads(args.meta)
    pair.finish()
    print("wrote", out / "side.png")


if __name__ == "__main__":
    main()
