#!/usr/bin/env python3
"""Fly Wipeout Pulse around a lap in PPSSPP, and write down what it pressed.

A scripted lap has to be authored from somewhere, and nobody wants to hand-write
six thousand ticks of `left`/`right`. This closes the loop instead: it breaks in
`Ship_UpdateCraft` every tick exactly the way `scripts/psp-trace.py` does, reads
the craft, steers it toward the track's own spline - dumped by
`oag-trace track`, so the line comes off the disc rather than out of a guess -
and **records the input it chose**. The recording is emitted as a committed input
script in `scripts/input_script.py`'s format, which is then the authored,
reusable artefact: `psp-trace.py --script` replays it into the emulator and
`oag-trace run --script` replays it into our own physics.

    cargo run -q -p oag-trace -- track --source data/images/pulse-psp-usa.chd \\
        > /tmp/spline.csv
    uv run --with websocket-client scripts/psp-drive.py restart
    uv run --with websocket-client scripts/psp-autopilot.py --spline /tmp/spline.csv \\
        --laps 1 --script-out verification/scenarios/talons-junction-lap.inputs \\
        --trace-out data/traces/talons-junction-autopilot.csv

Because it is breakpoint-driven it also writes a trace in `psp-trace.py`'s exact
columns for free: the run that finishes the race and the capture of it are the
same run, which is the only way to have both without hoping two runs agree.

**The controller is not a claim about the original's AI.** It is a pure-pursuit
steerer with a bang-bang d-pad, written to get a craft round a circuit, and
nothing in `docs/` should ever cite it. What is a claim is the *recording*: the
per-tick button state it produces is exactly what the emulator was sent.
"""

import argparse
import csv
import math
import struct
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import input_script
from niri_shot import find_window, screenshot
from ppsspp_debugger import Debugger
from psp_trace_fields import (
    BODY_FIELDS,
    CRAFT_FIELDS,
    ENTITY_FIELDS,
    ENTITY_OWNER,
    ENTITY_POINTER,
    SHIP_UPDATE_CRAFT,
)

CRAFT_REGISTER = "a0"
CRAFT_BYTES = 0x400
BODY_POINTER = 0x1CC

# Body offsets the controller itself reads. The trace columns come from the
# shared table; these three are named because the steering law talks about them.
BODY_ROW0 = 0x000  # measured to be the ship's LEFT, not its right - see replay.rs
BODY_UP = 0x010
BODY_FORWARD = 0x020
BODY_POSITION = 0x030
BODY_VELOCITY = 0x140


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def add(a, b):
    return (a[0] + b[0], a[1] + b[1], a[2] + b[2])


def scale(a, k):
    return (a[0] * k, a[1] * k, a[2] * k)


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def length(a):
    return math.sqrt(dot(a, a))


def unit(a):
    n = length(a)
    return (a[0] / n, a[1] / n, a[2] / n) if n > 1e-9 else (0.0, 0.0, 0.0)


class Line:
    """The track's spline as a ring of points, with cumulative distance.

    `oag-trace track` emits the paths in file order, and on a circuit the last
    path's end is the first path's start, so treating the whole table as one ring
    is exactly the lap. A fork would need the junction graph; Talon's Junction's
    two paths are a plain ring and this says so rather than pretending to handle
    the general case - `--spline` output with more than one *loop* is out of
    scope and the closing gap below is what would catch it.
    """

    def __init__(self, path, column="line"):
        self.points = []
        with open(path, newline="") as handle:
            for row in csv.DictReader(handle):
                self.points.append(
                    tuple(float(row["%s_%s" % (column, axis)]) for axis in "xyz")
                )
        if len(self.points) < 8:
            raise SystemExit("%s: only %d point(s)" % (path, len(self.points)))
        self.cumulative = [0.0]
        for a, b in zip(self.points, self.points[1:]):
            self.cumulative.append(self.cumulative[-1] + length(sub(b, a)))
        self.closing = length(sub(self.points[0], self.points[-1]))
        self.total = self.cumulative[-1] + self.closing

    def __len__(self):
        return len(self.points)

    def nearest(self, position, around, back=12, ahead=90):
        """The nearest point within a window of `around`, in ring order.

        Windowed rather than global so that a track which passes near itself -
        every circuit does - cannot teleport the progress counter, and so a
        craft going backwards is visible as a stalled index rather than hidden
        as a match somewhere else entirely.
        """
        n = len(self.points)
        best, best_d = around, float("inf")
        for offset in range(-back, ahead + 1):
            index = (around + offset) % n
            d = length(sub(self.points[index], position))
            if d < best_d:
                best, best_d = index, d
        return best, best_d

    def converge_to(self, start_index, gate, gate_index):
        """Bend the ring between two indices so it ends on `gate`.

        The same ramp `oag_trace::plan::Path::to_gate` builds, and for the same
        reason: a gate that does not sit on the authored racing line has to be
        converged onto over the whole approach rather than swerved at in the last
        few metres. The gate's own offset from the line is faded in linearly from
        nothing at `start_index` to all of it at `gate_index`; a gate already on
        the line leaves the ring untouched.

        The ring is mutated in place and its arc lengths recomputed, because the
        lap counter reads them. Only the arc *between* the two indices moves, so
        a lap that continues past the gate is back on the authored line.
        """
        n = len(self.points)
        span = (gate_index - start_index) % n
        offset = sub(gate, self.points[gate_index])
        for step in range(span + 1):
            at = (start_index + step) % n
            fraction = step / span if span else 1.0
            self.points[at] = add(self.points[at], scale(offset, fraction))
        self.cumulative = [0.0]
        for a, b in zip(self.points, self.points[1:]):
            self.cumulative.append(self.cumulative[-1] + length(sub(b, a)))
        self.closing = length(sub(self.points[0], self.points[-1]))
        self.total = self.cumulative[-1] + self.closing

    def ahead(self, index, distance):
        """The point `distance` further along the ring."""
        n = len(self.points)
        travelled = 0.0
        at = index
        while travelled < distance:
            step = length(sub(self.points[(at + 1) % n], self.points[at]))
            travelled += step
            at = (at + 1) % n
            if at == index:
                break
        return self.points[at]


def read_floats(blob, table):
    return {name: struct.unpack_from("<f", blob, at)[0] for name, at in table}


def vec(blob, at):
    return struct.unpack_from("<3f", blob, at)


def parse_vec(parser, flag, text):
    """An `x,y,z` command-line vector, or None."""
    if text is None:
        return None
    parts = text.split(",")
    if len(parts) != 3:
        parser.error("%s wants three comma-separated numbers, got %r" % (flag, text))
    try:
        return tuple(float(p) for p in parts)
    except ValueError:
        parser.error("%s: %r is not three numbers" % (flag, text))
        return None


def steer_for(state, line, index, args):
    """One tick of the controller: what to hold, given where the craft is.

    Pure pursuit. The aim point is a fixed distance ahead along the spline,
    scaled with speed so a fast craft looks further; the error is that
    direction's component along the ship's own right axis, which is the d-pad
    axis the game's own steering ramp consumes. Bang-bang, because a PSP d-pad
    has three positions and the scripted format is a list of button names.

    The airbrake on the inside of a turn is what a player does and what
    `docs/ghidra/functions/psp-pulse-usa/engine.md`'s force law rewards: it is added
    only past a wider threshold than the steering's, so a straight is never
    braked.
    """
    speed = length(state["velocity"])
    look = min(args.look_max, args.look_min + args.look_speed * speed)
    target = line.ahead(index, look)
    want = unit(sub(target, state["position"]))
    # Row 0 of the recorded basis is the ship's LEFT (measured, confidence 84),
    # so the body's right is its negation. Getting this backwards steers into
    # every wall on the circuit, which is at least a loud failure.
    right = tuple(-c for c in state["row0"])
    error = dot(want, right)
    forward = dot(want, state["forward"])

    held = ["cross"]
    if error > args.deadband:
        held.append("right")
        if error > args.brake_at:
            held.append("r")
    elif error < -args.deadband:
        held.append("left")
        if error < -args.brake_at:
            held.append("l")
    # Aimed away from where the line goes: thrust would only drive it further,
    # so coast and let the steering come round.
    if forward < args.reverse_at:
        held = [b for b in held if b != "cross"]
    return held, error, look


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--port", type=int, default=47810)
    parser.add_argument("--spline", type=Path, required=True, help="`oag-trace track` output")
    parser.add_argument(
        "--column",
        default="line",
        choices=("line", "lift", "pos"),
        help="which spline column to chase: the authored racing line, the lifted "
        "centre, or the raw surface centre",
    )
    parser.add_argument("--laps", type=float, default=1.0)
    parser.add_argument(
        "--gate",
        metavar="X,Y,Z",
        help="drive at this world point instead of round the lap: the spline is "
        "bent to end on it - see Line.converge_to - and the run stops once the "
        "craft has crossed the plane through it. This is the closed-loop route "
        "to a target: the controller reads the emulator's own craft state every "
        "tick, so it corrects for the emulator instead of hoping an open-loop "
        "script transfers. `oag-trace plan` is the offline route to the same "
        "place. See docs/tools/autopilot-planning.md.",
    )
    parser.add_argument(
        "--gate-dir",
        metavar="X,Y,Z",
        help="the direction to cross --gate in. Defaults to the spline's own "
        "heading there, which is what a gate across the track means. "
        "`oag-trace pads` prints a speed pad's centre and push axis.",
    )
    parser.add_argument(
        "--gate-after",
        type=int,
        default=60,
        metavar="N",
        help="keep driving N ticks past the gate, so a recording does not end on it",
    )
    parser.add_argument("--max-ticks", type=int, default=30000)
    parser.add_argument("--script-out", type=Path, help="write the recorded input script here")
    parser.add_argument(
        "--emit-lead",
        type=int,
        default=2,
        metavar="N",
        help="shift the emitted script N ticks later than it was sent, so the "
        "committed file is in `oag-trace`'s convention rather than the "
        "breakpoint's. Input set at a breakpoint lands three frames later and a "
        "replay's row k is driven by input k-1, so the emulator is two ticks "
        "behind us; the default undoes that here, once, and `psp-trace.py "
        "--script-lead 2` puts it back when the file is replayed into the "
        "emulator. 0 emits exactly what was sent.",
    )
    parser.add_argument("--trace-out", type=Path, help="write a psp-trace.py-shaped CSV here")
    parser.add_argument("--look-min", type=float, default=18.0)
    parser.add_argument("--look-speed", type=float, default=0.55)
    parser.add_argument("--look-max", type=float, default=90.0)
    parser.add_argument("--deadband", type=float, default=0.045)
    parser.add_argument("--brake-at", type=float, default=0.32)
    parser.add_argument("--reverse-at", type=float, default=-0.2)
    parser.add_argument(
        "--shot-dir",
        type=Path,
        help="screenshot the emulator's window here at every spline-lap boundary "
        "and at the end, so the game's own HUD says what the race did",
    )
    parser.add_argument(
        "--shot-window",
        type=int,
        help="niri window id of the emulator. Defaults to the first PPSSPPSDL "
        "window (`niri msg windows` lists them), since ids change every launch.",
    )
    parser.add_argument(
        "--stall-ticks",
        type=int,
        default=1200,
        help="give up after this many ticks with no progress along the spline",
    )
    args = parser.parse_args()

    gate = parse_vec(parser, "--gate", args.gate)
    gate_dir = parse_vec(parser, "--gate-dir", args.gate_dir)
    if gate_dir is not None and gate is None:
        parser.error("--gate-dir needs --gate")

    if args.shot_dir and args.shot_window is None:
        args.shot_window = find_window()
        if args.shot_window is None:
            parser.error(
                "no PPSSPPSDL window found to screenshot; pass --shot-window ID"
            )
        print("screenshotting niri window %d" % args.shot_window, file=sys.stderr)

    line = Line(args.spline, args.column)
    print(
        "%s: %d point(s), %.1f units round, closing gap %.2f"
        % (args.spline, len(line), line.total, line.closing),
        file=sys.stderr,
    )

    dbg = Debugger(args.port)
    dbg.resume()

    trace = args.trace_out.open("w") if args.trace_out else None
    if trace:
        trace.write(
            "tick,"
            + ",".join(
                [n for n, _ in CRAFT_FIELDS]
                + [n for n, _ in ENTITY_FIELDS]
                + [n for n, _ in BODY_FIELDS]
            )
            + "\n"
        )

    recorded = []
    tick = 0
    index = None
    start_index = None
    laps = 0.0
    best_progress = 0
    stalled_since = 0
    gate_index = None
    gate_normal = None
    crossed_at = None
    gate_distance = None
    began = time.time()
    craft = None
    body = None
    sent = None

    try:
        for _, _ in dbg.each_hit(SHIP_UPDATE_CRAFT, args.max_ticks + 8, timeout=60):
            if tick >= args.max_ticks:
                break
            if craft is None:
                registers = dbg.call("cpu.getAllRegs")
                gpr = next(c for c in registers["categories"] if c["name"] == "GPR")
                craft = dict(zip(gpr["registerNames"], gpr["uintValues"]))[CRAFT_REGISTER]
                print("craft at 0x%08x" % craft, file=sys.stderr)
            craft_blob = dbg.read(craft, CRAFT_BYTES)
            body = struct.unpack_from("<I", craft_blob, BODY_POINTER)[0]
            body_blob = dbg.read(body, CRAFT_BYTES)

            state = {
                "position": vec(body_blob, BODY_POSITION),
                "velocity": vec(body_blob, BODY_VELOCITY),
                "row0": vec(body_blob, BODY_ROW0),
                "up": vec(body_blob, BODY_UP),
                "forward": vec(body_blob, BODY_FORWARD),
            }

            if index is None:
                index, distance = line.nearest(state["position"], 0, back=len(line), ahead=0)
                start_index = index
                print(
                    "starting at spline point %d, %.2f units off the line"
                    % (index, distance),
                    file=sys.stderr,
                )
                if gate is not None:
                    gate_index, gate_off = line.nearest(
                        gate, index, back=0, ahead=len(line) - 1
                    )
                    gate_normal = unit(
                        gate_dir
                        if gate_dir is not None
                        else sub(
                            line.points[(gate_index + 1) % len(line)],
                            line.points[gate_index],
                        )
                    )
                    line.converge_to(index, gate, gate_index)
                    print(
                        "gate (%.2f, %.2f, %.2f) at spline point %d, %.2f units off "
                        "the line, crossing (%.3f, %.3f, %.3f)"
                        % (gate + (gate_index, gate_off) + gate_normal),
                        file=sys.stderr,
                    )
            previous = index
            index, off = line.nearest(state["position"], index)
            # Signed arc progress, wrapped the short way round, so a craft that
            # rocks back and forth against a wall does not accumulate laps out
            # of the oscillation. This counter is the run's *stopping* rule, not
            # its evidence - what the race did is read off the game's own lap
            # counter afterwards.
            delta = line.cumulative[index] - line.cumulative[previous]
            if delta > line.total / 2:
                delta -= line.total
            elif delta < -line.total / 2:
                delta += line.total
            was = int(laps)
            laps += delta / line.total
            if int(laps) != was:
                print(
                    "spline lap %d at tick %d, race clock is the game's own"
                    % (int(laps), tick),
                    file=sys.stderr,
                    flush=True,
                )
                screenshot(args.shot_window, args.shot_dir, "lap%d-tick%05d" % (int(laps), tick))
            if delta > 0:
                best_progress = tick
            if tick - best_progress > args.stall_ticks:
                print(
                    "no progress for %d tick(s) at spline point %d - giving up"
                    % (args.stall_ticks, index),
                    file=sys.stderr,
                )
                break

            if gate_normal is not None:
                was, gate_distance = gate_distance, dot(sub(state["position"], gate), gate_normal)
                if crossed_at is None and was is not None and was < 0.0 <= gate_distance:
                    crossed_at = tick
                    print(
                        "crossed the gate at tick %d, (%.2f, %.2f, %.2f) doing %.2f"
                        % ((tick,) + state["position"] + (length(state["velocity"]),)),
                        file=sys.stderr,
                        flush=True,
                    )
                if crossed_at is not None and tick - crossed_at >= args.gate_after:
                    print("%d tick(s) past the gate - stopping" % args.gate_after, file=sys.stderr)
                    break

            held, error, look = steer_for(state, line, index, args)
            script_state = input_script.parse("1 %s\n" % (" ".join(held) or "none"))[0]
            recorded.append(script_state)

            if trace:
                values = [struct.unpack_from("<f", craft_blob, at)[0] for _, at in CRAFT_FIELDS]
                # The energy pool, one hop out at craft+0x1c4 - the same walk
                # psp-trace.py makes, with the same reciprocal check. Written
                # here too so a lap capture carries the column: the whole point
                # of one address book is that the two writers cannot drift.
                entity = struct.unpack_from("<I", craft_blob, ENTITY_POINTER)[0]
                if dbg.read_u32(entity + ENTITY_OWNER) != craft:
                    raise SystemExit(
                        "entity 0x%08x does not point back at craft 0x%08x - "
                        "the chain in psp_trace_fields.py no longer holds"
                        % (entity, craft)
                    )
                entity_blob = dbg.read(entity + ENTITY_FIELDS[0][1], 4 * len(ENTITY_FIELDS))
                values += [
                    struct.unpack_from("<f", entity_blob, at - ENTITY_FIELDS[0][1])[0]
                    for _, at in ENTITY_FIELDS
                ]
                values += [struct.unpack_from("<f", body_blob, at)[0] for _, at in BODY_FIELDS]
                trace.write("%d,%s\n" % (tick, ",".join("%.7g" % v for v in values)))

            if sent is None or script_state != sent:
                dbg.hold(**input_script.button_payload(script_state))
                sent = script_state

            if tick % 300 == 0:
                print(
                    "tick %5d  point %4d  off %5.1f  speed %6.2f  err %+.3f  look %4.1f  laps %.3f"
                    % (tick, index, off, length(state["velocity"]), error, look, laps),
                    file=sys.stderr,
                    flush=True,
                )
            tick += 1
            if laps >= args.laps:
                print("reached %.3f lap(s) at tick %d" % (laps, tick), file=sys.stderr)
                break
    except TimeoutError:
        # `Ship_UpdateCraft` stops being called the moment the race ends and the
        # results screen takes over, so the breakpoint simply never fires again.
        # That is a finished race, not a failure, and the recording so far is the
        # whole of the race.
        print(
            "the craft update stopped firing at tick %d - the race is over" % tick,
            file=sys.stderr,
        )
    finally:
        elapsed = time.time() - began
        screenshot(args.shot_window, args.shot_dir, "end-tick%05d" % tick)
        try:
            dbg.resume()
            dbg.hold(**input_script.button_payload(input_script.State()))
            dbg.analog(0.0, 0.0)
        except Exception:  # noqa: BLE001
            pass
        if trace:
            trace.close()
        print(
            "%d tick(s) in %.1f s (%.1f ticks/s), %.3f lap(s), spline point %s"
            % (tick, elapsed, tick / max(elapsed, 1e-9), laps, index),
            file=sys.stderr,
        )
        if args.script_out and recorded:
            args.script_out.write_text(to_script(recorded, args), encoding="utf-8")
            print(
                "wrote %s (%d tick(s))" % (args.script_out, len(recorded)),
                file=sys.stderr,
            )
        try:
            print("state: %s" % dbg.state_name(), file=sys.stderr)
        except Exception:  # noqa: BLE001
            pass
        dbg.close()


def to_script(states, args):
    """The recorded per-tick input, run-length encoded in the committed format.

    Shifted by `--emit-lead` so the file is canonical rather than raw: see that
    option's help for the two-tick argument.
    """
    if args.emit_lead > 0:
        states = [states[0]] * args.emit_lead + states[: -args.emit_lead]
    out = [
        "# Recorded by scripts/psp-autopilot.py chasing %s (%s column)." % (args.spline, args.column),
        "# Every line is what the emulator was actually sent on those ticks,",
        "# shifted %d tick(s) later so the file is in oag-trace's convention;" % args.emit_lead,
        "# replay it into the emulator with `psp-trace.py --script-lead %d`." % args.emit_lead,
        "",
    ]
    run = 1
    for previous, state in zip(states, states[1:]):
        if state == previous:
            run += 1
            continue
        out.append("%d %s" % (run, " ".join(previous.held_names()) or "none"))
        run = 1
    out.append("%d %s" % (run, " ".join(states[-1].held_names()) or "none"))
    return "\n".join(out) + "\n"


if __name__ == "__main__":
    main()
