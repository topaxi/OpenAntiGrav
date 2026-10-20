#!/usr/bin/env python3
"""The original's Autopilot flying a lone craft in Zone, per frame: pose, speed, shield.

Pulse cannot pick Zone on a fresh profile, so this patches it the way
`scripts/psp-launch-boost.py --game-mode 6` does: `g_game_mode` (`0x08b31048`)
written to 6 on a loaded Time Trial race, and again every logged frame. Mode 6
is what `Ship_UpdateEngine`'s auto-speed and `Ship_UpdateHover`'s four-corner
selector read (`docs/ghidra/functions/psp-pulse-usa/zone-start.md`,
`zone-hover.md`). The craft is then handed to the original's own player driver
through the Autopilot pickup with a huge timer, `scripts/psp-autopilot-rail.py`'s
`arm`, and one stop a frame in `Ai_SnapToLane` logs:

- the body's position and velocity, the ground fraction (`craft+0x2b0`), the
  contact count (`body+0x370`);
- the shield (`entity+0x88`), the craft state (`entity+0x8c`) and the zone
  counter (`craft+0x28c`, what the auto-speed multiplies);
- `g_game_mode` as read back, so a row the patch did not hold is visible;
- the engine's inputs: throttle (`craft+0x2b8`), thrust (`craft+0x328`) and the
  launch multiplier (`craft+0x294`), so the auto-speed's `(34 + 4.4 * zone) * 2`
  can be read off a row.

`--exclude N` writes the player's driver's fork decision directly, after the
state has loaded: `Ai_ChooseBranch`'s state `ai+0x58 = 1` and its excluded
path `ai+0x60 = N` (`docs/ghidra/functions/psp-pulse-usa/ai-branch-choice.md`),
so the driver takes the other successor of the fork it is approaching. A patch
of the coin instruction itself does not reach PPSSPP's compiled block.

    source data/scratch/<lane>/pp/env.sh
    uv run --with websocket-client python3 scripts/psp-zone-autopilot.py \\
        --port $OAG_PPSSPP_PORT --out data/scratch/<lane>/zone-autopilot.json

Load a grid state first (`scripts/psp-state.py load 2 --file ...`). Raw captures
are derived game data: keep them under `data/`, never commit them.
"""

import argparse
import importlib.util
import json
import struct
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger  # noqa: E402

_spec = importlib.util.spec_from_file_location(
    "rail", Path(__file__).resolve().parent / "psp-autopilot-rail.py"
)
rail = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(rail)

G_GAME_MODE = 0x08B31048
SHIELD = 0x88
STATE = 0x8C
ZONE_COUNTER = 0x28C


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--game-mode", type=int, default=6)
    parser.add_argument("--arm-after", type=int, default=1)
    parser.add_argument("--frames", type=int, default=2400)
    parser.add_argument("--budget", type=float, default=540.0)
    parser.add_argument("--exclude", type=int, help="the path the driver's fork decision excludes")
    args = parser.parse_args()
    args.steer = None
    args.stint = None

    dbg = Debugger(args.port)
    log = {"args": {k: str(v) for k, v in vars(args).items()}, "events": [], "frames": []}
    start = time.time()
    try:
        dbg.brk()
        dbg.write_u32(G_GAME_MODE, args.game_mode)
        if args.exclude is not None:
            p = rail.player(dbg)
            if p is None:
                raise SystemExit("no player craft to write a decision into")
            ai = p["driver"]
            before = [dbg.read_u32(ai + 0x58), dbg.read_u32(ai + 0x5C), dbg.read_u32(ai + 0x60)]
            dbg.write_u32(ai + 0x58, 1)
            dbg.write_u32(ai + 0x60, args.exclude)
            log["events"].append({"branch_state_before": before, "excluded": args.exclude})
        dbg.resume()
        dbg.hold(cross=True)
        p = rail.arm(dbg, args, log)
        dbg.write_u32(G_GAME_MODE, args.game_mode)
        dbg.add_breakpoint(rail.SNAP_AFTER_LANE_POINT, "s0 == 0x%x" % p["driver"])
        try:
            for n in range(args.frames):
                if time.time() - start > args.budget:
                    log["events"].append({"stopped": "budget", "samples": n})
                    break
                dbg.call("cpu.status")
                dbg.pending = []
                dbg.call("cpu.resume")
                try:
                    dbg.wait_for_break(rail.SNAP_AFTER_LANE_POINT, timeout=30.0)
                except TimeoutError:
                    status = dbg.call("cpu.status")
                    if not (status["stepping"] and status["pc"] == rail.SNAP_AFTER_LANE_POINT):
                        raise
                status = dbg.call("cpu.status")
                mode = dbg.read_u32(G_GAME_MODE)
                if mode != args.game_mode:
                    dbg.write_u32(G_GAME_MODE, args.game_mode)
                pose = rail.f32s(dbg, p["body"], 16)
                row = {
                    "cycle_frame": int(status["ticks"] / rail.CYCLES_PER_FRAME),
                    "pos": pose[12:15],
                    "vel": rail.f32s(dbg, p["body"] + 0x140, 3),
                    "shield": rail.f32s(dbg, p["entity"] + SHIELD, 1)[0],
                    "state": dbg.read_u32(p["entity"] + STATE),
                    "zone": dbg.read_u32(p["craft"] + ZONE_COUNTER),
                    "grounded": rail.f32s(dbg, p["craft"] + 0x2B0, 1)[0],
                    "contacts": dbg.read_u32(p["body"] + 0x370),
                    "mode_read": mode,
                    "throttle_2b8": rail.f32s(dbg, p["craft"] + 0x2B8, 1)[0],
                    "thrust_328": rail.f32s(dbg, p["craft"] + 0x328, 1)[0],
                    "mul_294": rail.f32s(dbg, p["craft"] + 0x294, 1)[0],
                }
                log["frames"].append(row)
                if n % 300 == 0:
                    v = row["vel"]
                    print("sample %d pos %s speed %.1f shield %.2f zone %d mode %d" % (
                        n, [round(c, 1) for c in row["pos"]],
                        (v[0] ** 2 + v[1] ** 2 + v[2] ** 2) ** 0.5, row["shield"], row["zone"], mode),
                        file=sys.stderr)
        finally:
            dbg.brk()
            dbg.remove_breakpoint(rail.SNAP_AFTER_LANE_POINT)
            dbg.hold(cross=False)
            dbg.resume()
    finally:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(log))
        print("wrote %s (%d frames)" % (args.out, len(log["frames"])), file=sys.stderr)


if __name__ == "__main__":
    main()
