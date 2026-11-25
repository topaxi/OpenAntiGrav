#!/usr/bin/env python3
"""Ours beside a Missile-hit film, posed on the struck rival: `rpcs3-hd-missile-film.py`'s frames.

    python3 scripts/hd-hit-posed-pair.py --film DIR --team piranha [--out DIR/posed] [--only a0500,a0670]
        [--shield-percent 89] [--hit-offset 0.0] [--box X,Y,W,H]

The film's `aNNNN.json` holds the pushbuffer camera and every craft's pose at the frame. Ours' craft 0 is
posed on the struck rival (the craft nearest the player in `pre.json`, held at that pose every tick at rest:
`--pose-hold-ticks`), the camera is the film's (`--camera-pose`, `--camera-fov`), the rival's shield is set
under the hit (`--force-shield`, `--shield-percent`) and the weapon hit forced (`--force-hit`), and the run
goes on to the film frame's pool age after the hit, so the hit's own effects (the weapon spark, the damage
smoke) are the ones a ticking race draws. The original's player hull is not drawn in ours (no craft stands
at its pose) and the rival's pose is the film's (`--pose-from`: position and full basis), so the score is over a box around the rival (`--box`, default 360x280 centred on the
rival's projection) and says nothing about the rest of the frame.

Writes `ours_NN.png`, `theirs_NN.png`, `side_NN.png` and `scores.json` (mean absolute difference over the
box, and the bright-pixel count in the original and in ours: the sparks the hit throws).
"""
import argparse
import importlib.util
import json
import math
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402

MAIN = Path("/home/topaxi/projects/OpenAntiGrav")
HIT_AFTER_GO_TICKS = 5


def load_module(name, file):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def run(cmd):
    print("+", " ".join(str(c) for c in cmd), file=sys.stderr)
    subprocess.run([str(c) for c in cmd], check=True, stdout=subprocess.DEVNULL)


def project(view_proj, size, point):
    import numpy as np
    m = np.array(view_proj).reshape(4, 4)
    v = np.append(point, 1.0) @ m
    return (v[0] / v[3] * 0.5 + 0.5) * size[0], (1 - (v[1] / v[3] * 0.5 + 0.5)) * size[1]


def box_stats(path_a, path_b, box):
    from PIL import Image
    import numpy as np
    x, y, w, h = box
    a = np.asarray(Image.open(path_a).convert("RGB").crop((x, y, x + w, y + h)), dtype=float)
    b = np.asarray(Image.open(path_b).convert("RGB").resize(Image.open(path_a).size).crop((x, y, x + w, y + h)),
                   dtype=float)
    luma = lambda im: im @ [0.299, 0.587, 0.114]
    return {"mad": float(np.abs(a - b).mean()), "bright_theirs": int((luma(a) > 230).sum()),
            "bright_ours": int((luma(b) > 230).sum()), "mean_theirs": float(luma(a).mean()),
            "mean_ours": float(luma(b).mean())}


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--film", type=Path, required=True)
    ap.add_argument("--team", required=True, help="the struck rival's team (not recorded in the film)")
    ap.add_argument("--out", type=Path)
    ap.add_argument("--only", help="comma separated frame stems")
    ap.add_argument("--shield-percent", type=float, default=89.0, help="the rival's shield under the hit, percent")
    ap.add_argument("--hit-offset", type=float, default=0.0, help="seconds added to the film's pool age")
    ap.add_argument("--box", help="X,Y,W,H of the scored box in the film frame's pixels")
    ap.add_argument("--image", default=str(MAIN / "data/images/hdfury-ps3-eu-dec.iso"))
    ap.add_argument("--oag-game", default=str(HERE.parent / "target/debug/oag-game"))
    ap.add_argument("--platform", default="hd-fury")
    ap.add_argument("--track", default="/data/environments/talons_junction/track.vex")
    ap.add_argument("--extra", nargs=argparse.REMAINDER, default=[], help="flags passed to ours as they are")
    args = ap.parse_args()
    out = (args.out or args.film / "posed").resolve()
    out.mkdir(parents=True, exist_ok=True)
    plat = ce.platform(args.platform)
    go = ce.go_tick(plat)
    scratch = out / "ours-profile"
    ce.write_ours_settings(plat, scratch, None)
    os.environ.update(ce.ours_env(scratch))
    pre = json.loads((args.film / "pre.json").read_text())["ship"]
    player = next(c for c in pre["craft"] if c["player"])
    rival = min((c for c in pre["craft"] if not c["player"]), key=lambda c: math.dist(c["pos"], player["pos"]))["ship"]
    wanted = set(args.only.split(",")) if args.only else None
    scores = {}
    for record_path in sorted(args.film.glob("a[0-9][0-9][0-9][0-9].json")):
        stem = record_path.stem
        if wanted and stem not in wanted:
            continue
        record = json.loads(record_path.read_text())
        camera, ship = record.get("camera"), record["ship"]
        if not camera:
            print("%s: no pushbuffer camera, skipped" % stem)
            continue
        craft = next(c for c in ship["craft"] if c["ship"] == rival)
        hit_tick = go + HIT_AFTER_GO_TICKS
        ticks = hit_tick + round((ship["pool_age"] + args.hit_offset) * 60)
        row = {"shield": 0.0}
        for name, vec in (("right", craft["rows"][0]), ("up", craft["rows"][1]), ("fwd", craft["rows"][2]),
                          ("pos", craft["pos"]), ("cam_pos", camera["eye"])):
            for axis, value in zip("xyz", vec):
                row["%s_%s" % (name, axis)] = value
        # The recorded camera rows are the world axes as columns (`oag_trace::replay::camera_orientation_of`):
        # right, up and the back axis (minus the look direction).
        axes = (camera["right"], camera["up"], [-v for v in camera["forward"]])
        for r, name in enumerate(("cam_right", "cam_up", "cam_fwd")):
            for c, axis in enumerate("xyz"):
                row["%s_%s" % (name, axis)] = axes[c][r]
        trace = out / ("pose_%s.csv" % stem)
        ce.write_trace_row(trace, row)
        flags = ce.ours_flags(plat)
        from PIL import Image
        theirs = Image.open(record["screenshot"])
        flags[flags.index("--size") + 1] = "%dx%d" % theirs.size
        cmd = [args.oag_game, args.image, "--race", "--track", args.track, "--team", args.team.upper(),
               "--mode", "single_race", "--pose-from", trace,
               "--camera-fov", "%.4f" % camera["fov_y_deg"], "--pose-hold",
               "--pose-hold-ticks", "--force-shield", "%d:%.1f" % (hit_tick - 1, args.shield_percent),
               "--force-hit", "%d:0" % hit_tick] + flags + args.extra
        ours = out / ("ours_%s.png" % stem)
        run(cmd + ["--ticks", ticks, "--screenshot", ours])
        theirs.save(out / ("theirs_%s.png" % stem))
        run(["magick", "montage", out / ("theirs_%s.png" % stem), ours, "-tile", "2x1", "-geometry", "+2+2",
             out / ("side_%s.png" % stem)])
        if args.box:
            box = [int(v) for v in args.box.split(",")]
        else:
            x, y = project(camera["view_proj"], theirs.size, craft["pos"])
            box = [max(0, int(x) - 180), max(0, int(y) - 140), 360, 280]
        scores[stem] = {"box": box, "age": ship["pool_age"], "ticks": ticks, **box_stats(out / ("theirs_%s.png" % stem), ours, box)}
        print(stem, json.dumps(scores[stem]), flush=True)
    (out / "scores.json").write_text(json.dumps(scores, indent=1))


if __name__ == "__main__":
    main()
