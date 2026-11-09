#!/usr/bin/env python3
"""Render ours' frames of a weapon absorb on the player's craft, one `oag-game` run per age, and the matching no-absorb control.

    scripts/absorb-ours-frames.py --out data/scratch/<lane>/ours --circle-tick 780 --frames 5,10,15,22,30 \\
        --step-ticks 2 --lag-ticks 2 -- <image> --race --track ... --pose-from <trace.csv> --pose-tick 0 --pose-hold ...

Everything after `--` is the `oag-game` command line (binary first), without `--ticks`, `--screenshot`, `--give` or
`--input-script`. For each frame `f` of the original's capture the run is `--give mine` with circle pressed at
`--circle-tick`, shot at tick `circle-tick + f * step-ticks - lag-ticks` (the original's frame step in ours' ticks, and
the screenshot's lag behind the stop; `docs/rendering/weapon-absorb-pairs.md`, "Method"). `--control` shoots the same
ticks with no `--give` and no press. Writes `<out>/f<NN>.png` or `<out>/control_f<NN>.png`.
"""
import argparse
import subprocess
import sys
from pathlib import Path


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--circle-tick", type=int, required=True)
    ap.add_argument("--frames", required=True, help="comma list of the original's frame indices")
    ap.add_argument("--step-ticks", type=int, default=1, help="ours' ticks per frame of the original (Pulse 1, Pure 2)")
    ap.add_argument("--lag-ticks", type=int, default=2)
    ap.add_argument("--give", default="mine")
    ap.add_argument("--control", action="store_true")
    ap.add_argument("command", nargs=argparse.REMAINDER)
    args = ap.parse_args()
    command = [c for c in args.command if c != "--"]
    args.out.mkdir(parents=True, exist_ok=True)
    script = args.out / "press.inputs"
    script.write_text("%d none\n1 circle\n600 none\n" % args.circle_tick)
    for frame in (int(f) for f in args.frames.split(",")):
        tick = args.circle_tick + frame * args.step_ticks - args.lag_ticks
        name = ("control_f%02d.png" if args.control else "f%02d.png") % frame
        extra = [] if args.control else ["--give", args.give, "--input-script", str(script)]
        cmd = command + extra + ["--ticks", str(tick), "--screenshot", str(args.out / name)]
        print("+", " ".join(cmd), file=sys.stderr)
        subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL)


if __name__ == "__main__":
    main()
