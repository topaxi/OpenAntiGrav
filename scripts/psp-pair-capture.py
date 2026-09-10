#!/usr/bin/env python3
"""Catch a live craft-to-craft contact in Pulse and read what it does to spin.

`Body_ResolveContactPair` (`0x0884ef30`) applies its impulse to each body through
`Body_ApplyImpulseAtPoint` (`0x0884d64c`). The static reading says the "point"
it passes is the body's *own position* (`a1 == a0 + 0x30`), which makes the lever
arm zero and the angular half of the impulse vanish. This script is the runtime
leg of that claim: it stops at the `jal` for the first body, reads the registers
and both bodies, steps to the return, reads again, and then follows both bodies'
angular velocity for a run of ticks through `Ship_UpdateCraft`.

Run it against a race that is already live (`psp-drive.py menu --single-race`):

    uv run --with websocket-client scripts/psp-pair-capture.py --contacts 5 \\
        --log target/pair-capture.jsonl

Every hit is written as one JSON line; the summary printed at the end is what a
doc page quotes.
"""

import argparse
import json
import math
import struct
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger

# Body_ResolveContactPair's tail. s0 = bodyA, s1 = bodyB, s2 = contact,
# s4 = +j*n (for A), s5 = -j*n (for B).
CALL_B = 0x0884F630  # jal Body_ApplyImpulseAtPoint(bodyB, a1, s5)
RET_B = 0x0884F638
CALL_A = 0x0884F67C  # jal Body_ApplyImpulseAtPoint(bodyA, s3, s4)
RET_A = 0x0884F684
SHIP_UPDATE_CRAFT = 0x08849618
BODY_POINTER = 0x1CC

BODY_ROW0 = 0x000
BODY_UP = 0x010
BODY_FORWARD = 0x020
BODY_POSITION = 0x030
BODY_TRANSPOSE = 0x0C0
BODY_VELOCITY = 0x140
BODY_OMEGA = 0x150
BODY_ANGULAR_MOMENTUM = 0x160
BODY_INV_MASS = 0x378
BODY_ANGULAR_SCALE = 0x394

CONTACT_POINT = 0x00
CONTACT_NORMAL = 0x10
CONTACT_DEPTH = 0x30


def gpr(dbg):
    registers = dbg.call("cpu.getAllRegs")
    block = next(c for c in registers["categories"] if c["name"] == "GPR")
    return dict(zip(block["registerNames"], block["uintValues"]))


def vec3(dbg, at):
    return list(dbg.read_f32s(at, 3))


def body(dbg, at):
    return {
        "address": at,
        "position": vec3(dbg, at + BODY_POSITION),
        "velocity": vec3(dbg, at + BODY_VELOCITY),
        "omega": vec3(dbg, at + BODY_OMEGA),
        "angular_momentum": vec3(dbg, at + BODY_ANGULAR_MOMENTUM),
        "inv_mass": dbg.read_f32(at + BODY_INV_MASS),
        "angular_scale": dbg.read_f32(at + BODY_ANGULAR_SCALE),
        # The four 4x4 blocks the resolver and the applier read: the basis
        # rows, the body-space inverse inertia, its world copy, and the
        # transpose. Recorded whole so `j` can be recomputed offline exactly.
        "rows": list(dbg.read_f32s(at + 0x00, 12)),
        "inverse_inertia": list(dbg.read_f32s(at + 0x40, 16)),
        "world_inverse_inertia": list(dbg.read_f32s(at + 0x80, 16)),
        "transpose": list(dbg.read_f32s(at + 0xC0, 16)),
    }


def contact(dbg, at):
    return {
        "address": at,
        "point": vec3(dbg, at + CONTACT_POINT),
        "normal": vec3(dbg, at + CONTACT_NORMAL),
        "depth": dbg.read_f32(at + CONTACT_DEPTH),
    }


def sub(a, b):
    return [x - y for x, y in zip(a, b)]


def cross(a, b):
    return [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def length(a):
    return math.sqrt(dot(a, a))


def point_velocity(b, point):
    return [
        v + w for v, w in zip(b["velocity"], cross(b["omega"], sub(point, b["position"])))
    ]


def move_breakpoint(dbg, old, new):
    dbg.brk()
    if old is not None:
        dbg.remove_breakpoint(old)
    dbg.add_breakpoint(new)


def normalized(v):
    n = length(v)
    return [x / n for x in v]


def stage_head_on(dbg, player, speed, gap, log, facing="same"):
    """Drop the player's craft nose-to-nose in front of a rival, closing fast.

    The same body write `psp-drive.py place` uses, aimed at a live rival
    rather than a pad: the player goes `gap` units ahead of the first rival
    `Ship_UpdateCraft` visits, facing back down its forward, at `speed` along
    that new forward. The CPU stays stopped from the write to the breakpoint
    switch, so the contact that follows is the first thing that runs.
    """
    move_breakpoint(dbg, None, SHIP_UPDATE_CRAFT)
    rival = None
    for _ in range(64):
        dbg.call("cpu.resume")
        dbg.wait_for_break(SHIP_UPDATE_CRAFT, timeout=30.0)
        craft = gpr(dbg)["a0"]
        at = dbg.read_u32(craft + BODY_POINTER)
        if craft != player and rival is None:
            rival = {
                "craft": craft,
                "body": at,
                "position": vec3(dbg, at + BODY_POSITION),
                "up": vec3(dbg, at + BODY_UP),
                "forward": vec3(dbg, at + BODY_FORWARD),
                "velocity": vec3(dbg, at + BODY_VELOCITY),
            }
            continue
        if craft == player and rival is not None:
            # Facing the rival head-on puts the craft 180 degrees against the
            # track's own alignment, and the placed craft tumbled at 84 rad/s
            # before the contact even resolved (2026-09-10). Same facing, a
            # slow craft in the rival's path, is a fast rear-end hit instead.
            sign = -1.0 if facing == "opposite" else 1.0
            row2 = normalized([sign * x for x in rival["forward"]])
            up = normalized(rival["up"])
            lean = dot(up, row2)
            row1 = normalized([u - lean * f for u, f in zip(up, row2)])
            row0 = cross(row1, row2)
            position = [p + gap * f for p, f in zip(rival["position"], rival["forward"])]
            velocity = [f * speed for f in row2]
            dbg.write_f32s(at + BODY_ROW0, row0)
            dbg.write_f32s(at + BODY_UP, row1)
            dbg.write_f32s(at + BODY_FORWARD, row2)
            dbg.write_f32s(at + BODY_POSITION, position)
            for slot in range(3):
                dbg.write_f32s(
                    at + BODY_TRANSPOSE + 0x10 * slot, (row0[slot], row1[slot], row2[slot])
                )
            dbg.write_f32s(at + BODY_VELOCITY, velocity)
            dbg.write_f32s(at + BODY_OMEGA, (0.0, 0.0, 0.0))
            dbg.write_f32s(at + BODY_ANGULAR_MOMENTUM, (0.0, 0.0, 0.0))
            staged = {
                "staged": True,
                "player_body": at,
                "rival": rival,
                "position": position,
                "velocity": velocity,
                "closing_speed": speed + dot(rival["velocity"], rival["forward"]),
            }
            log.write(json.dumps(staged) + "\n")
            print(
                "staged: player body 0x%08x placed %.1f ahead of rival body 0x%08x "
                "(rival doing %.1f along its forward), closing at %.1f"
                % (at, gap, rival["body"], dot(rival["velocity"], rival["forward"]),
                   staged["closing_speed"]),
                flush=True,
            )
            return SHIP_UPDATE_CRAFT, {at, rival["body"]}
    raise RuntimeError("never saw both a rival and the player at Ship_UpdateCraft")


def one_contact(dbg, index, log, follow_ticks, stage=None):
    previous = None
    wanted = None
    if stage is not None:
        previous, wanted = stage_head_on(
            dbg, stage["player"], stage["speed"], stage["gap"], log, stage["facing"]
        )
    move_breakpoint(dbg, previous, CALL_B)
    # A staged pair is not necessarily the first pair the resolver visits that
    # frame, so keep going until it is the one on the stack.
    for skipped in range(200):
        dbg.call("cpu.resume")
        dbg.wait_for_break(CALL_B, timeout=600.0)
        regs = gpr(dbg)
        if wanted is None or {regs["s0"], regs["s1"]} == wanted:
            break
    else:
        raise RuntimeError("the staged pair never reached the resolver in 200 contacts")
    if skipped:
        print("  (skipped %d other pairs' contacts first)" % skipped, flush=True)
    body_a, body_b, contact_at = regs["s0"], regs["s1"], regs["s2"]
    record = {
        "contact_index": index,
        "regs": {k: regs[k] for k in ("a0", "a1", "a2", "s0", "s1", "s2", "s3", "s4", "s5")},
        "contact": contact(dbg, contact_at),
        "impulse_a": vec3(dbg, regs["s4"]),
        "impulse_b": vec3(dbg, regs["s5"]),
        "a_before": body(dbg, body_a),
        "b_before": body(dbg, body_b),
    }
    record["a1_is_bodyB_position"] = regs["a1"] == body_b + BODY_POSITION
    record["s3_is_bodyA_position"] = regs["s3"] == body_a + BODY_POSITION

    move_breakpoint(dbg, CALL_B, RET_B)
    dbg.call("cpu.resume")
    dbg.wait_for_break(RET_B, timeout=30.0)
    record["b_after"] = body(dbg, body_b)

    move_breakpoint(dbg, RET_B, RET_A)
    dbg.call("cpu.resume")
    dbg.wait_for_break(RET_A, timeout=30.0)
    record["a_after"] = body(dbg, body_a)

    # The derived numbers a reader wants in front of them.
    point = record["contact"]["point"]
    normal = record["contact"]["normal"]
    vpa = point_velocity(record["a_before"], point)
    vpb = point_velocity(record["b_before"], point)
    record["vn_before"] = dot(sub(vpa, vpb), normal)
    record["j"] = dot(record["impulse_a"], normal)
    record["lever_a"] = length(sub(point, record["a_before"]["position"]))
    record["lever_b"] = length(sub(point, record["b_before"]["position"]))
    record["omega_delta_a"] = sub(record["a_after"]["omega"], record["a_before"]["omega"])
    record["omega_delta_b"] = sub(record["b_after"]["omega"], record["b_before"]["omega"])
    record["velocity_delta_a"] = sub(
        record["a_after"]["velocity"], record["a_before"]["velocity"]
    )
    record["velocity_delta_b"] = sub(
        record["b_after"]["velocity"], record["b_before"]["velocity"]
    )
    log.write(json.dumps(record) + "\n")
    log.flush()
    print(
        "contact %d: vn %.2f j %.3f lever a %.2f b %.2f | a1==B.pos %s s3==A.pos %s | "
        "d(omega) A %s B %s | d(v) A %.2f B %.2f"
        % (
            index,
            record["vn_before"],
            record["j"],
            record["lever_a"],
            record["lever_b"],
            record["a1_is_bodyB_position"],
            record["s3_is_bodyA_position"],
            ["%.5f" % x for x in record["omega_delta_a"]],
            ["%.5f" % x for x in record["omega_delta_b"]],
            length(record["velocity_delta_a"]),
            length(record["velocity_delta_b"]),
        ),
        flush=True,
    )

    # Follow both bodies through the next ticks: a reaction outside the
    # resolver (a ring-record consumer, say) would show up here, not above.
    move_breakpoint(dbg, RET_A, SHIP_UPDATE_CRAFT)
    seen = {body_a: 0, body_b: 0}
    ticks = []
    hits = 0
    limit = follow_ticks * 8 + 8
    while hits < limit and min(seen.values()) < follow_ticks:
        dbg.call("cpu.resume")
        dbg.wait_for_break(SHIP_UPDATE_CRAFT, timeout=30.0)
        hits += 1
        craft = gpr(dbg)["a0"]
        at = dbg.read_u32(craft + BODY_POINTER)
        if at in seen:
            seen[at] += 1
            sample = {
                "hit": hits,
                "which": "a" if at == body_a else "b",
                "tick": seen[at],
                "omega": vec3(dbg, at + BODY_OMEGA),
                "velocity": vec3(dbg, at + BODY_VELOCITY),
                "position": vec3(dbg, at + BODY_POSITION),
            }
            ticks.append(sample)
    dbg.brk()
    dbg.remove_breakpoint(SHIP_UPDATE_CRAFT)
    log.write(json.dumps({"contact_index": index, "follow": ticks}) + "\n")
    log.flush()
    for which in ("a", "b"):
        peak = max(
            (length(s["omega"]) for s in ticks if s["which"] == which), default=float("nan")
        )
        print(
            "  %s: peak |omega| over the next %d ticks %.4f rad/s (%.1f deg/s)"
            % (which, follow_ticks, peak, math.degrees(peak)),
            flush=True,
        )
    return record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=47810)
    parser.add_argument("--contacts", type=int, default=3)
    parser.add_argument("--follow", type=int, default=20, metavar="TICKS")
    parser.add_argument("--log", type=Path, default=Path("target/pair-capture.jsonl"))
    parser.add_argument(
        "--stage-speed",
        type=float,
        default=0.0,
        metavar="UNITS_PER_S",
        help="before each contact, teleport --craft nose-to-nose in front of a "
        "rival at this speed, so the hit is fast rather than a grid-start touch",
    )
    parser.add_argument("--stage-gap", type=float, default=20.0, metavar="UNITS")
    parser.add_argument(
        "--stage-facing",
        choices=("same", "opposite"),
        default="same",
        help="same: a slow craft placed in the rival's path (a fast rear hit); "
        "opposite: nose to nose, which the track alignment fights",
    )
    parser.add_argument("--craft", type=lambda v: int(v, 0), metavar="ADDRESS")
    args = parser.parse_args()

    stage = None
    if args.stage_speed > 0.0:
        if args.craft is None:
            raise SystemExit("--stage-speed needs --craft (the player's craft address)")
        stage = {
            "player": args.craft,
            "speed": args.stage_speed,
            "gap": args.stage_gap,
            "facing": args.stage_facing,
        }

    with Debugger(args.port) as dbg, args.log.open("a") as log:
        start = time.time()
        for index in range(args.contacts):
            one_contact(dbg, index, log, args.follow, stage)
        dbg.resume()
        print("%d contacts in %.0fs, log at %s" % (args.contacts, time.time() - start, args.log))


if __name__ == "__main__":
    main()
