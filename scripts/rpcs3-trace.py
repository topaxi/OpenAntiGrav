#!/usr/bin/env python3
"""Per-tick trace of the player craft in a running Wipeout HD race on RPCS3.

The PS3 counterpart of `scripts/psp-trace.py`, built on the no-pause guest
memory read `scripts/rpcs3-mem-poll.py` measured (`/proc/<pid>/mem` at
`0x300000000 + A` is guest address `A`). See
`docs/reverse-engineering/rpcs3-capture.md`, "A per-tick craft trace".

    OAG_RPCS3_DISPLAY=97 OAG_RPCS3_GDB=127.0.0.1:23497 \\
    OAG_RPCS3_PAD_NAME="OAG Pad <lane>" XDG_CONFIG_HOME=... XDG_CACHE_HOME=... \\
        uv run --with evdev python3 scripts/rpcs3-trace.py \\
        --image <abs>/hdfury-ps3-eu-dec.iso --out <dir> \\
        --run name=verification/scenarios/hd-thrust.inputs[@x,y,z,yaw[,speed]] ...

One boot runs every `--run`. Each one optionally teleports the player
(`rpcs3_place`), then plays the script **keyed on the game's own clock**
(`craft+0x308`), not on host time or frames: HD integrates each frame with that
frame's own delta and RPCS3 sometimes drops to 30 fps, so a frame-keyed script
holds a state for a game time that depends on the emulator. Every sample is the body's state plus the
control values the craft itself read (`craft+0x30c` throttle, the
`PlayerInput` record's steer and pitch), so the trace records what the game
received, latency included, rather than what was pressed.

Writes `<out>/<name>.csv` with one row per game frame seen (`time` is game
seconds since the script started, `dt` the frame's own step; a frame the
poller missed is absent, never interpolated) and `<out>/<name>.raw.pkl`.
HD runs a variable step here (RPCS3 drops to 30 fps at times), so compare in
game time, never by row.
"""
import argparse
import csv
import math
import importlib.util
import pickle
import struct
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import emu_guard  # noqa: E402
_spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)
import rpcs3_place as place  # noqa: E402
import input_script  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402

GUEST_BASE = 0x300000000
CODE_CHECK = (0x155568, bytes.fromhex("39400010"))
#: `PlayerInput`'s vtable (`docs/ghidra/functions/ps3-hdfury-eu/player-input.md`).
PLAYER_INPUT_VTABLE = 0x00863490
#: `0x00992ce0`'s `+0x4`; read 0 throughout a race here, unused.
FRAME_COUNTER = 0x00992CE4
SUBSTEPS = 0x008C1758
HALF_STEP = 0x00938560
WORLD_TICK = 0x40
WORLD_DT = 0x44
WORLD_BODIES = 0x48
WORLD_COUNT = 0x2C8


class Guest:
    def __init__(self, pid, mem=None):
        self.mem = mem if mem is not None else open("/proc/%d/mem" % pid, "rb", 0)

    def read(self, addr, n):
        emu_guard.beat()
        self.mem.seek(GUEST_BASE + addr)
        return self.mem.read(n)

    def u32(self, addr):
        return struct.unpack(">I", self.read(addr, 4))[0]

    def f32(self, addr):
        return struct.unpack(">f", self.read(addr, 4))[0]

    def scan(self, needle, lo=0x30000000, hi=0x40000000):
        hits = []
        for a in range(lo, hi, 0x100000):
            try:
                blk = self.read(a, 0x100000)
            except OSError:
                continue
            i = blk.find(needle)
            while i >= 0:
                if i % 4 == 0:
                    hits.append(a + i)
                i = blk.find(needle, i + 4)
        return hits


def find_world(guest, body):
    """An object whose inline pointer array holds `body`; its `+0x40` counts frames.

    Found the way `physics.md` describes the physics world (`+0x48` array,
    `+0x2c8` count), but what this finds carries vtable `0x00863780` and its
    `+0x40` advances once per *frame* (by 1 while the game clock advances by
    1/60 or 2/60), so it is used only as a frame counter for the reader.
    """
    for hit in guest.scan(struct.pack(">I", body)):
        for slot in range(160):
            world = hit - WORLD_BODIES - 4 * slot
            if world < 0x30000000:
                break
            try:
                count = guest.u32(world + WORLD_COUNT)
            except OSError:
                continue
            if 1 <= count <= 160 and slot < count:
                bodies = struct.unpack(">%dI" % count, guest.read(world + WORLD_BODIES, 4 * count))
                if bodies[slot] == body and all(0x30000000 <= b < 0x40000000 for b in bodies):
                    return world
    return None


def snapshot(guest, objs):
    """One raw sample: every watched block, read back to back."""
    return {name: guest.read(addr, n) for name, (addr, n) in objs.items()}


#: `*(*0x008b098c)` is the options object; `+0x473` is Pilot Assist, which
#: `FUN_0022f8c8` (the FirstPlayDialog handler) sets to 1 with the string
#: "change to pilotassist enabled" and to 0 with "...disabled", `+0x474` beside it.
OPTIONS_PTR = 0x008B098C
OPT_PILOT_ASSIST = 0x473

#: Script button -> pad button. `l`/`r` are this engine's airbrakes, and on HD
#: they are L2/R2 (measured 2026-10-08: L2 turns left, R2 right, L1/R1 move
#: nothing; `PlayerInput+0x54/+0x58` read 100 on the press).
DEFAULT_MAP = {"cross": "cross", "circle": "circle", "square": "square",
               "triangle": "triangle", "up": "up", "down": "down",
               "left": "left", "right": "right", "l": "l2", "r": "r2",
               "start": "start", "select": "select"}

HEADER = ["time", "dt", "throttle", "steer", "pitch", "airbrake_l", "airbrake_r",
          "right_x", "right_y", "right_z", "up_x", "up_y", "up_z",
          "fwd_x", "fwd_y", "fwd_z", "pos_x", "pos_y", "pos_z",
          "vel_x", "vel_y", "vel_z", "speed", "world_tick", "rival_dist", "pressed"]


def set_pilot_assist(gdb, on):
    gdb.pause()
    holder = struct.unpack(">I", gdb.read(OPTIONS_PTR, 4))[0]
    opts = struct.unpack(">I", gdb.read(holder, 4))[0]
    before = gdb.read(opts + OPT_PILOT_ASSIST, 2)
    gdb.write(opts + OPT_PILOT_ASSIST, bytes([int(on), int(on)]))
    after = gdb.read(opts + OPT_PILOT_ASSIST, 2)
    gdb.resume()
    print("pilot assist %s -> %s (options %#x)" % (before.hex(), after.hex(), opts), flush=True)


def apply_buttons(pad, want, held):
    for name in sorted(held - want):
        pad.set(name, False)
    for name in sorted(want - held):
        pad.set(name, True)
    return set(want)


def row_of(game_t, dt, wt, s, extra, inputs_key, pressed):
    b = s["body"]
    vel = struct.unpack_from(">3f", b, 0x190)
    r0 = struct.unpack_from(">3f", b, 0x1D0)
    up = struct.unpack_from(">3f", b, 0x1E0)
    fw = struct.unpack_from(">3f", b, 0x1F0)
    pos = struct.unpack_from(">3f", b, 0x200)
    inp = s.get(inputs_key)
    steer = struct.unpack_from(">f", inp, 0x4C)[0] / 100.0 if inp else 0.0
    pitch = struct.unpack_from(">f", inp, 0x5C)[0] / 100.0 if inp else 0.0
    brake_l = struct.unpack_from(">f", inp, 0x54)[0] / 100.0 if inp else 0.0
    brake_r = struct.unpack_from(">f", inp, 0x58)[0] / 100.0 if inp else 0.0
    throttle = struct.unpack_from(">f", s["entry"], 0x30C)[0] / 100.0
    # Our trace's `right` column is cross(forward, up); HD's row 0 is
    # cross(up, forward), so the column is its negation.
    right = tuple(-c for c in r0)
    speed = sum(v * v for v in vel) ** 0.5
    return [game_t, dt, throttle, steer, pitch, brake_l, brake_r,
            *right, *up, *fw, *pos, *vel, speed, wt, extra.get("rival", -1.0), pressed]


def rival_distance(guest, me_pos, rival_bodies):
    best = 1e9
    for body in rival_bodies:
        p = struct.unpack(">3f", guest.read(body + 0x200, 12))
        d = sum((a - b) ** 2 for a, b in zip(p, me_pos)) ** 0.5
        best = min(best, d)
    return best


#: `craft+0x308` (the class entry): the game clock in seconds. It advances by
#: each frame's own delta - 1/60 when RPCS3 keeps up, 2/60 when it drops to
#: 30 fps - and distance travelled over speed times its step is 0.996-0.999
#: across twenty runs, so it is the delta the craft is integrated with.
ENTRY_CLOCK = 0x308


def run_script(session, guest, objs, states, mapping, settle_ticks, rival_bodies):
    """Settle with nothing held, then hold `states[k]` from game time `k/60`.

    Keyed on the game's own clock rather than on frames: HD integrates each
    frame with that frame's delta, so a frame-keyed script would hold a state
    for a game time that depends on the emulator's frame rate.
    """
    entry = objs["entry"][0]
    frame_of = objs["world"][0] + WORLD_TICK

    def clock():
        return guest.f32(entry + ENTRY_CLOCK)

    held = apply_buttons(session.pad, set(), set())
    t = clock()
    while clock() < t + settle_ticks / 60.0:
        time.sleep(0.002)
    t0 = clock()
    raw, last = [], None
    torn = [0]
    while True:
        now = clock()
        k = int((now - t0) * 60.0 + 1e-4)
        if k >= len(states):
            break
        want = {mapping[n] for n in states[k].held_names() if n in mapping}
        if want != held:
            held = apply_buttons(session.pad, want, held)
        frame = guest.u32(frame_of)
        if frame != last:
            # Consistent only if the frame did not move under the reads and the
            # body read back identical.
            s = snapshot(guest, objs)
            body_again = guest.read(objs["body"][0], objs["body"][1])
            if guest.u32(frame_of) == frame and body_again == s["body"]:
                pos = struct.unpack_from(">3f", s["body"], 0x200)
                game_t = struct.unpack_from(">f", s["entry"], ENTRY_CLOCK)[0] - t0
                raw.append((game_t, frame, s, rival_distance(guest, pos, rival_bodies), "+".join(sorted(held))))
                last = frame
            else:
                torn[0] += 1
        time.sleep(0.001)
    apply_buttons(session.pad, set(), held)
    print("  %d torn read(s) retried" % torn[0], flush=True)
    return raw


def parse_run(text):
    """`name=script.inputs[@x,y,z,yaw[,speed]]`."""
    name, _, rest = text.partition("=")
    script, _, pose = rest.partition("@")
    pose = [float(v) for v in pose.split(",")] if pose else None
    return name, script, pose


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--nav", action="append", default=[])
    ap.add_argument("--load", type=float, default=70.0)
    ap.add_argument("--countdown", type=float, default=22.0)
    ap.add_argument("--pilot-assist", choices=["off", "on", "leave"], default="off")
    ap.add_argument("--map", action="append", default=[], help="script=pad, e.g. l=l2")
    ap.add_argument("--settle", type=int, default=90, help="ticks with nothing held after a teleport")
    ap.add_argument("--run", action="append", default=[], help="name=script[@x,y,z,yaw[,speed]]")
    ap.add_argument("--repeat", type=int, default=1)
    ap.add_argument("--run-assist", action="append", default=[],
                    help="name=on|off: write the Pilot Assist flag before that run (it is read every frame)")
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    mapping = dict(DEFAULT_MAP)
    for item in args.map:
        k, _, v = item.partition("=")
        mapping[k] = v
    runs = [parse_run(r) for r in args.run]
    run_assist = dict(item.partition("=")[::2] for item in args.run_assist)
    plan = {}
    for item in args.nav:
        screen, _, buttons = item.partition("=")
        plan[screen] = [b.strip() for b in buttons.split(",") if b.strip()]
    with drive.Session(args.image, str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        (out / "rpcs3.pid").write_text("%d\n" % session.proc.pid)
        if not session.wait_for_screen_pressing("Main Menu", 240):
            sys.exit("never reached the Main Menu")
        session.settle_menu(20)
        gdb = Debugger(port=port)
        if args.pilot_assist != "leave":
            set_pilot_assist(gdb, args.pilot_assist == "on")
        session.screens_seen = set()
        session.shot_dir = out / "screens"
        session.shot_dir.mkdir(exist_ok=True)

        def navigate(plan, session=session):
            # `wait` sleeps 3 s: a screen's entrance animation eats the first
            # tap (rpcs3-debugger.md, "navigate()'s taps have no confirmation").
            screen = drive.current_screen()
            for n, button in enumerate(plan.get(screen, [])):
                if button == "wait":
                    time.sleep(3.0)
                elif button in ("cross", "circle"):
                    session.press_once(button)
                else:
                    session.tap(button, settle=1.5)
                drive.screenshot(session.shot_dir / ("nav-%s-%02d.png" % (
                    "".join(c if c.isalnum() else "-" for c in screen), n)), trim=True)
                print("    nav %-8s at %s" % (button, screen), flush=True)
            plan.pop(screen, None)

        session.navigate = navigate
        if session.walk_to_race(plan=plan) not in drive.RACE_ARRIVED:
            sys.exit("no race")
        track = drive.track_name()
        print("track: %s" % track, flush=True)
        session.wait_for_load(args.load)
        session.tap("cross", settle=args.countdown)
        drive.screenshot(out / "grid.png", trim=True)
        gdb.pause()
        ship, body = place.find_player(gdb)
        entry = struct.unpack(">I", gdb.read(ship + place.OFF_ENTRY, 4))[0]
        ships = struct.unpack(">8I", gdb.read(place.CRAFT_ARRAY, 32))
        rivals = []
        for other in ships:
            if other and other != ship:
                rivals.append(struct.unpack(">I", gdb.read(other + place.OFF_BODY, 4))[0])
        gdb.resume()
        guest = Guest(session.proc.pid, session.open_mem())
        if guest.read(CODE_CHECK[0], 4) != CODE_CHECK[1]:
            sys.exit("guest base is not 0x300000000 on this build")
        world = find_world(guest, body)
        inputs = guest.scan(struct.pack(">I", PLAYER_INPUT_VTABLE))
        # The craft's loaded handling blocks, as the game holds them after its
        # own load-time scaling: `craft+0x7c` (read by the airbrake update for
        # gain/falloff/amount/turn/drag at +0x44..+0x54), `+0x78` (the second
        # ramp's block, +0x7c/+0x80) and `+0x84` (the airbrake targets).
        blocks = {}
        for off in (0x74, 0x78, 0x7C, 0x80, 0x84, 0x88):
            ptr = guest.u32(entry + off)
            if 0x30000000 <= ptr < 0x40000000:
                blocks[off] = (ptr, guest.read(ptr, 0x200))
        pickle.dump(blocks, open(out / "handling-blocks.pkl", "wb"))
        meta = {"ship": ship, "body": body, "entry": entry, "world": world,
                "inputs": inputs, "rivals": rivals, "track": track, "map": mapping}
        print(meta, flush=True)
        (out / "meta.txt").write_text(repr(meta) + "\n")
        if not world:
            sys.exit("physics world not found")
        objs = {"world": (world, 0x50), "body": (body, 0x210), "entry": (entry, 0x600),
                "ship": (ship, 0x200)}
        if inputs:
            objs["input"] = (inputs[0], 0x200)
        for rep in range(args.repeat):
            for name, script, pose in runs:
                states = input_script.parse(Path(script).read_text())
                if name in run_assist:
                    set_pilot_assist(gdb, run_assist[name] == "on")
                if pose:
                    gdb.pause()
                    rows = None
                    x, y, z = pose[:3]
                    cur = place.read_pose(gdb, body)
                    yaw = math.radians(pose[3]) if len(pose) > 3 else None
                    if yaw is not None:
                        f = (math.sin(yaw), 0.0, math.cos(yaw))
                        rows = place.basis_rows(f, (0.0, 1.0, 0.0))
                    place.write_pose(gdb, body, (x, y, z), rows or cur["rows"],
                                     pose[4] if len(pose) > 4 else 0.0)
                    gdb.resume()
                raw = run_script(session, guest, objs, states, mapping, args.settle, rivals)
                stem = "%s-%d" % (name, rep)
                with open(out / (stem + ".csv"), "w", newline="") as fh:
                    w = csv.writer(fh)
                    w.writerow(HEADER)
                    prev_t = None
                    for game_t, wt, s, rival, pressed in raw:
                        dt = game_t - prev_t if prev_t is not None else 0.0
                        prev_t = game_t
                        w.writerow(row_of(game_t, dt, wt, s, {"rival": rival}, "input", pressed))
                pickle.dump(raw, open(out / (stem + ".raw.pkl"), "wb"))
                drive.screenshot(out / (stem + ".png"), trim=True)
                frames = (raw[-1][1] - raw[0][1] + 1) if raw else 0
                print("%s: %d rows over %d frame(s), %.3f s game time -> %s" % (
                    stem, len(raw), frames, raw[-1][0] if raw else 0.0, out / (stem + ".csv")), flush=True)
        gdb.close()


if __name__ == "__main__":
    main()
