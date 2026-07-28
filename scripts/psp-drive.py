#!/usr/bin/env python3
"""Drive a committed input script into a *free-running* PPSSPP, in real time.

This is the third way to put input into the original, and it exists because the
other two cannot finish a race.

`scripts/psp-trace.py --script` is breakpoint-driven: it stops the CPU once per
tick, reads the whole craft, and resumes. That is exactly right for a 200-tick
measurement and hopeless for a lap - a breakpoint round trip costs far more than
a frame, so the emulator runs at a small fraction of real time and a three-lap
time trial would take hours. `--hold` cannot steer at all.

So this module sends the same committed script (`verification/scenarios/*.inputs`,
`scripts/input_script.py`) at a **free-running** emulator, paced by the
emulator's own clock rather than by breakpoints:

    uv run --with websocket-client scripts/psp-drive.py drive \\
        --script verification/scenarios/talons-junction-lap.inputs --log /tmp/run.csv

The clock is `cpu.status`'s `ticks` - the PSP cycle counter - which is answered
by the debugger's own thread and costs about 0.1 ms whether the CPU is running or
not (`memory.read` costs 520 ms running, which is why nothing here polls memory
in the pacing loop). Dividing by `CYCLES_PER_FRAME` turns it into an emulated
frame index, and a run-length script only has to be *sent* where it changes, so
the pacing loop is a handful of writes over a whole lap.

**What this trades away, said out loud:** the emulator runs at full speed and is
never stopped, so nothing here can record per-tick ship state. A drive is how the
race gets finished; `psp-trace.py` is still how a trace gets captured. Position
logging here is a coarse *progress* log - one `memory.read` a second on a second
connection, from a thread, so the pacing loop never waits on it - and it is
explicitly not a trace.

Subcommands:

    preflight is there an emulator to talk to, and if not, what to start.
    menu      walk the front end from wherever it is into a live Time Trial, so a
              scripted run is autonomous from a cold boot rather than from
              "somebody already navigated the menus".
    restart   pause -> RESTART RACE -> dismiss the track description -> sit out
              the countdown, ending with the craft stationary on the start line.
              The reproducible starting point, since the debugger has no
              save-state command (see ppsspp-debugger.md).
    state     what the front end is doing, and where the craft is.
    drive     send a script, in real time, and log progress.
"""

import argparse
import struct
import sys
import threading
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import input_script
from ppsspp_debugger import Debugger

# The same breakpoint `psp-trace.py` captures on, used here exactly once per run:
# a craft's address is not stable across a restart, so a drive learns it with a
# single hit and then never stops the CPU again.
SHIP_UPDATE_CRAFT = 0x08849618
CRAFT_REGISTER = "a0"
BODY_POINTER = 0x1CC

# Craft and body offsets, from engine.md and psp-trace.py's own tables. Only the
# handful a progress log needs; psp-trace.py remains the authority on the rest.
CRAFT_THROTTLE = 0x2B8
CRAFT_STEER = 0x2C0
CRAFT_SPEED_CACHED = 0x2EC
BODY_FORWARD = 0x020
BODY_POSITION = 0x030
BODY_VELOCITY = 0x140
BODY_SPEED = 0x398

# PSP cycles per displayed frame. The CPU counter runs at 222 MHz and the
# hardware's vblank at 59.94 Hz - both confirmed live here (`gpu.stats.get`
# reports a 59.94 target and an actual within 0.02 of it).
PSP_CLOCK_HZ = 222_000_000
VBLANK_HZ = 59.940059940059946
CYCLES_PER_FRAME = PSP_CLOCK_HZ / VBLANK_HZ

# How long each leg of `restart` takes, measured against PPSSPP v1.20.4 and Pulse
# `UCUS98712` on 2026-07-28 by screenshotting the window at four-second intervals.
# They are sleeps rather than state polls because **the front end's state name is
# `InGame` for all of it** - loading, the track description and the countdown are
# not distinguishable through `0x08b31784`, so there is nothing to poll. Each is
# padded well past what was observed.
RESTART_TO_DESCRIPTION = 20.0
DESCRIPTION_TO_GREEN = 24.0

# Where a craft comes up on Talon's Junction White after the countdown, and how
# far off that still counts as the same track. Measured on three separate
# restarts on 2026-07-28, which agreed to every digit printed - the *heading*
# does not repeat that well, but the position does, and that is all this is for.
TALONS_JUNCTION_START = (6.07, -50.07, -196.10)
TALONS_JUNCTION_TOLERANCE = 40.0


def frame_of(dbg):
    """The emulator's own frame index, from the CPU cycle counter."""
    return int(dbg.call("cpu.status")["ticks"] / CYCLES_PER_FRAME)


def find_craft(dbg, timeout=60.0):
    """The craft's address, learned from a single breakpoint hit.

    A restart moves the craft, so this runs after `restart` rather than once per
    session. The CPU is stopped for one hit and then resumed; nothing else in a
    drive stops it.
    """
    for _, _ in dbg.each_hit(SHIP_UPDATE_CRAFT, 1, timeout=timeout):
        registers = dbg.call("cpu.getAllRegs")
        gpr = next(c for c in registers["categories"] if c["name"] == "GPR")
        craft = dict(zip(gpr["registerNames"], gpr["uintValues"]))[CRAFT_REGISTER]
    dbg.resume()
    return craft


def read_progress(dbg, craft):
    """Position, speed and the two control states, in two reads.

    Costs about a second on a running CPU, which is why this is called from a
    probe thread on its own connection and never from the pacing loop.
    """
    body = dbg.read_u32(craft + BODY_POINTER)
    blob = dbg.read(body, 0x400)

    def vec(at):
        return struct.unpack_from("<3f", blob, at)

    def one(at):
        return struct.unpack_from("<f", blob, at)[0]

    return {
        "pos": vec(BODY_POSITION),
        "vel": vec(BODY_VELOCITY),
        "fwd": vec(BODY_FORWARD),
        "speed": one(BODY_SPEED),
        "throttle": dbg.read_f32(craft + CRAFT_THROTTLE),
        "steer": dbg.read_f32(craft + CRAFT_STEER),
    }


class Probe(threading.Thread):
    """A progress log, on its own connection so the pacing loop never waits.

    Deliberately coarse: one sample a second or so, whenever the read happens to
    come back. It answers "where did the ship get to and was it still moving",
    which is what iterating on a script needs. It is not a trace and must not be
    read as one - the rows are not on tick boundaries and the interval is
    whatever the emulator's safe points allowed.
    """

    def __init__(self, port, craft, interval, out):
        super().__init__(daemon=True)
        self.port = port
        self.craft = craft
        self.interval = interval
        self.out = out
        self.stop = threading.Event()
        self.rows = []

    def run(self):
        try:
            dbg = Debugger(self.port)
        except Exception as error:  # noqa: BLE001 - a probe must never kill a run
            print("probe: no connection (%s)" % error, file=sys.stderr)
            return
        start = time.time()
        try:
            while not self.stop.is_set():
                try:
                    frame = frame_of(dbg)
                    sample = read_progress(dbg, self.craft)
                except Exception as error:  # noqa: BLE001
                    print("probe: %s" % error, file=sys.stderr)
                    break
                row = (time.time() - start, frame) + sample["pos"] + sample["vel"] + (
                    sample["speed"],
                    sample["throttle"],
                    sample["steer"],
                )
                self.rows.append(row)
                if self.out:
                    self.out.write(
                        "%.3f,%d,%s\n" % (row[0], row[1], ",".join("%.7g" % v for v in row[2:]))
                    )
                    self.out.flush()
                self.stop.wait(self.interval)
        finally:
            dbg.close()


def changes(states):
    """The script as `(tick, state)` change points.

    A run-length script held for a whole lap is a few dozen of these, so pacing
    over them costs a few dozen writes rather than one per frame. The first tick
    is always a change: nothing has been sent yet, which is not the same as
    nothing being held.
    """
    out = []
    for tick, state in enumerate(states):
        if not out or state != out[-1][1]:
            out.append((tick, state))
    return out


def send(dbg, state, sent):
    """One tick of scripted state at the emulator. Mirrors `psp-trace.py::send`."""
    dbg.hold(**input_script.button_payload(state))
    analog = input_script.analog_payload(state)
    if analog is None and sent["analog"] not in (None, (0.0, 0.0)):
        analog = (0.0, 0.0)
    if analog is not None:
        dbg.analog(*analog)
        sent["analog"] = analog


def drive(args):
    try:
        states = input_script.load(args.script)
    except input_script.ScriptError as error:
        print(error, file=sys.stderr)
        raise SystemExit(1) from None
    if not states:
        print("%s: the script has no ticks in it" % args.script, file=sys.stderr)
        raise SystemExit(1)
    for reason in input_script.unrepresentable(states):
        print("%s cannot be sent to a PSP - %s" % (args.script, reason), file=sys.stderr)
        raise SystemExit(1)

    ticks = args.ticks if args.ticks is not None else len(states)
    dbg = Debugger(args.port)
    dbg.resume()

    craft = args.craft
    if craft is None:
        craft = find_craft(dbg)
    print("craft at 0x%08x" % craft, file=sys.stderr)

    log = args.log.open("w") if args.log else None
    if log:
        log.write("wall_s,frame,pos_x,pos_y,pos_z,vel_x,vel_y,vel_z,speed,throttle,steer\n")
    probe = Probe(args.port, craft, args.probe_interval, log) if args.probe_interval else None

    points = changes([input_script.at(states, t + args.lead) for t in range(ticks)])
    print(
        "%s: %d tick(s), %d change point(s), lead %d"
        % (args.script, ticks, len(points), args.lead),
        file=sys.stderr,
    )

    sent = {"analog": None}
    base = frame_of(dbg)
    if probe:
        probe.start()
    began = time.time()
    late = 0
    try:
        for tick, state in points:
            target = base + tick
            while True:
                now = frame_of(dbg)
                if now >= target:
                    late = max(late, now - target)
                    break
                # Sleep most of the remaining wait, then spin the last frame or
                # two: `cpu.status` is cheap, but a sleep that overshoots is a
                # send on the wrong frame and there is no getting it back.
                remaining = (target - now) / VBLANK_HZ
                time.sleep(min(0.25, max(0.0005, remaining * 0.5)))
            send(dbg, state, sent)
        # Hold the last state out to the script's own end, so a drive covers the
        # ticks it says it does rather than ending at its last change.
        while frame_of(dbg) < base + ticks:
            time.sleep(0.05)
    finally:
        elapsed = time.time() - began
        if probe:
            probe.stop.set()
            probe.join(timeout=5.0)
        try:
            dbg.hold(**input_script.button_payload(input_script.State()))
            dbg.analog(0.0, 0.0)
        except Exception:  # noqa: BLE001
            pass
        if log:
            log.close()
        print(
            "drove %d tick(s) in %.1f s (%.2fx real time), worst send %d frame(s) late"
            % (ticks, elapsed, ticks / VBLANK_HZ / max(elapsed, 1e-9), late),
            file=sys.stderr,
        )
        if probe and probe.rows:
            first, last = probe.rows[0], probe.rows[-1]
            travelled = sum((last[2 + i] - first[2 + i]) ** 2 for i in range(3)) ** 0.5
            print(
                "probe: %d sample(s), %.1f units between first and last, final speed %.2f"
                % (len(probe.rows), travelled, last[8]),
                file=sys.stderr,
            )
        print("state: %s" % dbg.state_name(), file=sys.stderr)
        dbg.close()


# How many times a `left` is sent to drive an option selector onto its first
# entry. The Custom Race selectors **clamp** rather than wrap - measured on RACE
# TYPE, which sat on TOURNAMENT and read SINGLE RACE after eight - so saturating
# is a way to reach a known entry without being able to read the label. Eight is
# comfortably past the longest of them.
SATURATE = 8

# `right` presses from the saturated first entry to TIME TRIAL: the list runs
# SINGLE RACE, TOURNAMENT, TIME TRIAL. Screenshot-verified 2026-07-28, and
# selecting it also flips WEAPONS to OFF and AI DIFFICULTY to N/A on its own,
# which is the reference scenario's configuration.
RACE_TYPE_TIME_TRIAL = 2

# Front-end states, from `0x08b31784+0x18c`. The menu tree announces itself; the
# hex-grid cells do not, which is why the walk below verifies at these points and
# counts presses in between.
MAIN_MENU = "Main Menu"
RACEBOX = "Racebox"
CUSTOM_RACE = "Single Player"
TRACK_SELECT = "Track Creation"
IN_GAME = "InGame"

# First-boot dialogs, in the order they appear. Answering them is a one-time cost
# per memory stick - the SDL build persists the profile - but a walk that cannot
# get past them is a walk that needs a human on a fresh install.
FIRST_BOOT = [
    ("RemoveMemoryStickWarning", [("cross", 1)]),
    ("NameSetup2FromBoot", [("right", 10), ("cross", 1)]),
    ("TagSetup2FromBoot", [("right", 3), ("cross", 1)]),
    ("CreateFromBoot", [("cross", 1), ("circle", 1)]),
    ("Show Logo", [("start", 1)]),
    ("LogoFMV", [("start", 1)]),
    ("Language Selection", [("cross", 1)]),
]


def tap(dbg, button, times=1, wait=0.45):
    for _ in range(times):
        dbg.press(button, duration=6)
        time.sleep(wait)


def expect(dbg, wanted, what, timeout=15.0):
    """Wait for a named front-end state, or say which one turned up instead."""
    end = time.time() + timeout
    seen = None
    while time.time() < end:
        seen = dbg.state_name()
        if seen == wanted:
            return
        time.sleep(0.4)
    print(
        "the front end is in %r, not %r, after %s. The menu walk is in "
        "docs/reverse-engineering/ppsspp-debugger.md; if the layout has changed, "
        "that page and this function are what need updating." % (seen, wanted, what),
        file=sys.stderr,
    )
    raise SystemExit(1)


def settle_into_race(dbg, describe=True):
    """From the ship-select confirmation to a stationary craft on the start line.

    Sleeps rather than state polls, because the front end reads `InGame` for the
    loading screen, the track description and the countdown alike - there is
    nothing to poll. Nothing is held through any of it: thrust through a
    countdown is a false start, and Pulse answers one by stalling the engine.
    """
    time.sleep(RESTART_TO_DESCRIPTION)
    if describe:
        dbg.press("cross", duration=6)
        print("dismissed the track description, sitting out the countdown", file=sys.stderr)
    time.sleep(DESCRIPTION_TO_GREEN)
    craft = find_craft(dbg)
    sample = read_progress(dbg, craft)
    print(
        "craft 0x%08x at (%.2f, %.2f, %.2f), speed %.3f, throttle %.0f"
        % (craft, *sample["pos"], sample["speed"], sample["throttle"])
    )
    return craft, sample


def menu(args):
    """Walk the front end from wherever it is into a live Time Trial.

    Idempotent: already in a race, this returns straight away, so
    `just scripted-emu` can call it unconditionally.

    **What it pins and what it cannot.** RACE TYPE and SPEED CLASS are pinned by
    saturating their selectors, which clamp. The *track* is not: Track Select is a
    single wrapping list of three, its index is on screen and nowhere this can
    read, and there is no key sequence that reaches entry 1 from an unknown one in
    a wrapping list. So the walk leaves the track selector alone - the profile
    persists it, and nothing in this repository moves it - and **checks afterwards**
    that the craft came up on Talon's Junction, which is a real check rather than
    a hope. `--any-track` turns the check into a warning.
    """
    dbg = Debugger(args.port)
    dbg.resume()
    dbg.hold(**input_script.button_payload(input_script.State()))
    dbg.analog(0.0, 0.0)

    state = dbg.state_name()
    if state and IN_GAME in state:
        print("already in a race (%r)" % state)
        dbg.close()
        return

    # First boot, if this memory stick has never run the game.
    for _ in range(len(FIRST_BOOT) * 2):
        state = dbg.state_name()
        match = next((keys for name, keys in FIRST_BOOT if state == name), None)
        if match is None:
            break
        print("first boot: answering %r" % state, file=sys.stderr)
        for button, times in match:
            tap(dbg, button, times)
        time.sleep(1.5)

    # Back out of wherever the menus are. `circle` is Back on every screen of the
    # tree, and `Main Menu` is the one state name that cannot be mistaken.
    for _ in range(12):
        if dbg.state_name() == MAIN_MENU:
            break
        tap(dbg, "circle", 1, wait=1.5)
    expect(dbg, MAIN_MENU, "backing out with circle")
    print("at %r" % MAIN_MENU, file=sys.stderr)

    tap(dbg, "down")  # RACE CAMPAIGN -> RACEBOX
    tap(dbg, "cross", 1, wait=2.5)
    expect(dbg, RACEBOX, "entering Racebox")

    tap(dbg, "cross", 1, wait=3.0)  # CUSTOM RACE, the first entry
    expect(dbg, CUSTOM_RACE, "entering Custom Race")

    tap(dbg, "left", SATURATE, wait=0.3)  # RACE TYPE -> SINGLE RACE
    tap(dbg, "right", RACE_TYPE_TIME_TRIAL, wait=0.4)  # -> TIME TRIAL
    tap(dbg, "down")  # SPEED CLASS
    tap(dbg, "left", SATURATE, wait=0.3)  # -> VENOM
    tap(dbg, "up")
    print("set TIME TRIAL / VENOM", file=sys.stderr)

    tap(dbg, "cross", 1, wait=3.0)
    expect(dbg, TRACK_SELECT, "confirming the race settings")
    tap(dbg, "cross", 1, wait=3.0)  # the track, whichever is selected
    tap(dbg, "cross", 1, wait=3.0)  # the ship
    expect(dbg, IN_GAME, "confirming the ship", timeout=30.0)
    print("loading the race", file=sys.stderr)

    _, sample = settle_into_race(dbg)
    off = sum((a - b) ** 2 for a, b in zip(sample["pos"], TALONS_JUNCTION_START)) ** 0.5
    if off > TALONS_JUNCTION_TOLERANCE:
        message = (
            "the craft came up %.1f units from Talon's Junction's start line, so "
            "this is a different track. Track Select is a wrapping list this cannot "
            "read; choose Talon's Junction White once by hand and the profile keeps "
            "it." % off
        )
        if args.any_track:
            print("warning: " + message, file=sys.stderr)
        else:
            print(message, file=sys.stderr)
            raise SystemExit(1)
    else:
        print("on Talon's Junction, %.2f units from the recorded start line" % off)
    dbg.close()


def restart(args):
    """Back to a stationary craft on the start line, past the countdown.

    The sequence is `ppsspp-debugger.md`'s, with its two traps: the confirm on
    RESTART RACE is routinely swallowed, so it is pressed until the state
    actually leaves the pause menu; and **nothing may be held through the
    countdown** or Pulse stalls the engine for a false start.
    """
    dbg = Debugger(args.port)
    dbg.resume()
    dbg.hold(**input_script.button_payload(input_script.State()))
    dbg.analog(0.0, 0.0)

    state = dbg.state_name()
    if "Pause" not in state:
        dbg.press("start", duration=6)
        time.sleep(1.5)
        state = dbg.state_name()
    if "Pause" not in state:
        print("never reached the pause menu (state %r)" % state, file=sys.stderr)
        raise SystemExit(1)
    for _ in range(4):
        dbg.press("down", duration=4)
        time.sleep(0.35)
    for attempt in range(6):
        dbg.press("cross", duration=6)
        time.sleep(1.5)
        if "Pause" not in dbg.state_name():
            break
    else:
        print("RESTART RACE never took", file=sys.stderr)
        raise SystemExit(1)
    print("restarting (%d confirm(s))" % (attempt + 1), file=sys.stderr)

    _, sample = settle_into_race(dbg)
    if sample["speed"] > 1.0:
        print(
            "note: the craft is already moving, so the countdown is not over or "
            "something is held",
            file=sys.stderr,
        )
    dbg.close()


def preflight(args):
    """Say whether the original is ready to be scripted, and what to do if not.

    `just scripted-emu` runs this first so that the one failure it cannot fix -
    no emulator at all - comes out as a paragraph somebody can act on rather than
    as a websocket traceback from four frames deep in a capture. Everything past
    that point `menu` handles.
    """
    try:
        dbg = Debugger(args.port, connect_timeout=3.0)
    except Exception:  # noqa: BLE001 - the diagnosis is the whole point here
        image = Path(args.image)
        iso = Path(args.iso)
        print("no PPSSPP debugger answering on ws://127.0.0.1:%d/debugger" % args.port,
              file=sys.stderr)
        print(file=sys.stderr)
        if not image.exists():
            print("...and %s is missing too. data/ is gitignored and holds only "
                  "user-supplied images; see data/README.md." % image, file=sys.stderr)
        print("Start one (docs/reverse-engineering/ppsspp-debugger.md):", file=sys.stderr)
        print(file=sys.stderr)
        if not iso.exists():
            print("    just extract-iso %s %s" % (image, iso), file=sys.stderr)
        print(r"    printf '[General]\nRemoteDebuggerOnStartup = True\n"
              r"RemoteDebuggerLocal = True\nRemoteISOPort = %d\n' > /tmp/debugger.ini"
              % args.port, file=sys.stderr)
        print("    SDL_VIDEODRIVER=wayland %s --appendconfig=/tmp/debugger.ini "
              "--windowed %s" % (args.emulator, iso), file=sys.stderr)
        print(file=sys.stderr)
        print("SDL_VIDEODRIVER=wayland is not optional on a Wayland session, and the "
              "window must be focused or the build throttles hard. The menus from "
              "there are `psp-drive.py menu`'s job, not yours.", file=sys.stderr)
        raise SystemExit(1)

    try:
        name = dbg.state_name()
        print("PPSSPP on port %d, front end in %r" % (args.port, name))
        if not name or IN_GAME not in name:
            print("not in a race - `psp-drive.py menu` will walk there", file=sys.stderr)
        print("ready")
    finally:
        dbg.close()


def state(args):
    dbg = Debugger(args.port)
    print("state: %s" % dbg.state_name())
    craft = args.craft if args.craft is not None else find_craft(dbg)
    sample = read_progress(dbg, craft)
    print("craft: 0x%08x" % craft)
    print("pos: (%.3f, %.3f, %.3f)" % sample["pos"])
    print("vel: (%.3f, %.3f, %.3f)" % sample["vel"])
    print("fwd: (%.3f, %.3f, %.3f)" % sample["fwd"])
    print("speed: %.3f  throttle: %.1f  steer: %.1f"
          % (sample["speed"], sample["throttle"], sample["steer"]))
    dbg.close()


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--port", type=int, default=47810)
    sub = parser.add_subparsers(dest="command", required=True)

    p = sub.add_parser(
        "preflight", help="is the original ready to be scripted, and if not, why"
    )
    p.add_argument("--image", default="data/images/pulse-psp-usa.chd")
    p.add_argument("--iso", default="data/cache/pulse-psp-usa.iso")
    p.add_argument("--emulator", default="PPSSPPSDL")
    p.set_defaults(run=preflight)

    p = sub.add_parser(
        "menu", help="walk the front end from wherever it is into a live Time Trial"
    )
    p.add_argument(
        "--any-track",
        action="store_true",
        help="warn instead of failing when the race did not come up on Talon's "
        "Junction. Track Select is a wrapping list this cannot read.",
    )
    p.set_defaults(run=menu)

    p = sub.add_parser("restart", help="back to a stationary craft on the start line")
    p.set_defaults(run=restart)

    p = sub.add_parser("state", help="what the front end and the craft are doing")
    p.add_argument("--craft", type=lambda v: int(v, 0))
    p.set_defaults(run=state)

    p = sub.add_parser("drive", help="send an input script at a free-running emulator")
    p.add_argument("--script", type=Path, required=True)
    p.add_argument("--ticks", type=int, help="defaults to the script's own length")
    p.add_argument(
        "--lead",
        type=int,
        default=0,
        metavar="N",
        help="send the script's tick k+N at emulated frame k. The free-running "
        "path does not pay psp-trace.py's three-frame breakpoint latency, so this "
        "defaults to 0 rather than to --script-lead's 2.",
    )
    p.add_argument("--craft", type=lambda v: int(v, 0))
    p.add_argument("--log", type=Path, metavar="CSV", help="write the progress log here")
    p.add_argument(
        "--probe-interval",
        type=float,
        default=1.0,
        metavar="SECONDS",
        help="how often the probe thread samples the craft. 0 disables it.",
    )
    p.set_defaults(run=drive)

    args = parser.parse_args()
    args.run(args)


if __name__ == "__main__":
    main()
