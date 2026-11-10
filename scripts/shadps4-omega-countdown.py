#!/usr/bin/env python3
"""A run of frames across Wipeout Omega Collection's start countdown on shadPS4, with throttle held through the release.

    python3 scripts/shadps4-omega-pair.py prepare-game --game data/scratch/<lane>/omega-game       # once
    uv run --with evdev python3 scripts/emu-run.py --child-status <log-dir>/status.json -- \\
        python3 scripts/shadps4-omega-countdown.py --scratch data/scratch/<lane>/omega \\
        --game data/scratch/<lane>/omega-game --out data/scratch/<lane>/countdown --seconds 26

Uses the rig of `shadps4-omega-pair.py` (headless `sway`, muted SDL audio, one uinput pad). The walk is RACEBOX, one Right on
GAME MODE (WipEout 2048), Track Select (the first circuit, ALTIMA), Ship Select (the first craft), the loading screen and
START RACE. Throttle (cross) is then held, so the first frame the hull moves is the release; every grab is stamped with the
monotonic clock in `frames.json`, so the frame a digit or `GO` shows on can be set against it.
"""

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import importlib  # noqa: E402

op = importlib.import_module("shadps4-omega-pair")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--scratch", required=True)
    ap.add_argument("--game", required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--seconds", type=float, default=26.0, help="how long to grab after START RACE")
    ap.add_argument("--hold", type=float, default=22.0, help="seconds to hold throttle from START RACE")
    args = ap.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    rig = op.Rig(args.scratch, args.game)
    try:
        rig.write_config()
        rig.start_sway()
        rig.start_shadps4()
        rig.start_pad()
        probe = rig.dir / "probe.png"
        for waited in range(0, 150, 3):
            time.sleep(3)
            if (rig.dir / "shadps4.log").read_text(errors="replace").count("dumped core"):
                sys.exit("STALLED at boot: shadPS4 dumped core")
            try:
                rig.grab(probe)
                r, g, b = op.pixel(probe, *op.MENU_RED)
            except subprocess.CalledProcessError:
                continue
            if r > 200 and g < 40 and b < 40:
                op.log("MAIN MENU after about %d s" % (waited + 3))
                break
        else:
            sys.exit("STALLED at boot: no MAIN MENU in 150 s (see shadps4.log)")
        rig.press("right")
        rig.press("cross", 3)  # Race Box
        rig.press("right", 1.5)  # GAME MODE: WipEout 2048
        rig.grab(out / "racebox.png")
        rig.press("cross", 3)  # Track Select, ALTIMA
        rig.grab(out / "track-select.png")
        rig.press("cross", 3)  # Ship Select
        rig.grab(out / "ship-select.png")
        rig.press("cross", 3)  # loading
        time.sleep(12)
        rig.grab(out / "loading.png")
        time.sleep(14)  # the START RACE prompt waits for the confirm
        rig.press("cross", 0.2)
        frames, t0, held_at = [], time.monotonic(), None
        while time.monotonic() - t0 < args.seconds:
            at = time.monotonic() - t0
            path = rig.grab(out / ("f%03d.png" % len(frames)))
            frames.append({"frame": len(frames), "t": round(at, 3)})
            if held_at is None and len(frames) % 30 == 0 and len(frames) <= 120:
                # Omega's 2048 mode plays a flyby after START RACE; a further confirm skips it
                rig.hold.write(b"a press cross\n")
            if held_at is None and op.hud_up(path):
                # the HUD is up: the countdown has begun, hold the throttle through the release
                rig.hold.write(b"a hold cross %.1f\n" % args.hold)
                held_at = len(frames) - 1
        frames_meta = {"hud_first_frame": held_at}
        (out / "frames.json").write_text(json.dumps({"seconds": args.seconds, "hold": args.hold, **frames_meta, "frames": frames}, indent=1))
        op.log("%d frames in %.1f s" % (len(frames), time.monotonic() - t0))
    finally:
        rig.stop()


if __name__ == "__main__":
    main()
