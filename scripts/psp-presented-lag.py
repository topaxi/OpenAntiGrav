#!/usr/bin/env python3
"""How many ticks the presented PPSSPP frame lags the paused stop, measured on a moving craft.

    source <scratch>/pp/env.sh; export DISPLAY OAG_PPSSPP_PORT PP_HOME OAG_SHOT_DISPLAY=$DISPLAY XDG_CONFIG_HOME=...
    python3 scripts/psp-presented-lag.py --state data/saves/pulse-psp/race-talons-venom-alongside.ppst --track 16_Track \\
        --team Assegai --out data/scratch/<lane>/lag [--ticks 14]

Loads the state, stops the CPU, and records `--ticks` consecutive per-tick rows (`psp-trace.py --camera`) with a
screenshot at every stop. The screenshot at stop `i` shows the last PRESENTED frame, which is the game state of some
earlier tick `i - lag`. Ours is rendered at every candidate row `k` (`--pose-from trace.csv --pose-tick k`, the row's
own pose and camera) and the candidate with the smallest mean grey difference to the shot names the lag. A craft that
is moving (the race state's Autopilot flies at about 163 units/s) moves the whole picture by several pixels a tick,
so the minimum is sharp; at rest every candidate ties and the script says so.

Writes `lag.json` (per shot: the score of every candidate, the best `k`, the lag) and prints the table; the pair
scripts read the constant from `docs/tools/compare-page.md`, not from a run.
"""

import argparse
import importlib.util
import json
import os
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402


def pair_module():
    spec = importlib.util.spec_from_file_location("psp_pulse_pair", HERE / "psp-pulse-pair.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def skew_table(args, pp, plat, flags, entry, csv, shots, out, window):
    """The craft's row `k` posed with the camera of row `k + c`: which pair of rows the presented frame is drawn from.
    The two are read at one stop but the game updates the craft, then the camera, then draws, so a frame may pair a craft
    with a camera a tick apart (the race pair's player hull is larger than ours, which a camera a tick ahead would do)."""
    import csv as csvmod
    ww, wh, wx, wy = window
    rows = list(csvmod.DictReader(open(csv)))
    camcols = [c for c in rows[0] if c.startswith("cam_")]
    skews = [int(v) for v in args.camera_skews.split(",")]
    table = []
    for i in range(args.first, args.ticks):
        theirs = out / ("theirs%02d.png" % i)
        pp.run(["magick", shots / ("tick%05d.png" % i), "-crop", "%dx%d+%d+%d" % (ww, wh, wx, wy), "+repage", theirs])
        for k in range(max(0, i - 2), i + 1):
            for c in skews:
                src = min(max(k + c, 0), len(rows) - 1)
                variant = out / ("variant_%02d_%d_%d.csv" % (i, k, c))
                mod = [dict(r) for r in rows]
                for col in camcols:
                    mod[k][col] = rows[src][col]
                with open(variant, "w", newline="") as f:
                    w = csvmod.DictWriter(f, fieldnames=list(rows[0]))
                    w.writeheader()
                    w.writerows(mod)
                png = out / ("v%02d_%d_%d.png" % (i, k, c))
                fov = ce.speed_fov(ce.csv_row(csv, k), 60.0)
                pp.run([args.oag_game, ce.MAIN / plat["image"], "--race", "--track", entry, "--pose-from", variant,
                        "--pose-tick", k, "--camera-fov", "%.4f" % fov, "--ticks", 0, "--screenshot", png] + flags,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                table.append({"shot": i, "craft_row": k, "camera_skew": c, "mean_grey": ce.overlap_score(theirs, png).get("mean_grey")})
        best = min((t for t in table if t["shot"] == i), key=lambda t: t["mean_grey"])
        print("shot %d: best craft row %d (lag %d) with camera of row %+d from it, mean grey %.2f" % (
            i, best["craft_row"], i - best["craft_row"], best["camera_skew"], best["mean_grey"]))
        print("   " + "  ".join("k=%d c=%+d:%.1f" % (t["craft_row"], t["camera_skew"], t["mean_grey"]) for t in table if t["shot"] == i))
    (out / "skew.json").write_text(json.dumps(table, indent=2))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--port", type=int, default=int(os.environ.get("OAG_PPSSPP_PORT", 47810)))
    ap.add_argument("--home", default=os.environ.get("PP_HOME"))
    ap.add_argument("--state", required=True)
    ap.add_argument("--track", required=True)
    ap.add_argument("--team")
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--ticks", type=int, default=14)
    ap.add_argument("--window", type=int, default=6, help="candidate rows k in [max(0, i - window), i]")
    ap.add_argument("--first", type=int, default=6, help="first shot to score; 1 scores the stops right after the CPU resumes "
                    "(a best row of 0 there means the frame is older than the record, so the lag is at least the stop's index)")
    ap.add_argument("--camera-skews", help="comma list of camera row offsets to try against each craft row (e.g. -1,0,1,2)")
    ap.add_argument("--oag-game", default=str(HERE.parent / "target/debug/oag-game"))
    args = ap.parse_args()
    pp = pair_module()
    plat = ce.platform("pulse-psp")
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    profile_dir = Path(os.environ["XDG_CONFIG_HOME"]).parent
    ce.write_ours_settings(plat, profile_dir)
    facts = ce.state_facts(args.state)
    flags = ce.ours_flags(plat) + ["--mode", facts.get("mode", "single_race"), "--class", facts.get("class", "venom")]
    if args.team:
        flags += ["--team", args.team]
    uv = ["uv", "run", "--with", "websocket-client", "python3"]
    pp.set_window(960, 544)
    pp.resume(args.port)
    pp.run(uv + [HERE / "psp-state.py", "--port", args.port, "--home", args.home, "load", "2", "--file", args.state])
    pp.freeze(args.port)
    time.sleep(6.0)
    wid, wx, wy, ww, wh = pp.set_window(*plat["output"])
    extra = []
    if facts.get("mode") == "single_race":
        rf = pp.race_field_module()
        from ppsspp_debugger import Debugger
        extra = ["--craft", hex(rf.Field(Debugger(args.port)).player["craft"])]
    shots = out / "shots"
    csv = out / "trace.csv"
    with open(csv, "w") as f:
        pp.run(uv + [HERE / "psp-trace.py", "--port", args.port, "--ticks", args.ticks, "--camera", "--shot-every", 1,
                     "--shot-dir", shots] + extra, stdout=f)
    pp.set_window(960, 544)
    entry = "Data\\Environments\\%s\\track.vex" % args.track
    results = []
    if args.camera_skews:
        skew_table(args, pp, plat, flags, entry, csv, shots, out, (ww, wh, wx, wy))
        return
    for i in range(args.first, args.ticks):
        theirs = out / ("theirs%02d.png" % i)
        pp.run(["magick", shots / ("tick%05d.png" % i), "-crop", "%dx%d+%d+%d" % (ww, wh, wx, wy), "+repage", theirs])
        scores = {}
        for k in range(max(0, i - args.window), i + 1):
            png = out / ("ours%02d_%02d.png" % (i, k))
            row = ce.csv_row(csv, k)
            fov = ce.speed_fov(row, 60.0)
            pp.run([args.oag_game, ce.MAIN / plat["image"], "--race", "--track", entry, "--pose-from", csv, "--pose-tick", k,
                    "--camera-fov", "%.4f" % fov, "--ticks", 0, "--screenshot", png] + flags,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            scores[k] = ce.overlap_score(theirs, png).get("mean_grey")
        best = min(scores, key=lambda k: scores[k])
        results.append({"shot": i, "scores": scores, "best_row": best, "lag": i - best})
        print("shot %2d  best row %2d  lag %d   %s" % (i, best, i - best,
                                                       " ".join("%d:%.2f" % (k, v) for k, v in scores.items())))
    lags = [r["lag"] for r in results]
    (out / "lag.json").write_text(json.dumps({"state": str(args.state), "results": results, "lags": lags}, indent=2))
    print("lags", lags)


if __name__ == "__main__":
    main()
