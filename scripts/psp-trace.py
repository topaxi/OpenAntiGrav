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
import math
import struct
import time
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import input_script
from niri_shot import find_window, screenshot
from ppsspp_debugger import Debugger
from psp_trace_fields import (
    BODY_FIELDS,
    CAMERA_FIELDS,
    CAMERA_NODE_OFFSET,
    CAMERA_UPDATE_BREAK,
    CRAFT_FIELDS,
    ENTITY_FIELDS,
    ENTITY_OWNER,
    ENTITY_POINTER,
    FLARE_FIELDS,
    FLARE_OWNER,
    FLARE_POINTER,
    RACER_POINTER,
    SHIP_UPDATE_CRAFT,
)

# The craft's own address and the two structure layouts live in
# `psp_trace_fields`, because `scripts/psp-autopilot.py` breaks on the same
# function and writes the same columns, and two copies of an offset table drift.
CRAFT_REGISTER = "a0"
CRAFT_BYTES = 0x400
BODY_POINTER = 0x1CC


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
        "scripted capture warms up holding *nothing* by default, which keeps the "
        "start pose reproducible; a live capture found no penalty for holding "
        "thrust through a countdown (throttleState just reads 0 for the gated "
        "ticks, no stall - see docs/gameplay/race-modes.md), so this default is "
        "about pose, not about avoiding a false start. --warmup-hold is for the "
        "other case: reaching the script's starting speed first, so that a "
        "scenario begins the way the reference capture did rather than from a "
        "standstill.",
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
    parser.add_argument(
        "--start-heading",
        type=float,
        metavar="DEGREES",
        help="discard ticks until the craft's heading - `atan2(fwd.x, fwd.z)` in "
        "degrees - passes this, then record tick 0. A craft parked on the start "
        "line is not at rest: it yaws at a steady 0.311 deg/s, the same to six "
        "digits on every restart, so its pose is a function of how many frames it "
        "has sat there and a wall-clock handover between `psp-drive.py restart` "
        "and this script is what makes a start pose irreproducible. Waiting for a "
        "heading is self-anchoring - it needs no shared frame counter and no "
        "memory write - and it pins the pose to one tick, 0.005 degrees. See "
        "docs/reverse-engineering/ppsspp-debugger.md.",
    )
    parser.add_argument(
        "--start-heading-timeout",
        type=int,
        default=6000,
        metavar="TICKS",
        help="give up waiting for --start-heading after this many ticks. The "
        "guard that matters is the other one: the heading only ever increases on "
        "this start line, so a target already passed would otherwise wait for a "
        "whole lap.",
    )
    parser.add_argument(
        "--camera",
        action="store_true",
        help="also record the player camera's pose per tick, as the cam_* "
        "columns. The camera node's address is learned from one hit of a "
        "breakpoint inside Camera_UpdatePlayerView, then read at the ship "
        "breakpoint every tick - two live breakpoints cannot coexist, only the "
        "most recently added one fires. The node's stored position is the "
        "negated eye and is negated back here, so cam_pos_* is a world "
        "position like pos_*. See docs/ghidra/functions/psp-pulse-usa/camera.md.",
    )
    parser.add_argument(
        "--flare",
        action="store_true",
        help="also record the craft's Engine Flare node per tick, as the "
        "boost_timer / plume_timer / intensity / half_size columns. This is "
        "what makes a boost comparison quantitative rather than eyeballed: "
        "boost_timer pins the entry tick exactly (it steps to the 0.8 literal "
        "ExhaustFlare_OnSpeedupPad stores) so a frame of ours can be posed at "
        "the same boost age with --pose-boost, instead of the age being "
        "inferred from where the craft looks like it crossed. The node is "
        "reached by two pointer hops off the traced craft and the walk is "
        "checked against the flare's own owner field; see "
        "scripts/psp_trace_fields.py.",
    )
    parser.add_argument(
        "--shot-every",
        type=int,
        metavar="N",
        help="screenshot the emulator's window at every Nth recorded tick, "
        "named tick%%05d.png under --shot-dir. The CPU is stopped at the "
        "breakpoint when the shot is taken, so shots are tick-addressable - but "
        "they show the last *presented* frame, which lags the paused tick by a "
        "constant measured in docs/tools/frame-compare.md, not by zero.",
    )
    parser.add_argument(
        "--shot-dir",
        type=Path,
        help="where --shot-every writes its screenshots (derived game data: "
        "keep it under data/shots/, which is gitignored)",
    )
    parser.add_argument(
        "--edram-every",
        type=int,
        metavar="N",
        help="at every Nth recorded tick, write the two framebuffers and the "
        "bloom's scratch buffers A and B (EDRAM 0x110000 and 0x132000, stride "
        "256) as raw bytes tick%%05d.edram under --edram-dir. The alpha "
        "channel of a framebuffer is the glow mask.",
    )
    parser.add_argument("--edram-dir", type=Path)
    parser.add_argument(
        "--shot-window",
        type=int,
        help="niri window id of the emulator. Defaults to the first PPSSPPSDL "
        "window (`niri msg windows` lists them), since ids change every launch.",
    )
    args = parser.parse_args()

    if args.shot_every is not None and not args.shot_dir:
        parser.error("--shot-every needs --shot-dir")
    if args.shot_every is not None and args.shot_window is None:
        args.shot_window = find_window()
        if args.shot_window is None:
            parser.error(
                "no PPSSPPSDL window found to screenshot; pass --shot-window ID"
            )
        print("screenshotting niri window %d" % args.shot_window, file=sys.stderr)

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

    # The camera node is a stable heap object for the life of the race, so its
    # address is learned once - from the one hit the camera breakpoint gets
    # before it is removed again - and its 0x40 bytes are then read at the ship
    # breakpoint each tick. Not a second live breakpoint: only the most
    # recently added one ever fires (see Debugger.each_hit_any), a trap this
    # capture found by recording 189 camera hits and no ship at all.
    camera_node = None
    if args.camera:
        for _, _ in dbg.each_hit(CAMERA_UPDATE_BREAK, 1, timeout=60):
            registers = dbg.call("cpu.getAllRegs")
            gpr = next(c for c in registers["categories"] if c["name"] == "GPR")
            s7 = dict(zip(gpr["registerNames"], gpr["uintValues"]))["s7"]
            camera_node = dbg.read_u32(s7 + CAMERA_NODE_OFFSET)
        print("camera node at 0x%08x" % camera_node, file=sys.stderr)
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

        names = (
            [name for name, _ in CRAFT_FIELDS]
            + [name for name, _ in ENTITY_FIELDS]
            + [name for name, _ in BODY_FIELDS]
        )
        if args.camera:
            names += [name for name, _ in CAMERA_FIELDS]
        if args.flare:
            names += [name for name, _ in FLARE_FIELDS]
        print("tick," + ",".join(names), file=out)

        # Resolved once, on the first recorded tick, and reused: both hops are
        # allocated with the craft and do not move for the life of a race, so
        # re-walking them every tick would cost two extra reads a tick for a
        # constant. `None` until then.
        flare_node = None
        # Same reasoning for the ship entity, which every capture reads.
        entity = None

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
        waited = 0
        # The ticks spent waiting for --start-heading are breakpoint hits too, so
        # the budget has to cover them or the loop runs out before tick 0.
        allowance = args.start_heading_timeout if args.start_heading is not None else 0
        budget = (ticks + allowance) * args.max_ships + args.max_ships
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

            if args.start_heading is not None:
                forward = struct.unpack_from("<3f", body_blob, 0x20)
                degrees = math.degrees(math.atan2(forward[0], forward[2]))
                if waited == 0:
                    print("waiting for heading %.4f, at %.4f" % (args.start_heading, degrees),
                          file=sys.stderr)
                    if degrees > args.start_heading:
                        print("the heading is already past the target - a start pose "
                              "cannot be pinned to it on this run", file=sys.stderr)
                        return 2
                waited += 1
                if degrees < args.start_heading:
                    if waited > args.start_heading_timeout:
                        print("gave up after %d tick(s) at heading %.4f" % (waited, degrees),
                              file=sys.stderr)
                        return 2
                    continue
                print("started at heading %.4f after %d tick(s)" % (degrees, waited),
                      file=sys.stderr)
                args.start_heading = None

            values = [struct.unpack("<f", craft_blob[at : at + 4])[0] for _, at in CRAFT_FIELDS]
            if entity is None:
                entity = struct.unpack(
                    "<I", craft_blob[ENTITY_POINTER : ENTITY_POINTER + 4]
                )[0]
                # The reciprocal identity, checked rather than assumed - the
                # entity's +0x94 points back at the craft we came from. Refusing
                # here instead of recording a column of garbage: the offset this
                # replaces read an orientation-matrix element and looked entirely
                # plausible. See psp_trace_fields.py.
                owner = dbg.read_u32(entity + ENTITY_OWNER)
                if owner != craft:
                    print(
                        "entity 0x%08x claims craft 0x%08x, not the 0x%08x it "
                        "was reached from - the chain in psp_trace_fields.py no "
                        "longer holds" % (entity, owner, craft),
                        file=sys.stderr,
                    )
                    return 2
                print(
                    "entity 0x%08x (craft 0x%08x -> +0x1c4)" % (entity, craft),
                    file=sys.stderr,
                )
            # A single contiguous read spanning the lowest to the highest offset,
            # not `4 * len(ENTITY_FIELDS)` from the first: that sizing assumed the
            # fields were packed back to back, which held while `shield` was the
            # only one and stopped holding the moment the sideshift timers - 0x824
            # bytes further into the entity - joined it. Costs one bigger read
            # instead of one small one; still far under the 4 KB where a `memory.read`
            # gets expensive (docs/reverse-engineering/ppsspp-debugger.md).
            entity_lo = min(at for _, at in ENTITY_FIELDS)
            entity_hi = max(at for _, at in ENTITY_FIELDS) + 4
            entity_blob = dbg.read(entity + entity_lo, entity_hi - entity_lo)
            values += [
                struct.unpack("<f", entity_blob[at - entity_lo :][:4])[0]
                for _, at in ENTITY_FIELDS
            ]
            values += [struct.unpack("<f", body_blob[at : at + 4])[0] for _, at in BODY_FIELDS]
            if camera_node is not None:
                # Read at the ship stop: the node holds whatever the last
                # camera update wrote, so its phase against this row is a
                # constant fraction of a frame, absorbed by the calibration in
                # docs/tools/frame-compare.md. The node's +0x30 holds the
                # negated eye; the file stores the eye (camera.md has the
                # evidence for the sign).
                blob = dbg.read(camera_node, 0x40)
                values += [
                    -struct.unpack("<f", blob[at : at + 4])[0]
                    if name.startswith("cam_pos_")
                    else struct.unpack("<f", blob[at : at + 4])[0]
                    for name, at in CAMERA_FIELDS
                ]
            if args.flare:
                if flare_node is None:
                    racer = struct.unpack(
                        "<I", craft_blob[RACER_POINTER : RACER_POINTER + 4]
                    )[0]
                    flare_node = dbg.read_u32(racer + FLARE_POINTER)
                    owner = dbg.read_u32(flare_node + FLARE_OWNER)
                    # The identity that makes the walk evidence rather than an
                    # offset guess. Refusing here rather than recording a column
                    # of garbage: a wrong pointer would still read as plausible
                    # floats, and a boost comparison built on it would be worse
                    # than no comparison at all.
                    if owner != racer:
                        print(
                            "flare 0x%08x claims owner 0x%08x, not the 0x%08x "
                            "craft+0x1c4 reached - the chain in "
                            "psp_trace_fields.py no longer holds"
                            % (flare_node, owner, racer),
                            file=sys.stderr,
                        )
                        return 2
                    print(
                        "flare 0x%08x (craft 0x%08x -> 0x%08x -> +0x78)"
                        % (flare_node, craft, racer),
                        file=sys.stderr,
                    )
                blob = dbg.read(flare_node, 0x100)
                values += [
                    struct.unpack("<f", blob[at : at + 4])[0] for _, at in FLARE_FIELDS
                ]
            print("%d,%s" % (tick, ",".join("%.7g" % v for v in values)), file=out)

            if args.edram_every is not None and tick % args.edram_every == 0:
                args.edram_dir.mkdir(parents=True, exist_ok=True)
                blob = b"".join(
                    dbg.read(0x04000000 + off, n)
                    for off, n in (
                        (0x0, 512 * 272 * 4),
                        (0x88000, 512 * 272 * 4),
                        (0x110000, 256 * 136 * 4),
                        (0x132000, 256 * 136 * 4),
                    )
                )
                (args.edram_dir / ("tick%05d.edram" % tick)).write_bytes(blob)

            if args.shot_every is not None and tick % args.shot_every == 0:
                screenshot(args.shot_window, args.shot_dir, "tick%05d" % tick)

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
    sys.exit(main())
