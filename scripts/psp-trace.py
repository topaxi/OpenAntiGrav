#!/usr/bin/env python3
"""Capture a per-tick trace of ship state out of Wipeout Pulse running in PPSSPP.

This is the emulator side of M3's verification harness: the recording that
`oag-trace` compares a Rust run against. See
`docs/reverse-engineering/ppsspp-debugger.md` for how to get a PPSSPP into a
state where this can run, and `docs/reverse-engineering/verification-protocol.md`
for what the comparison is for.

The capture is breakpoint-driven, and has to be: a memory read costs about 520 ms
while the CPU is running and about 0.3 ms while it is stepping. So this breaks in
`Ship_UpdateCraft` once per tick, reads the whole craft structure in a single
`memory.read`, and resumes.

    uv run --with websocket-client scripts/psp-trace.py --ticks 600 --hold cross

There are two ways to drive the capture. `--hold` holds one input for the whole
run, which is enough for a straight line and nothing else. `--script` reads a
committed input script - `verification/scenarios/*.inputs`, see
`scripts/input_script.py` - and sends a fresh controller state on every tick, so
the capture and `oag-trace run --script <the same file>` are driven by the same
authored intent rather than by each other:

    uv run --with websocket-client scripts/psp-trace.py \\
        --script verification/scenarios/steer-both-ways.inputs --warmup 25

The output is CSV on stdout, one row per tick. It records **derived game data**
and must never be committed; write it under `data/traces/`. The *script* is not
derived data - it is a list of button names somebody chose - and is committed.
"""

import argparse
import struct
import time
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import input_script
from ppsspp_debugger import Debugger

# Ship_UpdateCraft, docs/ghidra/functions/psp-pulse/engine.md. **The craft is a0
# and the body is a1**, not a1/a2 as that page's signature says: at a breakpoint
# on entry, `craft+0x1cc` holds exactly the value in a1, dt at `craft+0x1c8`
# reads as a plausible frame time and `grounded` at `craft+0x2b0` as 1.0, while
# the same offsets off a1 are nonsense. a2 holds the function's own address,
# which is what the vtable dispatch at the call site leaves there.
SHIP_UPDATE_CRAFT = 0x08849618
CRAFT_REGISTER = "a0"
CRAFT_BYTES = 0x400
BODY_POINTER = 0x1CC

# Offsets into the craft, from engine.md.
#
# `stun_timer` and `timer_2e0` are the two gates on `Ship_UpdateEngine`'s early
# return (`0x0884c634`, confidence 88): with flag `0x200` clear, either one above
# zero means **no thrust and a zeroed throttle state** for that frame.
# `craft+0x290` is the collision stun timer - `Ship_ApplyCollisionImpulse`
# (`0x0883f274`) arms it with `+= 0.5` on a hit and `Ship_ApplyLateralGrip`
# (`0x08848b78`) returns early while it runs, so a stunned craft also slides
# (confidence 85). `craft+0x2e0`'s arming condition has never been read, so it
# keeps its offset for a name rather than a guess dressed as one - engine.md's
# own suggestion is that it fits a capture taken near a race start.
#
# engine.md asks for exactly these two columns by name: without them a capture
# cannot say whether the gate fired at all, and the force-balance argument that
# the missing "12x of resistance" is really thrust the original never applied
# stays an inference. They are also the cheapest wall-contact indicator the
# already-documented fields offer.
CRAFT_FIELDS = [
    ("dt", 0x1C8),
    ("grounded", 0x2B0),
    ("throttle", 0x2B8), ("brake", 0x2BC), ("steer", 0x2C0),
    ("airbrake_l", 0x2C4), ("airbrake_r", 0x2C8),
    ("speed_cached", 0x2EC),
    ("stun_timer", 0x290), ("timer_2e0", 0x2E0),
]

# Offsets into the rigid body, measured at runtime by diffing successive frames.
# Rows 0/1/2 are orthonormal, and velocity times the frame time reproduces the
# position delta to within 0.007 units per tick over a 200-tick capture.
#
# **`speed` at +0x398 is not the velocity's own length**, which an earlier pass
# recorded here and a real capture disproved: over 200 ticks of Talon's Junction
# it runs a steady 3.67 % high (ratio 1.0367, sd 0.0026), about 0.86 units/s.
# Nor is it the forward projection (ratio 1.0402, sd 0.0028) - the two candidates
# are within each other's spread here, so this capture cannot tell them apart.
# What +0x398 holds is not established, and the capture's own speed range is only
# 3 %, so a constant offset and a constant factor cannot be separated either.
#
# `speed_cached` on the craft **is** the previous tick's `dot(velocity, forward)`
# - the forward-projected speed, not the velocity's magnitude. That is what
# engine.md says at confidence 95, and a shorter pass wrote the magnitude reading
# here in contradiction of it; this supersedes that. The same 200 ticks put the
# residual at a mean 3.3e-6 and a max 9.7e-6 - the `%.7g` below at a speed of 24,
# so indistinguishable from exact - against a mean 0.077 for the stale magnitude,
# and 1.3e-3 for `dot(velocity(t-1), forward(t))`, which says both vectors are the
# previous tick's rather than only one. The distinction matters as soon as the
# ship is not travelling straight ahead: sliding through a corner, the two
# readings differ by the cosine of the slip angle, and no capture has been taken
# there yet.
#
# Both columns are recorded; neither is assumed.
#
# `avel_*` at +0x160 is the **angular velocity**, on two independent legs, one per
# binary. PSP: `Body_ClearVelocity` (`0x0884da5c`) zeroes `body+0x140` and
# `body+0x160` and nothing else, and `+0x140` is the linear velocity already
# verified here - velocity times the frame time reproduces the position delta to
# 0.007 units per tick. PS2: `Ship_ApplyAngularDamping` (`0x0015c1b0`) reads
# `body+0x160` as the angular velocity it damps, per
# `docs/ghidra/functions/ps2-pulse/craft-update.md`.
#
# **Its sign and its frame are both open, and the raw value is what is recorded.**
# The PS2 page names it `angularVelocityLocal` but lists "which frame `body+0x150`
# and `body+0x160` are each expressed in" as unresolved, and the same page's
# `w_game = -w_physics` result says the engine's angular velocity is the negative
# of the physical one. Nothing is negated or rotated on the way out of memory;
# `oag_trace::trace::AngularReading` is where the four readings are enumerated,
# and a capture's own basis rows settle which one it is - see
# `Summary::angular_readings`, which needs no simulation to run.
BODY_FIELDS = [
    ("right_x", 0x000), ("right_y", 0x004), ("right_z", 0x008),
    ("up_x", 0x010), ("up_y", 0x014), ("up_z", 0x018),
    ("fwd_x", 0x020), ("fwd_y", 0x024), ("fwd_z", 0x028),
    ("pos_x", 0x030), ("pos_y", 0x034), ("pos_z", 0x038),
    ("vel_x", 0x140), ("vel_y", 0x144), ("vel_z", 0x148),
    ("speed", 0x398),
    ("avel_x", 0x160), ("avel_y", 0x164), ("avel_z", 0x168),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=47810)
    parser.add_argument(
        "--ticks",
        type=int,
        metavar="N",
        help="how many ticks to record. Defaults to the script's own length with "
        "--script, and to 300 without one.",
    )
    parser.add_argument(
        "--hold",
        action="append",
        default=[],
        metavar="BUTTON",
        help="hold a button for the whole capture, e.g. --hold cross (thrust)",
    )
    parser.add_argument(
        "--script",
        type=Path,
        metavar="FILE",
        help="drive the capture from a committed input script, e.g. "
        "verification/scenarios/steer-both-ways.inputs. The same file drives "
        "`oag-trace run --script`, which is the point: it is the only input mode "
        "that is authored rather than inferred, so a varying input is known on "
        "both sides. See scripts/input_script.py for the format.",
    )
    parser.add_argument(
        "--script-lead",
        type=int,
        default=0,
        metavar="N",
        help="send the script's tick k+N at the breakpoint for tick k. "
        "**Unverified, and here to be measured rather than assumed**: the "
        "breakpoint is inside the frame, so whether a state set there is seen by "
        "that frame or the next depends on where the game polls input, which has "
        "not been read out of the binary. A capture of steer-both-ways settles it "
        "in one line - the recorded `steer` column is flat until the tick the "
        "game first saw `left`, so the gap between that and the script's own tick "
        "60 is the lead to use.",
    )
    parser.add_argument(
        "--warmup-hold",
        action="append",
        default=[],
        metavar="BUTTON",
        help="hold a button during --warmup only, then hand over to --script. A "
        "scripted capture warms up holding *nothing* by default, which is what "
        "sitting through a countdown needs (thrust through it is a false start). "
        "This is for the other case: reaching the script's starting speed first, "
        "so that a scenario begins the way the reference capture did rather than "
        "from a standstill.",
    )
    parser.add_argument("--out", type=Path, help="write here instead of stdout")
    parser.add_argument(
        "--ship",
        type=int,
        default=0,
        metavar="N",
        help="which craft to follow, by order of first appearance within a tick. "
        "A time trial has one; a race has eight and updates them all from this "
        "same function, so the others' hits are resumed past rather than recorded.",
    )
    parser.add_argument(
        "--craft",
        type=lambda v: int(v, 0),
        metavar="ADDRESS",
        help="follow this craft address instead of selecting one by --ship. The "
        "addresses seen are printed to stderr, so a first run identifies them.",
    )
    parser.add_argument(
        "--max-ships",
        type=int,
        default=8,
        metavar="N",
        help="how many craft a tick may update, which bounds how many breakpoint "
        "hits one tick of the followed craft is allowed to cost.",
    )
    parser.add_argument(
        "--warmup",
        type=float,
        default=0.0,
        metavar="SECONDS",
        help="run free with the buttons held for this long before capturing. A "
        "breakpoint round trip costs far more than a frame, so capture runs the "
        "emulator at a small fraction of real time - too slow to sit through a "
        "start-line countdown. Warm up first, then capture the part that matters.",
    )
    args = parser.parse_args()

    if args.script and args.hold:
        parser.error(
            "--hold holds one input for the whole capture and --script sends a "
            "fresh one every tick, so they cannot both drive it. --warmup-hold is "
            "what holds something during the warmup of a scripted capture."
        )
    if args.warmup_hold and not args.script:
        parser.error("--warmup-hold is for scripted captures; without --script, --hold covers it")

    states = []
    if args.script:
        try:
            states = input_script.load(args.script)
        except input_script.ScriptError as error:
            parser.error(str(error))
        if not states:
            parser.error("%s: the script has no ticks in it" % args.script)
        for reason in input_script.unrepresentable(states):
            parser.error("%s cannot be sent to a PSP - %s" % (args.script, reason))
    ticks = args.ticks if args.ticks is not None else (len(states) or 300)
    if states and ticks > len(states):
        print(
            "note: %s covers %d tick(s) and --ticks asks for %d, so its last state "
            "is held for the remaining %d"
            % (args.script, len(states), ticks, ticks - len(states)),
            file=sys.stderr,
        )

    dbg = Debugger(args.port)
    out = args.out.open("w") if args.out else sys.stdout
    # What the pad was last told, so a tick that changes nothing costs no
    # traffic. `None` means "nothing has been sent yet", which is not the same as
    # "nothing is held" - the first tick must always send.
    sent = {"state": None, "analog": None}

    def send(state):
        """Push one tick of scripted state at the emulator, if it changed."""
        if sent["state"] is not None and state == sent["state"]:
            return
        sent["state"] = state
        dbg.hold(**input_script.button_payload(state))
        analog = input_script.analog_payload(state)
        if analog is None and sent["analog"] not in (None, (0.0, 0.0)):
            # A script that stops asking for analog must recentre the stick, or
            # the last explicit deflection would silently outlive its own line.
            analog = (0.0, 0.0)
        if analog is not None:
            dbg.analog(*analog)
            sent["analog"] = analog

    try:
        if args.hold:
            dbg.resume()
            dbg.hold(**{button: True for button in args.hold})
        if args.warmup_hold:
            dbg.resume()
            dbg.hold(**{button: True for button in args.warmup_hold})
        if args.warmup:
            dbg.resume()
            time.sleep(args.warmup)
        if args.warmup_hold:
            # The script owns the input from here; anything the warmup held that
            # the script's first tick does not is released by the first send().
            dbg.hold(**{button: False for button in args.warmup_hold})

        names = [name for name, _ in CRAFT_FIELDS] + [name for name, _ in BODY_FIELDS]
        print("tick," + ",".join(names), file=out)

        # A race updates every craft from the same function, so the breakpoint
        # fires once per ship per tick and only one of those hits is the ship
        # being traced. Hits for the others are resumed past rather than
        # recorded: interleaving eight ships into one file would produce a trace
        # of nothing in particular, and stopping at the second address - which
        # this did until a real eight-ship capture met it - makes a trace of a
        # race impossible to take at all.
        seen = []
        follow = args.craft
        tick = 0
        hits = 0
        budget = ticks * args.max_ships + args.max_ships
        # `each_hit` owns the break/arm/resume cycle, including the two traps
        # around it, so the filter is a `continue` inside its loop rather than a
        # second copy of that sequence.
        for _, _ in dbg.each_hit(SHIP_UPDATE_CRAFT, budget, timeout=60):
            if tick >= ticks:
                break
            hits += 1

            registers = dbg.call("cpu.getAllRegs")
            gpr = next(c for c in registers["categories"] if c["name"] == "GPR")
            craft = dict(zip(gpr["registerNames"], gpr["uintValues"]))[CRAFT_REGISTER]
            if craft not in seen:
                seen.append(craft)
                print("craft %d at 0x%08x" % (len(seen) - 1, craft), file=sys.stderr)
            if follow is None and len(seen) > args.ship:
                follow = seen[args.ship]
                print("following craft 0x%08x" % follow, file=sys.stderr)
            if craft != follow:
                continue

            craft_blob = dbg.read(craft, CRAFT_BYTES)
            body = struct.unpack("<I", craft_blob[BODY_POINTER : BODY_POINTER + 4])[0]
            body_blob = dbg.read(body, CRAFT_BYTES)
            values = [struct.unpack("<f", craft_blob[at : at + 4])[0] for _, at in CRAFT_FIELDS]
            values += [struct.unpack("<f", body_blob[at : at + 4])[0] for _, at in BODY_FIELDS]
            print("%d,%s" % (tick, ",".join("%.7g" % v for v in values)), file=out)

            # The row above is the craft as this frame's update *found* it, and
            # the input set here is what drives the frame that produces the next
            # row - which is exactly `oag-trace`'s alignment, where a row is
            # emitted before its own step. Whether the emulator's own input poll
            # has already run by the time the breakpoint is reached is not
            # established, so `--script-lead` exists to shift this by whole ticks
            # once a capture has measured it.
            if states:
                send(input_script.at(states, tick + args.script_lead))
            tick += 1
        print(
            "%d tick(s) from %d hit(s) across %d craft" % (tick, hits, len(seen)),
            file=sys.stderr,
        )
    finally:
        try:
            if args.hold:
                dbg.resume()
                dbg.hold(**{button: False for button in args.hold})
            elif states or args.warmup_hold:
                # Leave the pad the way it was found, or the next capture starts
                # with whatever the last line of this script happened to hold.
                dbg.resume()
                dbg.hold(**input_script.button_payload(input_script.State()))
                dbg.analog(0.0, 0.0)
        except Exception:
            pass
        if args.out:
            out.close()
        dbg.close()


if __name__ == "__main__":
    main()
