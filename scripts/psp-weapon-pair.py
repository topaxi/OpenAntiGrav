#!/usr/bin/env python3
"""Fire one weapon from the player's craft at a chosen frame after GO, and photograph the frames after it.

This is the emulator half of a *matched-state* weapon comparison: the same
authored intent as `oag-game --race --input-script <FILE>.inputs --give <weapon>`,
so the two crafts are at the same speed when the weapon leaves. It exists because
`psp-fire-weapon.py --accelerate` times the fire by wall clock, which put the
original at 140-237 km/h and ours at 48-100 km/h in the first Rocket comparison
(handover thread, 2026-09-24) - two different states, not a rendering difference.

How the timing is made repeatable, without a shared frame counter:

1. The pause menu's RESTART RACE is walked to the track description and
   dismissed, then `cross` is held. Holding thrust through the countdown is not a
   false start (`docs/gameplay/race-modes.md#the-countdown-is-measured`): the
   throttle word just reads 0 until the gate lifts.
2. The script breaks in `Weapons_DispatchFire` once a frame. The first frame the
   player's throttle word (`craft+0x2b8`) is non-zero is **GO**, frame 0.
3. At frame `--go-offset` the weapon bit is ORed into the player's fire word
   inside the same breakpoint, so the handler runs in that very frame.
4. A screenshot is taken at each frame in `--shots`, counted from the fire. The
   CPU is stopped when it is taken; the presented frame lags the stop by about
   two frames (`fx-brightness` measured `det-001 == det-002`), so the PNG
   `fire+k` shows the world as of about `fire+k-2`. The file names carry the
   stop frame; the lag is the reader's to apply, once, to both sides' set.

Needs a window on a display where the emulator window sits at `160,88`
(`--place-window` moves it there; an unmanaged Xvfb puts it off-screen).

    python3 scripts/psp-weapon-pair.py --port 45682 --display :92 \\
        --out data/scratch/<lane>/rocket-orig rocket

Raw captures are derived game data: write them under `data/`, never commit them.
"""

import argparse
import json
import struct
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger  # noqa: E402

WEAPONS_DISPATCH_FIRE = 0x08861814
RACE_MANAGER = 0x08B317B4
# The race manager's player slot is the *ship entity* (`craft+0x1c4` points to it, its
# `+0x94` points back), not the craft `Ship_UpdateCraft` takes - measured 2026-10-01.
PLAYER_OFFSET = 0x2C0
ENTITY_CRAFT = 0x94
RECORD_OFFSET = 0x4C
BODY_OFFSET = 0x1CC
FIRE_WORD = 0x1B8
TARGET_WORD = 0x1BC
THROTTLE = 0x2B8
SPEED_CACHED = 0x2EC

# Fire-request bits, from `psp-fire-weapon.py`'s table and weapon-fire.md.
BITS = {
    "rocket": 0x0080,
    "shield": 0x0020,
    "missile": 0x0040,
    "mine": 0x0002,
    "bomb": 0x0100,
    "backward": 0x0100,
    "quake": 0x0008,
}

# `Psys_Spawn_q` (name in a1) and `Rocket_Update` (the rocket in a0): the two probes
# that say what a launch spawns and where each rocket goes, frame by frame.
PSYS_SPAWN = 0x08915484
ROCKET_UPDATE = 0x0885D2A8
CYCLES_PER_FRAME = 222_000_000 / 59.940059940059946
MINE_POSE_NODE = 0x08859CE4
BOMB_NODE = 0x08863390
CAMERA_BREAK = 0x0883C13C
PROBES = {"spawns": PSYS_SPAWN, "rocket": ROCKET_UPDATE, "mine": MINE_POSE_NODE}

# The emulator window is 960x544 (the PSP's 480x272, doubled) and is moved here.
WINDOW_X, WINDOW_Y, WINDOW_W, WINDOW_H = 160, 88, 960, 544


def ram(pointer):
    return 0x08800000 <= pointer < 0x0A000000


def gpr(dbg):
    registers = dbg.call("cpu.getAllRegs")
    category = next(c for c in registers["categories"] if c["name"] == "GPR")
    return dict(zip(category["registerNames"], category["uintValues"]))


def camera_node(dbg):
    """The player camera's node: one hit of `Camera_UpdatePlayerView`, `*(s7 + 0x3c)`.

    The node holds the view matrix with the **negated eye** in its fourth row
    (`camera.md`); `camera_eye` negates it back.
    """
    node = None
    for _, _ in dbg.each_hit(CAMERA_BREAK, 1, timeout=60.0):
        node = dbg.read_u32(gpr(dbg)["s7"] + 0x3C)
    return node


def camera_eye(dbg, node):
    m = struct.unpack("<16f", dbg.read(node, 0x40))
    return {"right": m[0:3], "up": m[4:7], "fwd": m[8:11], "eye": [-m[12], -m[13], -m[14]]}


def probe_after_fire(dbg, kind, frames, log):
    """Break at `kind`'s address for `frames` frames after the fire, logging each hit.

    Run inside the `Weapons_DispatchFire` stop that wrote the fire word. Only the
    most recently added breakpoint fires on v1.20.4, so the probe replaces the
    dispatch breakpoint rather than joining it. Frames are the PSP cycle counter
    over 222 MHz / 59.94, which is a wall clock of the emulated machine, not a
    hit count - the probe address is hit several times a frame.
    """
    address = PROBES[kind]
    start = dbg.call("cpu.status")["ticks"]
    out = log.setdefault(kind, [])
    try:
        _probe_loop(dbg, kind, address, frames, start, out)
    except TimeoutError:
        log[kind + "_quiet_after"] = out[-1]["frame"] if out else 0.0
        print("probe quiet: no further hit within 20 s of wall clock", file=sys.stderr)


def _probe_loop(dbg, kind, address, frames, start, out):
    for _, _ in dbg.each_hit(address, 100000, timeout=20.0):
        now = (dbg.call("cpu.status")["ticks"] - start) / CYCLES_PER_FRAME
        if now > frames:
            break
        regs = gpr(dbg)
        entry = {"frame": round(now, 2), "ra": regs["ra"]}
        if kind == "spawns":
            name = dbg.read_cstring(regs["a1"], 40) if ram(regs["a1"]) else None
            entry["name"] = name
            entry["regs"] = {k: regs[k] for k in ("a0", "a1", "a2", "a3", "t0", "t1", "t2", "t3")}
            entry["ptr_floats"] = {}
            for k in ("a0", "a2", "a3", "t0", "t1", "t2", "t3"):
                if ram(regs[k]):
                    entry["ptr_floats"][k] = list(struct.unpack("<16f", dbg.read(regs[k], 64)))
        elif kind == "mine":
            mine = regs["a0"]
            blob = dbg.read(mine, 0x100)
            entry["entity"] = mine
            entry["matrix"] = list(struct.unpack_from("<16f", blob, 0x60))
            entry["fuse"] = struct.unpack_from("<f", blob, 0x48)[0]
            entry["owner"] = struct.unpack_from("<I", blob, 0x40)[0]
        else:
            rocket = regs["a0"]
            entry["rocket"] = rocket
            blob = dbg.read(rocket, 0x140)
            entry["words"] = list(struct.unpack("<%dI" % (0x140 // 4), blob))
            entry["pos"] = list(struct.unpack_from("<3f", blob, 0x90))
            entry["vel"] = list(struct.unpack_from("<3f", blob, 0xE0))
            entry["prev"] = list(struct.unpack_from("<3f", blob, 0xF0))
            entry["normal"] = list(struct.unpack_from("<3f", blob, 0x100))
        out.append(entry)


def place_window(display):
    from Xlib import display as xdisplay

    d = xdisplay.Display(display)
    for child in d.screen().root.query_tree().children:
        name = child.get_wm_name()
        if name and "Pulse" in name:
            child.configure(x=WINDOW_X, y=WINDOW_Y)
            d.sync()
            return True
    return False


def shoot(display, path):
    """The emulator window as the PSP's own 480x272, nearest-neighbour halved."""
    argv = [
        "magick", "import", "-window", "root", "-crop",
        "%dx%d+%d+%d" % (WINDOW_W, WINDOW_H, WINDOW_X, WINDOW_Y), "+repage",
        "-filter", "point", "-resize", "50%", str(path),
    ]
    env = {"DISPLAY": display, "PATH": "/usr/bin:/bin"}
    return subprocess.run(argv, env=env, check=False).returncode == 0


def restart_to_countdown(dbg, hold):
    """Pause -> RESTART RACE -> dismiss the description, then (optionally) hold thrust."""
    dbg.resume()
    dbg.hold(cross=False)
    state = dbg.state_name()
    if "Pause" not in state:
        dbg.press("start", duration=6)
        time.sleep(1.5)
    for _ in range(4):
        dbg.press("down", duration=4)
        time.sleep(0.35)
    for _ in range(6):
        dbg.press("cross", duration=6)
        time.sleep(1.5)
        if "Pause" not in dbg.state_name():
            break
    else:
        raise SystemExit("RESTART RACE never took")
    # The track description is dismissed by its own confirm, which has to land
    # after the description is up and before the countdown ends.
    time.sleep(20.0)
    dbg.press("cross", duration=6)
    if hold:
        dbg.hold(cross=True)
    print("description dismissed; thrust %s" % ("held" if hold else "not held"), file=sys.stderr)


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("weapon", choices=sorted(BITS))
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--display", required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--go-offset", type=int, default=120,
                        help="fire this many frames after GO (the first non-zero throttle)")
    parser.add_argument("--shots", default="0,1,2,3,4,5,6,8,10,12,15,18,20,25,30,40",
                        help="frames after the fire to photograph")
    parser.add_argument("--frames", type=int, default=0,
                        help="log this many frames after the fire (default: through the last shot)")
    parser.add_argument("--no-restart", action="store_true",
                        help="the race is already at its countdown; just arm the hold")
    parser.add_argument("--no-hold", action="store_true", help="do not hold thrust (stand still)")
    parser.add_argument("--fire-frame", type=int,
                        help="fire at this absolute stop frame instead of --go-offset frames after "
                        "GO. Needed when thrust is not held, so there is no GO to see; GO came at "
                        "stop frame 266-267 on every run, so 300 is about 33 frames after it")
    parser.add_argument("--set-word", action="append", default=[], metavar="OFFSET=VALUE",
                        help="also write this word into the player's weapon record with the fire "
                        "bit, e.g. 0x1ac=5 for the Mine's round counter, which the pickup's arm "
                        "function sets and a hand-set fire bit does not")
    parser.add_argument("--place-window", action="store_true")
    parser.add_argument("--probe", choices=sorted(PROBES),
                        help="after the fire, instead of photographing, log every hit of the "
                        "probe address for --probe-frames frames")
    parser.add_argument("--probe-frames", type=float, default=150.0)
    parser.add_argument("--camera", action="store_true",
                        help="record the player camera's pose in every frame row")
    parser.add_argument("--no-fire", action="store_true",
                        help="a control: run the whole capture and write no fire bit")
    parser.add_argument("--timeout", type=float, default=60.0)
    parser.add_argument("--max-frames", type=int, default=900,
                        help="give up after this many frames if GO or the shots never come")
    args = parser.parse_args()

    args.out.mkdir(parents=True, exist_ok=True)
    shots = sorted({int(v) for v in args.shots.split(",") if v})
    last = max(max(shots), args.frames)

    if args.place_window and not place_window(args.display):
        raise SystemExit("no emulator window found on %s" % args.display)

    dbg = Debugger(args.port)
    log = {"args": {k: str(v) for k, v in vars(args).items()}, "frames": []}
    try:
        if not args.no_restart:
            restart_to_countdown(dbg, hold=not args.no_hold)
        elif not args.no_hold:
            dbg.resume()
            dbg.hold(cross=True)

        node = camera_node(dbg) if args.camera else None
        go = None
        fire_frame = None
        announced = None
        probe_pending = False
        frame = 0
        for _, _ in dbg.each_hit(WEAPONS_DISPATCH_FIRE, 6000, timeout=args.timeout):
            manager = dbg.read_u32(RACE_MANAGER)
            if not ram(manager):
                continue
            player = dbg.read_u32(manager + PLAYER_OFFSET)
            if not ram(player):
                continue
            rec = dbg.read_u32(player + RECORD_OFFSET)
            craft = dbg.read_u32(player + ENTITY_CRAFT)
            if not (ram(rec) and ram(craft)):
                continue
            body = dbg.read_u32(craft + BODY_OFFSET)
            if not ram(body):
                continue
            if announced != (player, rec, craft, body):
                announced = (player, rec, craft, body)
                print("entity 0x%08x record 0x%08x craft 0x%08x body 0x%08x" % announced,
                      file=sys.stderr)
            throttle = struct.unpack("<f", dbg.read(craft + THROTTLE, 4))[0]
            speed = struct.unpack("<f", dbg.read(craft + SPEED_CACHED, 4))[0]
            position = struct.unpack("<3f", dbg.read(body + 0x30, 12))
            if frame > args.max_frames:
                print("gave up: no GO or no fire within %d frames" % frame, file=sys.stderr)
                break
            row = {"frame": frame, "throttle": throttle, "speed": speed, "pos": position}
            if node:
                row["camera"] = camera_eye(dbg, node)
            if go is None and throttle > 0.0 and args.fire_frame is None:
                go = frame
                print("GO at stop frame %d" % go, file=sys.stderr)
            if go is not None or args.fire_frame is not None:
                if go is not None:
                    row["since_go"] = frame - go
                due = (frame >= args.fire_frame) if args.fire_frame is not None \
                    else (frame - go >= args.go_offset)
                if fire_frame is None and due:
                    fire_frame = frame
                    if args.no_fire:
                        print("control: no fire word written at stop frame %d" % frame,
                              file=sys.stderr)
                    else:
                        before = dbg.read_u32(rec + FIRE_WORD)
                        dbg.write_u32(rec + TARGET_WORD, 0xFFFFFFFF)
                        for item in args.set_word:
                            offset, value = item.split("=")
                            dbg.write_u32(rec + int(offset, 0), int(value, 0))
                        dbg.write_u32(rec + FIRE_WORD, before | BITS[args.weapon])
                        row["fired"] = True
                        row["fire_word"] = [before, dbg.read_u32(rec + FIRE_WORD)]
                        print("fired %s at stop frame %d (%s after GO), speed %.2f"
                              % (args.weapon, frame, frame - go if go is not None else "-", speed),
                              file=sys.stderr)
                    if args.probe:
                        # Not nested in this loop: the dispatch breakpoint is still
                        # armed inside it and would stop the probe's run at the next
                        # frame. Leave the loop (its exit removes the breakpoint with
                        # the CPU stopped, before the handler runs) and probe after.
                        probe_pending = True
                        log["frames"].append(row)
                        break
            if fire_frame is not None:
                k = frame - fire_frame
                row["since_fire"] = k
                if k in shots:
                    name = "k%03d.png" % k
                    row["shot"] = name if shoot(args.display, args.out / name) else None
                if k >= last:
                    log["frames"].append(row)
                    break
            log["frames"].append(row)
            frame += 1
        if probe_pending:
            probe_after_fire(dbg, args.probe, args.probe_frames, log)
        dbg.resume()
    finally:
        dbg.hold(cross=False)
        (args.out / "log.json").write_text(json.dumps(log, indent=1))
        dbg.close()
    print("wrote %s" % args.out)


if __name__ == "__main__":
    main()
