#!/usr/bin/env python3
"""Ours beside a Missile-hit film: `rpcs3-hd-missile-film.py`'s frames, rendered from the same camera relative to the blast.

    python3 scripts/hd-missile-film-pair.py --film data/reference/hd-capture/missile-hit/hit2 \\
        --out data/reference/hd-capture/missile-hit/ours-hit2 [--without-streaks-env NAME]

Ours' scenario is `--force-missile-hit 289:1` on Talon's Junction (the one `hd_missile_blast_ground_truth` runs):
the blast is at a fixed place and its age `a` is tick `290 + round(60 a)`. For every film frame (`NN.json` with
the pool age and the pushbuffer camera) this puts ours' camera where the original's was *relative to the blast
centre*: `eye_ours = centre_ours + (eye_film - centre_film)`, the film's forward, up and field of view, so the
eye-to-blast distance and bearing match at every age and the player's own motion (the original's player flies
through the blast at 100 to 150 units/s) is part of the camera. A frame the pushbuffer gave no camera for takes
the nearest frame's camera moved by the player's displacement between the two (`ship.pos`). Three renders per
frame: `with` (the streaks as wired), `without` (`--without-streaks-env`: an environment switch a build of ours
carries locally, never committed; omitted, the column is left out) and `base`, tick 289 from the same camera
(no blast), the backdrop's own luma. `luma.json` has the whole-frame mean luma (Rec. 601) of every frame.
"""
import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MAIN = Path("/home/topaxi/projects/OpenAntiGrav")
OURS_CENTRE = (-121.987, -50.615, -175.324)
#: Ours' road runs along +x at the blast (the player rests 9.8 units behind it, tick 289).
OURS_ROAD = (1.0, 0.0)
HIT_TICK = 289
FIRE_SCRIPT = "285 none\n1 square\n400 none\n"


def luma(path):
    out = subprocess.run(["magick", str(path), "-format", "%[fx:(0.299*mean.r+0.587*mean.g+0.114*mean.b)*255]", "info:"],
                         capture_output=True, text=True, check=True).stdout
    return float(out)


def film_frames(film):
    frames = []
    for path in sorted(film.glob("[a-z]*[0-9].json")) + sorted(film.glob("pre.json")):
        if path.stem == "steps":
            continue
        r = json.loads(path.read_text())
        frames.append({"stem": path.stem, "age": r["ship"]["pool_age"], "pos": r["ship"]["pos"],
                       "heading": r["ship"]["rows"][2],
                       "centre": r["ship"].get("centre"), "camera": r.get("camera"), "png": path.with_suffix(".png")})
    return frames


def fill_cameras(frames):
    """A frame with no pushbuffer camera borrows the nearest frame's, translated by the player's displacement."""
    have = [f for f in frames if f["camera"]]
    for f in frames:
        if f["camera"]:
            f["camera_source"] = "pushbuffer"
            continue
        near = min(have, key=lambda g: abs(g["age"] - f["age"]))
        shift = [f["pos"][i] - near["pos"][i] for i in range(3)]
        cam = dict(near["camera"])
        cam["eye"] = [near["camera"]["eye"][i] + shift[i] for i in range(3)]
        f["camera"] = cam
        f["camera_source"] = "borrowed from %s" % near["stem"]


def road_spin(heading, align):
    """A rotation about the vertical taking the film's heading at the hit to ours' road direction, so a camera
    that follows the original's road follows ours: the two hits sit on differently oriented stretches."""
    import math
    theta = (math.atan2(OURS_ROAD[1], OURS_ROAD[0]) - math.atan2(heading[2], heading[0])) if align else 0.0
    c, s = math.cos(theta), math.sin(theta)
    return lambda v: [v[0] * c - v[2] * s, v[1], v[0] * s + v[2] * c]


def render(args, scratch, frame, tick, out, extra_env):
    cam = frame["camera"]
    spin = frame["spin"]
    offset = spin([cam["eye"][i] - frame["centre"][i] for i in range(3)])
    eye = [OURS_CENTRE[i] + offset[i] for i in range(3)]
    numbers = ",".join("%.6f" % v for v in eye + spin(list(cam["forward"])) + spin(list(cam["up"])))
    env = dict(os.environ, XDG_CONFIG_HOME=str(scratch / "config"), XDG_DATA_HOME=str(scratch / "data"),
               XDG_STATE_HOME=str(scratch / "state"))
    env.update(extra_env)
    cmd = [args.oag_game, args.image, "--race", "--no-audio", "--size", "960x544", "--opponents",
           "--render-scale", "100", "--msaa", "off", "--screen-filter", "off", "--anisotropy", "off",
           "--motion-blur", "off", "--ticks", str(tick), "--give", "missile", "--force-missile-hit",
           "%d:1" % HIT_TICK, "--camera-pose=" + numbers, "--camera-fov", "%.4f" % cam["fov_y_deg"],
           "--input-script", str(scratch / "fire.inputs"), "--screenshot", str(out)]
    subprocess.run(["systemd-run", "--user", "--scope", "-q", "-p", "MemoryMax=12G", "-p", "MemorySwapMax=0"] + cmd,
                   env=env, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--film", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--oag-game", default=str(HERE.parent / "target/debug/oag-game"))
    ap.add_argument("--image", default=str(MAIN / "data/images/hdfury-ps3-eu-dec.iso"))
    ap.add_argument("--without-streaks-env", help="environment variable that makes this build start no streaks")
    ap.add_argument("--env", action="append", default=[], metavar="NAME=VALUE",
                    help="environment for every render of ours (a local build's experiment switch)")
    ap.add_argument("--only", help="comma separated frame stems")
    ap.add_argument("--no-align", action="store_true", help="do not turn the film's road onto ours'")
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    scratch = args.out / "profile"
    scratch.mkdir(exist_ok=True)
    (scratch / "fire.inputs").write_text(FIRE_SCRIPT)
    frames = film_frames(args.film)
    fill_cameras(frames)
    centre = next(f["centre"] for f in frames if f["centre"])
    spin = road_spin(next(f["heading"] for f in frames if f["stem"] != "pre"), not args.no_align)
    for f in frames:
        f["centre"] = f["centre"] or centre
        f["spin"] = spin
    wanted = set(args.only.split(",")) if args.only else None
    kept = args.out / "luma.json"
    table = json.loads(kept.read_text()) if kept.exists() else []
    for f in frames:
        if wanted and f["stem"] not in wanted:
            continue
        tick = HIT_TICK + 1 + round(f["age"] * 60) if f["stem"] != "pre" else HIT_TICK
        row = {"stem": f["stem"], "age": f["age"], "tick": tick, "camera_source": f["camera_source"],
               "original": luma(f["png"])}
        variants = [("with", {}), ("base", {})]
        if args.without_streaks_env:
            variants.insert(1, ("without", {args.without_streaks_env: "1"}))
        shared = dict(e.split("=", 1) for e in args.env)
        for name, env in variants:
            env = {**shared, **env}
            png = args.out / ("%s_%s.png" % (f["stem"], name))
            render(args, scratch, f, HIT_TICK if name == "base" else tick, png, env)
            row[name] = luma(png)
        table = [r for r in table if r["stem"] != row["stem"]] + [row]
        print(json.dumps(row), flush=True)
        kept.write_text(json.dumps(sorted(table, key=lambda r: r["age"]), indent=1))


if __name__ == "__main__":
    sys.exit(main())
