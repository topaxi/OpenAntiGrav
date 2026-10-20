#!/usr/bin/env python3
"""Run `psp-pulse-pair.py` over every kept Pulse PSP Time Trial state, one PPSSPP instance, one after the other.

    source <scratch>/pp/env.sh      # DISPLAY, PP_HOME, OAG_PPSSPP_PORT (scripts/ppsspp-start.sh with PPSSPP_EXTRA_INI)
    OAG_SHOT_DISPLAY=$DISPLAY XDG_CONFIG_HOME=<scratch>/profile/cfg ... \\
    uv run --with websocket-client python3 scripts/compare-pulse-psp-batch.py --out data/reference/compare-page/pulse-psp \\
        [--only moa-therma-white] [--states data/saves/pulse-psp]

The circuit directory of each state is a lookup; the layout file is NOT: `psp-pulse-pair.py --layout auto` picks
it from the original's grid pose and refuses a pair whose start is not within 25 units. A refused pair is
reported and the batch continues; the exit status is the number of refused pairs.
"""

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent

#: state name stem (`time-trial-<stem>-venom-grid`) -> (circuit title, circuit directory)
CIRCUITS = {
    "basilico": ("Basilico", "01_Track"), "metropia": ("Metropia", "02_Track"),
    "moa-therma": ("Moa Therma", "03_Track"), "tech-de-ra": ("Tech de Ra", "04_Track"),
    "de-konstruct": ("De Konstruct", "05_Track"), "vertica": ("Vertica", "06_Track"),
    "outpost-7": ("Outpost 7", "07_Track"), "amphiseum": ("The Amphiseum", "09_Track"),
    "arc-prime": ("Arc Prime", "10_Track"), "platinum-rush": ("Platinum Rush", "13_Track"),
    "fort-gale": ("Fort Gale", "14_Track"), "talons": ("Talon's Junction", "16_Track"),
}


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--states", type=Path, default=ROOT / "data/saves/pulse-psp")
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--only", help="substring of the state file name")
    ap.add_argument("--port", type=int, default=int(os.environ.get("OAG_PPSSPP_PORT", 0)))
    ap.add_argument("--home", default=os.environ.get("PP_HOME"))
    args = ap.parse_args()
    refused = 0
    for state in sorted(args.states.glob("time-trial-*.ppst")):
        name = state.stem
        if args.only and args.only not in name:
            continue
        stem = name[len("time-trial-"):].replace("-venom-grid", "")
        colour = "black" if stem.endswith("-black") else "white"
        circuit = stem.removesuffix("-black").removesuffix("-white")
        title, track = CIRCUITS[circuit]
        out = args.out / ("%s-%s" % (circuit, colour))
        meta = {"id": "pulse-psp-%s-%s" % (circuit, colour), "title": "%s %s, the grid" % (title, colour.capitalize()),
                "track": "%s %s (%s), Time Trial" % (title, colour.capitalize(), track),
                "craft": "Venom class, the state's own team (ours: the title's default team)",
                "state": "Time Trial start, race clock matched to the original's (read from its RAM); ship and camera pose read from the original, ours rendered from that row"}
        cmd = ["uv", "run", "--with", "websocket-client", "python3", HERE / "psp-pulse-pair.py", "--port", args.port,
               "--home", args.home, "--out", out, "--track", track, "--state", state, "--meta", json.dumps(meta)]
        print("== %s" % name, flush=True)
        done = subprocess.run([str(c) for c in cmd], capture_output=True, text=True)
        (out.parent / (out.name + ".log")).write_text(done.stderr) if out.parent.exists() else None
        lines = [ln for ln in done.stderr.splitlines() if ln.startswith(("ok", "MISMATCH", "REFUSED", "parity"))]
        print("\n".join(lines), flush=True)
        if done.returncode:
            refused += 1
            print("-> FAILED rc=%d" % done.returncode, flush=True)
    sys.exit(refused)


if __name__ == "__main__":
    main()
