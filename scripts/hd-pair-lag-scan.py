#!/usr/bin/env python3
"""How many ticks older than the RAM read is a moving HD race frame? Render ours rewound and score each.

    python3 scripts/hd-pair-lag-scan.py <pair-dir> [CAMERA:CRAFT ...] [--oag-game PATH]

A `--field` pair (`scripts/rpcs3-hd-pair.py`) draws ours at the RAM state read in the pause. The original's frame is
the one the RSX presented, which on a moving craft can be older than that read (`docs/tools/compare-page.md`, "HD:
the picture is not always the RAM instant"). Each `CAMERA:CRAFT` pair is a number of 60 Hz ticks: the camera eye is
moved back along the player's recorded velocity by the first, every craft (the player and the field) back along its
own recorded velocity by the second, and ours is rendered with the pair's own flags (`pair.json` `ours.flags`,
`field.csv`, `ours-profile`) at that state. Printed per pair: edge correlation and RMSE against `theirs.png`
(`compare_env.overlap_score`). A sharp peak away from `0:0` is a lag; no peak is no lag or a frame this metric
cannot read. Orientation is not rewound, so keep the ticks few. This reads the original's RAM and our render only;
nothing it writes is a pair (the renders go to `<pair-dir>/lag/`).
"""
import argparse
import csv
import json
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402

ROOT = HERE.parent
MAIN = Path("/home/topaxi/projects/OpenAntiGrav")


def rewound_field(rows, ticks):
    out = []
    for r in rows:
        r2 = dict(r)
        for a in "xyz":
            r2["pos_" + a] = repr(float(r["pos_" + a]) - float(r["vel_" + a]) * ticks / 60.0)
        out.append(r2)
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("pair", type=Path)
    ap.add_argument("ticks", nargs="*", default=["0:0", "1:1", "2:2", "2:3", "3:3", "4:4"])
    ap.add_argument("--oag-game", default=str(ROOT / "target/debug/oag-game"))
    ap.add_argument("--image", default=str(MAIN / "data/images/hdfury-ps3-eu-dec.iso"))
    args = ap.parse_args()
    pair = args.pair.resolve()
    rec = json.loads((pair / "pair.json").read_text())
    flags = list(rec["ours"]["flags"])
    rows = list(csv.DictReader((pair / "field.csv").open()))
    player = rows[0]
    vel = [float(player["vel_" + a]) for a in "xyz"]
    i_cam = next(i for i, f in enumerate(flags) if f.startswith("--camera-pose="))
    cam = [float(x) for x in flags[i_cam].split("=")[1].split(",")]
    (pair / "lag").mkdir(exist_ok=True)
    env = dict(os.environ, **ce.ours_env(pair / "ours-profile" / "profile"))
    print("%-8s %-9s %-7s" % ("cam:craft", "edge_ncc", "rmse"))
    for spec in args.ticks:
        t_cam, t_craft = (float(x) for x in spec.split(":"))
        name = spec.replace(":", "_").replace(".", "p")
        field = pair / "lag" / ("field%s.csv" % name)
        with field.open("w", newline="") as f:
            w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
            w.writeheader()
            w.writerows(rewound_field(rows, t_craft))
        eye = [c - v * t_cam / 60.0 for c, v in zip(cam[:3], vel)] + cam[3:]
        fl = list(flags)
        fl[i_cam] = "--camera-pose=" + ",".join("%.6f" % x for x in eye)
        fl[fl.index("--field-from") + 1] = str(field)
        png = pair / "lag" / ("ours%s.png" % name)
        subprocess.run([args.oag_game, args.image, *fl, "--ticks", "0", "--screenshot", str(png)],
                       env=env, capture_output=True, check=True)
        score = ce.overlap_score(pair / "theirs.png", png)
        print("%-8s %-9.4f %-7.3f" % (spec, score.get("edge_ncc", float("nan")), score.get("rmse", float("nan"))))


if __name__ == "__main__":
    main()
