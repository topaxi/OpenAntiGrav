#!/usr/bin/env python3
"""The original's autopilot driver, logged for an open-loop replay of its controller.

Arms the player's Autopilot pickup after GO the way `scripts/psp-autopilot-rail.py`
does, then stops once a frame inside `Ai_ComputeControls` (`0x08855dc4`) for the
player's driver, at `0x08856058`: `Ai_SampleLookahead` has filled the lookahead
sample at `sp+0x40` and `Ai_LanePoint` the target at `sp+0xb0`, and nothing of
this frame's steering is written yet. It logs everything `steer::steer` and
`steer::pitch` read, so each frame can be fed to this engine's controller and
compared with what the original wrote:

- the racer record `*(driver+0)`: `+0x10` position, `+0x20` up, `+0x30` forward,
  `+0x40` left, `+0x50` velocity, `+0x60` angular velocity (what the yaw
  prediction reads), and its own sample `+0x260` (`SplinePt`: position,
  tangent, down, lateral, progress `+0x40`, bounds `+0x4c`/`+0x50`, racing line
  `+0x54`);
- the lookahead sample `sp+0x40` and the target `sp+0xb0`;
- the driver: smoothed speed `+0x170`, steering state `+0x174[0..3]` (last heading
  error, not yet updated this frame), `+0x190` last forward (pitch's rate term,
  not yet updated), rail blend `+0x48`, and the control record `+0x08` (steer,
  thrust, left, right, pitch), which still holds **the previous frame's** output:
  pair frame N's inputs with frame N+1's record;
- the craft: ground fraction `+0x2b0`, `+0x1c0`, hull up `+0x160`, the located
  sample's down `*(craft+0x1c4) + 0xb10` (the pitch gate `FUN_088492bc`), the
  in-air bias `*(craft+0x6c) + 0x90`, and the consumed controls `+0x2b8..`.

    uv run --with websocket-client python3 scripts/psp-autopilot-air.py \\
        --port 45697 --out data/scratch/<lane>/air.json --restart --frames 1650
    OAG_AIR_REPLAY=data/scratch/<lane>/air.replay.txt cargo nextest run -p oag-ai \\
        --test air_replay --run-ignored all --no-capture

`--at pitch` stops at `Ai_ComputePitch`'s return instead, for the pitch replay.

One breakpoint at a time: PPSSPP v1.20.4 fires only the most recently added one.
Raw captures are derived game data: keep them under `data/`, never commit them.
"""

import argparse
import importlib.util
import json
import sys
import time
from pathlib import Path

_spec = importlib.util.spec_from_file_location(
    "rail", Path(__file__).resolve().parent / "psp-autopilot-rail.py"
)
rail = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(rail)

AFTER_TARGET = 0x08856058
PITCH_RETURN = 0x088533B4


def sample(dbg, address):
    """A `SplinePt`: position, tangent, down, lateral, then the scalars."""
    v = rail.f32s(dbg, address, 22)
    return {"pos": v[0:3], "tangent": v[4:7], "down": v[8:11], "lateral": v[12:15],
            "progress": v[16], "bound_left": v[19], "bound_right": v[20], "racing_line": v[21]}


def flatten(frames, at):
    """One line per frame for `crates/ai/tests/air_replay.rs` (`OAG_AIR_REPLAY`).

    Columns: pose (position, up, forward, left, velocity, angular velocity), the own
    and lookahead samples (16 floats each), the target, the smoothed speed, the
    heading error the frame starts from, the last forward, the ground fraction,
    then the five outputs (steer, thrust, left, right, pitch) and the heading error
    the frame ends on. At `controls` a frame's outputs and end error are read off
    the next frame; at `pitch` they are the frame's own, its start error and last
    forward the previous frame's, and the lookahead and target are zero.
    """
    def point(sample):
        return (sample["pos"] + sample["tangent"] + sample["down"] + sample["lateral"]
                + [sample["progress"], sample["bound_left"], sample["bound_right"],
                   sample["racing_line"]])

    lines = []
    for a, b in zip(frames, frames[1:]):
        now, out, before = (a, b, a) if at == "controls" else (b, b, a)
        vals = []
        for key in ("pos", "up", "fwd", "left", "vel", "angvel"):
            vals += now[key]
        vals += point(now["own"])
        vals += point(now["ahead"]) if now["ahead"] else [0.0] * 16
        vals += now["target"] or [0.0] * 3
        vals += [now["smoothed_speed"], before["state"][0]]
        vals += now["last_fwd"] if at == "controls" else before["last_fwd"]
        vals += [now["grounded"]]
        vals += out["prev_out"] + [out["state"][0]]
        lines.append(" ".join(repr(float(v)) for v in vals))
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--restart", action="store_true", help="pause -> RESTART RACE first")
    parser.add_argument("--arm-after", type=int, default=60)
    parser.add_argument("--frames", type=int, default=3000)
    parser.add_argument("--budget", type=float, default=520.0, help="wall-clock seconds")
    parser.add_argument("--at", choices=["controls", "pitch"], default="controls",
                        help="stop in Ai_ComputeControls (default) or at Ai_ComputePitch's "
                        "return (0x088533b4, a0 = driver), where driver+0x18 is this "
                        "frame's pitch and driver+0x190 already holds this frame's forward "
                        "when airborne")
    args = parser.parse_args()
    args.stint = None
    args.steer = None

    dbg = rail.Debugger(args.port)
    log = {"args": {k: str(v) for k, v in vars(args).items()}, "events": [], "frames": []}
    start = time.time()
    try:
        if args.restart:
            rail.pair.restart_to_countdown(dbg, hold=True)
        else:
            dbg.resume()
            dbg.hold(cross=True)
        p = rail.arm(dbg, args, log)
        c = p["craft"]
        stats = dbg.read_u32(c + 0x6C)
        log["events"].append({"stats_base": stats,
                              "weight_distribution": rail.f32s(dbg, stats + 0x90, 1)[0]})
        stop = AFTER_TARGET if args.at == "controls" else PITCH_RETURN
        dbg.add_breakpoint(stop, "%s == 0x%x" % ("s1" if args.at == "controls" else "a0",
                                                  p["driver"]))
        try:
            for n in range(args.frames):
                if time.time() - start > args.budget:
                    log["events"].append({"stopped": "budget", "samples": n})
                    break
                dbg.call("cpu.status")
                dbg.pending = []
                dbg.call("cpu.resume")
                try:
                    dbg.wait_for_break(stop, timeout=30.0)
                except TimeoutError:
                    status = dbg.call("cpu.status")
                    if not (status["stepping"] and status["pc"] == stop):
                        raise
                status = dbg.call("cpu.status")
                sp = rail.regs(dbg)["sp"]
                d = p["driver"]
                racer = dbg.read_u32(d)
                pose = rail.f32s(dbg, racer + 0x10, 24)
                entity = dbg.read_u32(c + 0x1C4)
                row = {"cycle_frame": int(status["ticks"] / rail.CYCLES_PER_FRAME),
                       "pos": pose[0:3], "up": pose[4:7], "fwd": pose[8:11],
                       "left": pose[12:15], "vel": pose[16:19], "angvel": pose[20:23],
                       "own": sample(dbg, racer + 0x260),
                       "ahead": sample(dbg, sp + 0x40) if stop == AFTER_TARGET else None,
                       "target": rail.f32s(dbg, sp + 0xB0, 3) if stop == AFTER_TARGET else None,
                       "smoothed_speed": rail.f32s(dbg, d + 0x170, 1)[0],
                       "state": rail.f32s(dbg, d + 0x174, 3),
                       "last_fwd": rail.f32s(dbg, d + 0x190, 3),
                       "blend": rail.f32s(dbg, d + 0x48, 1)[0],
                       "prev_out": rail.f32s(dbg, d + 0x08, 5),
                       "grounded": rail.f32s(dbg, c + 0x2B0, 1)[0],
                       "flags1c0": dbg.read_u32(c + 0x1C0),
                       "hull_up": rail.f32s(dbg, c + 0x160, 3),
                       "spline_down": rail.f32s(dbg, entity + 0xB10, 3),
                       "craft_controls": rail.f32s(dbg, c + 0x2B8, 5)}
                log["frames"].append(row)
                if n % 600 == 0:
                    print("sample %d pos %s" % (n, [round(v, 1) for v in row["pos"]]),
                          file=sys.stderr)
        finally:
            dbg.brk()
            dbg.remove_breakpoint(stop)
            dbg.resume()
    finally:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(log))
        replay = args.out.with_suffix(".replay.txt")
        replay.write_text(flatten(log["frames"], args.at))
        print("wrote %s (%d frames) and %s" % (args.out, len(log["frames"]), replay),
              file=sys.stderr)


if __name__ == "__main__":
    main()
