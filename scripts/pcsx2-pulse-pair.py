#!/usr/bin/env python3
"""One original-vs-ours pair of Wipeout Pulse (PS2) from a savestate.

    source <scratch>/ps2/env.sh      # OAG_PCSX2_DISPLAY, OAG_PCSX2_DATA, private XDG_*
    uv run --with python-xlib python3 scripts/pcsx2-pulse-pair.py \\
        --slot 28097 --state data/saves/pulse-ps2/<point>.p2s --track 03_Track --out <dir>

Needs a booted PCSX2 on the private data directory (`pcsx2-drive.py boot`). Copies
`--state` into the instance's slot 1, loads it, waits, takes the exact GS
screenshot, captures three ticks of ship pose with `pcsx2-trace.py --state-file`
(the craft pointer comes from the state's own memory or its `.json` sidecar,
`pcsx2_state.py`), and renders ours from that row with `oag-game --race --pose-from`.
Writes `theirs.png`, `ours.png`, `side.png` and `trace.csv`. Only the ship pose is
carried: our camera is our own chase camera, not the original's.
"""

import argparse
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent


def run(cmd, **kw):
    print("+", " ".join(str(c) for c in cmd), file=sys.stderr)
    return subprocess.run([str(c) for c in cmd], check=True, **kw)


def resume(slot):
    """Run the VM: a previous `pcsx2-trace.py` leaves it paused, and the GS screenshot
    hotkey does nothing on a paused one (`screenshot FAILED`)."""
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
    ap.add_argument("--image", default=str(ROOT / "data/images/pulse-ps2-eu.chd"))
    ap.add_argument("--serial", default="SCES-54748 (F8AE6FF2)",
                    help="the state file name's game part in PCSX2's sstates directory")
    args = ap.parse_args()
    data = os.environ.get("OAG_PCSX2_DATA")
    if not data or "OAG_PCSX2_DISPLAY" not in os.environ:
        sys.exit("source the lane's own env: OAG_PCSX2_DATA and OAG_PCSX2_DISPLAY must be set")
    if not os.environ.get("XDG_CONFIG_HOME", "").startswith(str(ROOT.parent.parent / "OpenAntiGrav" / "data" / "scratch")):
        sys.exit("XDG_CONFIG_HOME must be under data/scratch/<lane>/ (oag-game writes its profile there)")
    args.out.mkdir(parents=True, exist_ok=True)
    states = Path(data) / "PCSX2" / "sstates"
    states.mkdir(parents=True, exist_ok=True)
    shutil.copy(args.state, states / ("%s.01.p2s" % args.serial))
    uv = ["uv", "run", "--with", "python-xlib", "python3"]
    drive = uv + [HERE / "pcsx2-drive.py", "--slot", args.slot]
    run(drive + ["state", "load", "1"])
    resume(args.slot)
    time.sleep(3)
    run(drive + ["shot", args.out / "theirs.png"])
    csv = args.out / "trace.csv"
    run([sys.executable, HERE / "pcsx2-trace.py", "--slot", args.slot, "--state-file", args.state,
         "--from-state", 1, "--ticks", 3, "--out", csv])
    entry = "Data\\Environments\\%s\\track%s.vex" % (args.track, "_reversed" if args.reversed else "")
    size = subprocess.run(["magick", "identify", "-format", "%wx%h", args.out / "theirs.png"],
                          capture_output=True, text=True, check=True).stdout
    cmd = [ROOT / "target/debug/oag-game", args.image, "--no-audio", "--race", "--track", entry,
           "--screenshot", args.out / "ours.png", "--ticks", 0, "--size", size,
           "--pose-from", csv, "--pose-tick", 0]
    if args.team:
        cmd += ["--team", args.team]
    run(cmd, stdout=subprocess.DEVNULL)
    run(["magick", "montage", args.out / "theirs.png", args.out / "ours.png", "-tile", "2x1",
         "-geometry", "+2+2", args.out / "side.png"])
    print("wrote", args.out / "side.png")


if __name__ == "__main__":
    main()
