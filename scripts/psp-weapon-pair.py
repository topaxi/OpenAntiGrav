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
import base64
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
    "repulser": 0x10000,
}

# `Psys_Spawn_q` (name in a1) and `Rocket_Update` (the rocket in a0): the two probes
# that say what a launch spawns and where each rocket goes, frame by frame.
PSYS_SPAWN = 0x08915484
ROCKET_UPDATE = 0x0885D2A8
# The instruction after `Rocket_Update`'s first `Collision_SweepSegment` call (the jal is at
# `0x0885d404`, its delay slot at `0x0885d408`): `v0` is the surface-probe result - `0x7f`
# nothing, `0`/`4` detonate, else a surface type to ride. Logged with the s registers, so the
# rocket (a pointer into RAM among them) can be matched to the entry rows by frame.
ROCKET_PROBE_RESULT = 0x0885D40C
CYCLES_PER_FRAME = 222_000_000 / 59.940059940059946
MINE_POSE_NODE = 0x08859CE4
BOMB_NODE = 0x08863390
# `g_ingame` (a pointer): its `+0x40` is the one animation clock every `Anim Transform`
# and texture transform reads (`anim-transform.md`).
G_INGAME = 0x08AB0818
INGAME_CLOCK = 0x40
CAMERA_BREAK = 0x0883C13C
BOMB_INIT = 0x08863188
SHIELD_UPDATE = 0x0885E254
# `Mesh_UpdateTextureTransforms` (mesh in a0): its time is `mesh+0x40`, the integral of the clock's
# delta since `Mesh_SetAnimTime` seeded it (`mesh+0x194` caches the clock). Hit for every animated
# mesh every frame, so the probe keeps only meshes whose time lags the race clock by 5 s or more -
# a model that was seeded at its own spawn rather than at session start.
MESH_TEXTURE_UPDATE = 0x0890E160
# `BombBlast_Update (float dt in f12, BombBlast *a0)`: the blast's age is `+0xd0`, its hemisphere
# and shockwave model nodes are `+0xd4` and `+0xd8`. The probe reads each node's `+0x40` (the
# texture time) and `+0x194` (the cached clock) beside the age, to pair an object age with the time.
BOMB_BLAST_UPDATE = 0x0887250C
# `FUN_089194d0` and `ParticleSystem_DrawRolledQuads` (instance in a0, view matrix in a1): the
# whole-instance draws of a pool-emitter's particles for render modes 0/1 (a plain square quad) and
# 2 (the rolled quad). Hit once per live instance per frame.
FLARE_DRAW = 0x089194D0
ROLLED_DRAW = 0x089178C0
PROBES = {
    "flare": FLARE_DRAW,
    "rolled": ROLLED_DRAW,
    "spawns": PSYS_SPAWN,
    "rocket": ROCKET_UPDATE,
    "sweep": ROCKET_PROBE_RESULT,
    "mine": MINE_POSE_NODE,
    "bomb": BOMB_INIT,
    "shield": SHIELD_UPDATE,
    "meshtex": MESH_TEXTURE_UPDATE,
    "blast": BOMB_BLAST_UPDATE,
}

# The emulator window is 960x544 (the PSP's 480x272, doubled) and is moved here.
WINDOW_X, WINDOW_Y, WINDOW_W, WINDOW_H = 160, 88, 960, 544


def ram(pointer):
    return 0x08800000 <= pointer < 0x0A000000


def gpr(dbg):
    registers = dbg.call("cpu.getAllRegs")
    category = next(c for c in registers["categories"] if c["name"] == "GPR")
    return dict(zip(category["registerNames"], category["uintValues"]))


def fpu(dbg):
    """The FPU registers as raw 32-bit words, by name (`f12` is a float argument)."""
    registers = dbg.call("cpu.getAllRegs")
    category = next(c for c in registers["categories"] if c["name"] == "FPU")
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


def probe_after_fire(dbg, kind, frames, log, detonate=None, condition=None):
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
        _probe_loop(dbg, kind, address, frames, start, out, detonate, condition)
    except TimeoutError:
        log[kind + "_quiet_after"] = out[-1]["frame"] if out else 0.0
        print("probe quiet: no further hit within 20 s of wall clock", file=sys.stderr)


def _probe_loop(dbg, kind, address, frames, start, out, detonate=None, condition=None):
    for _, _ in dbg.each_hit(address, 100000, timeout=20.0, condition=condition):
        now = (dbg.call("cpu.status")["ticks"] - start) / CYCLES_PER_FRAME
        if now > frames:
            break
        regs = gpr(dbg)
        entry = {"frame": round(now, 2), "ra": regs["ra"]}
        if detonate and not detonate["done"] and now >= detonate["at"]:
            detonate["done"] = True
            body = detonate["body"]
            here = struct.unpack("<3f", dbg.read(body + 0x30, 12))
            entry["detonation"] = detonate_bomb(dbg, body, here, detonate["previous"], detonate["ahead"])
        if kind == "spawns":
            name = dbg.read_cstring(regs["a1"], 40) if ram(regs["a1"]) else None
            entry["name"] = name
            entry["regs"] = {k: regs[k] for k in ("a0", "a1", "a2", "a3", "t0", "t1", "t2", "t3")}
            entry["ptr_floats"] = {}
            for k in ("a0", "a2", "a3", "t0", "t1", "t2", "t3"):
                if ram(regs[k]):
                    entry["ptr_floats"][k] = list(struct.unpack("<16f", dbg.read(regs[k], 64)))
        elif kind == "shield":
            # `ShipShield_Update (float dt in f12, ShipShield *a1)`: the whole shell animation.
            obj = regs["a0"] if ram(regs["a0"]) else regs["a1"]
            entry["dt"] = struct.unpack("<f", struct.pack("<I", fpu(dbg)["f12"]))[0]
            blob = dbg.read(obj, 0x90)
            entry["obj"] = obj
            entry["rgba"] = list(struct.unpack_from("<4f", blob, 0x40))
            entry["target"] = list(struct.unpack_from("<4f", blob, 0x50))
            entry["rate"], entry["swell"], entry["swell_target"], entry["swell_rate"] = \
                struct.unpack_from("<4f", blob, 0x60)
            entry["active"] = blob[0x74]
            entry["fading"] = blob[0x75]
            entry["time"] = struct.unpack_from("<f", blob, 0x78)[0]
            entry["models"] = {}
            for name, at in (("shell", 0x7C), ("cockpit", 0x80)):
                ptr = struct.unpack_from("<I", blob, at)[0]
                if ram(ptr):
                    entry["models"][name] = {"ptr": ptr, "flags": dbg.read_u32(ptr + 0x2C)}
        elif kind == "meshtex":
            mesh = regs["a0"]
            if not ram(mesh):
                continue
            ingame = dbg.read_u32(G_INGAME)
            clock = struct.unpack("<f", dbg.read(ingame + INGAME_CLOCK, 4))[0] if ram(ingame) else 0.0
            blob = dbg.read(mesh + 0x40, 4) + dbg.read(mesh + 0x18C, 12) + dbg.read(mesh + 0xC0, 1)
            time_now, shown, _, cached = struct.unpack_from("<4f", blob, 0)
            if clock - time_now < 5.0 and "detonation" not in entry:
                continue
            entry.update({"mesh": mesh, "time": time_now, "shown": shown, "cached_clock": cached,
                          "clock": clock, "paused": blob[16]})
        elif kind == "blast":
            blast = regs["a0"]
            if not ram(blast):
                continue
            ingame = dbg.read_u32(G_INGAME)
            entry["clock"] = struct.unpack("<f", dbg.read(ingame + INGAME_CLOCK, 4))[0]
            entry["blast"] = blast
            entry["age"] = struct.unpack("<f", dbg.read(blast + 0xD0, 4))[0]
            for name, at in (("hemisphere", 0xD4), ("shockwave", 0xD8)):
                node = dbg.read_u32(blast + at)
                if ram(node):
                    time_now = struct.unpack("<f", dbg.read(node + 0x40, 4))[0]
                    cached = struct.unpack("<f", dbg.read(node + 0x194, 4))[0]
                    entry[name] = {"node": node, "time": time_now, "cached_clock": cached}
        elif kind == "bomb":
            # `Bomb_Init (entity a0, position a1, direction a2, ...)`: the drop point.
            entry["entity"] = regs["a0"]
            entry["drop"] = list(struct.unpack("<4f", dbg.read(regs["a1"], 16)))
            entry["dir"] = list(struct.unpack("<4f", dbg.read(regs["a2"], 16)))
        elif kind == "mine":
            mine = regs["a0"]
            blob = dbg.read(mine, 0x100)
            entry["entity"] = mine
            entry["matrix"] = list(struct.unpack_from("<16f", blob, 0x60))
            entry["fuse"] = struct.unpack_from("<f", blob, 0x48)[0]
            entry["owner"] = struct.unpack_from("<I", blob, 0x40)[0]
        elif kind in ("flare", "rolled"):
            instance = regs["a0"]
            entry["instance"] = instance
            entry["view"] = list(struct.unpack("<16f", dbg.read(regs["a1"], 64)))
            head = dbg.read(instance, 0x180)
            entry["resource"] = struct.unpack_from("<I", head, 0x20)[0]
            entry["scale_params"] = list(struct.unpack_from("<7f", head, 0x28))
            entry["age_words"] = list(struct.unpack_from("<4I", head, 0x138))
            pool = struct.unpack_from("<I", head, 0x74)[0]
            entry["particles"] = []
            while ram(pool):
                block = dbg.read(pool, 0x10 + 32 * 0xA0)
                mask = struct.unpack_from("<I", block, 0)[0]
                for slot in range(32):
                    if not mask & (0x80000000 >> slot):
                        continue
                    base = 0x10 + slot * 0xA0
                    pos = struct.unpack_from("<4f", block, base + 0x40)
                    roll = struct.unpack_from("<f", block, base + 0x50)[0]
                    size = struct.unpack_from("<f", block, base + 0x70)[0]
                    rgba = list(block[base + 0x74:base + 0x78])
                    frame = struct.unpack_from("<I", block, base + 0x78)[0]
                    entry["particles"].append({"pos": pos, "roll": roll, "size": size,
                                               "rgba": rgba, "frame": frame})
                pool = dbg.read_u32(pool + 0x1410)
        elif kind == "sweep":
            entry["v0"] = regs["v0"]
            entry["s"] = [regs["s%d" % i] for i in range(8)]
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


BOMB_POOL = 0x08B3BF90
BOMB_SLOTS, BOMB_COUNT, BOMB_POSITION, BOMB_AGE = 0x44, 0xC4, 0xB0, 0xC0


def detonate_bomb(dbg, body, position, previous, ahead):
    """Move the first laid Bomb `ahead` units down the craft's travel and run its fuse out.

    `Bomb_Detonate` reads the position at `bomb+0xb0` (`FUN_088633c0`) and `Bomb_AdvanceFuse`
    returns `age (+0xc0) < timetodie`, so a huge age is a detonation on the next pool update,
    wherever the bomb is. Needs the CPU stopped.
    """
    pool = dbg.read_u32(BOMB_POOL)
    if not ram(pool):
        return {"error": "no bomb pool", "pool": pool}
    count = dbg.read_u32(pool + BOMB_COUNT)
    bomb = dbg.read_u32(pool + BOMB_SLOTS)
    if not count or not ram(bomb):
        return {"error": "no laid bomb", "count": count, "bomb": bomb}
    here = struct.unpack("<4f", dbg.read(bomb + BOMB_POSITION, 16))
    step = [a - b for a, b in zip(position, previous or position)]
    length = sum(c * c for c in step) ** 0.5 or 1.0
    target = [position[i] + step[i] / length * ahead for i in range(3)]
    dbg.write(bomb + BOMB_POSITION, struct.pack("<4f", target[0], target[1], target[2], here[3]))
    dbg.write_u32(bomb + BOMB_AGE, struct.unpack("<I", struct.pack("<f", 1000.0))[0])
    return {"bomb": bomb, "count": count, "was": here, "now": target}


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
    parser.add_argument("--detonate-bomb-at", type=int, metavar="K",
                        help="at stop frame fire+K, move the first laid Bomb to --detonate-ahead units "
                        "ahead of the craft along its travel and set its age past its fuse, so "
                        "`BombPool_Update` detonates it there: the owner only trips its own charge "
                        "when stationary and at the craft, which puts the camera inside the blast")
    parser.add_argument("--detonate-ahead", type=float, default=120.0,
                        help="how far ahead of the craft, in units, --detonate-bomb-at puts the blast")
    parser.add_argument("--ge-dump-k", type=int, metavar="K",
                        help="ask for a GE dump (`gpu.record.dump`, a .ppdmp of the next frame the GPU "
                        "draws) at stop frame fire+K, written to OUT/ge.ppdmp. The request is sent "
                        "while the CPU is stopped and answered after it resumes - a synchronous "
                        "request would wait for a frame that cannot draw")
    parser.add_argument("--edram", action="store_true",
                        help="also write both EDRAM framebuffers (0x04000000 and 0x04088000, 480x272 "
                        "at stride 512, RGBA8888, alpha = the bloom's glow mask) beside each shot. "
                        "Needs the emulator on the SOFTWARE renderer; the OpenGL backend leaves EDRAM zero")
    parser.add_argument("--probe", choices=sorted(PROBES),
                        help="after the fire, instead of photographing, log every hit of the "
                        "probe address for --probe-frames frames")
    parser.add_argument("--probe-frames", type=float, default=150.0)
    parser.add_argument("--probe-condition", help="a PPSSPP breakpoint condition on the probe, e.g. "
                        "'a0 >= 0x90af000 && a0 < 0x90b1000' to keep one heap block's hits only")
    parser.add_argument("--probe-after-detonation", action="store_true",
                        help="with --detonate-bomb-at: detonate from the dispatch loop, then start "
                        "the probe, so a probe address only the blast reaches is armed in time")
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
        dump_ticket = None
        dump_reply = []
        last_position = previous_position = None
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
            previous_position, last_position = last_position, position
            ingame = dbg.read_u32(G_INGAME)
            if ram(ingame):
                row["clock"] = struct.unpack("<f", dbg.read(ingame + INGAME_CLOCK, 4))[0]
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
                    if args.probe and args.detonate_bomb_at is None:
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
                if args.detonate_bomb_at is not None and k == args.detonate_bomb_at:
                    row["detonation"] = detonate_bomb(dbg, body, position, previous_position,
                                                      args.detonate_ahead)
                    if args.probe and args.probe_after_detonation:
                        probe_pending = True
                        log["frames"].append(row)
                        break
                if args.ge_dump_k is not None and k == args.ge_dump_k:
                    dump_ticket = dbg.send("gpu.record.dump")
                    _receive = dbg._recv

                    def _spy(_receive=_receive, ticket=dump_ticket):
                        message = _receive()
                        if message and message.get("ticket") == ticket:
                            dump_reply.append(message)
                        return message

                    dbg._recv = _spy
                if k in shots:
                    name = "k%03d.png" % k
                    row["shot"] = name if shoot(args.display, args.out / name) else None
                    if args.edram:
                        for index, base in enumerate((0x04000000, 0x04088000)):
                            (args.out / ("k%03d.fb%d.bin" % (k, index))).write_bytes(
                                dbg.read(base, 0x88000))
                if k >= last:
                    log["frames"].append(row)
                    break
            log["frames"].append(row)
            frame += 1
        if probe_pending:
            detonate = None
            if args.detonate_bomb_at is not None and not args.probe_after_detonation:
                # `previous` is where the craft was a frame before the fire: the direction of
                # travel the blast is placed along; `at` is in frames after the fire.
                detonate = {"at": float(args.detonate_bomb_at), "ahead": args.detonate_ahead,
                            "body": body, "previous": previous_position, "done": False}
                # `detonate_bomb` steps from `previous` to the position it is given, so hand it a
                # `previous` that is the fire frame's own position.
                detonate["previous"] = last_position if previous_position is None else previous_position
            probe_after_fire(dbg, args.probe, args.probe_frames, log, detonate, args.probe_condition)
        dbg.resume()
        if dump_ticket is not None:
            end = time.time() + 120
            while not dump_reply and time.time() < end:
                dbg._recv()
            if dump_reply:
                _, b64 = dump_reply[0]["uri"].split(",", 1)
                (args.out / "ge.ppdmp").write_bytes(base64.b64decode(b64))
                print("wrote %s" % (args.out / "ge.ppdmp"), file=sys.stderr)
            else:
                print("no GE dump arrived", file=sys.stderr)
    finally:
        dbg.hold(cross=False)
        (args.out / "log.json").write_text(json.dumps(log, indent=1))
        dbg.close()
    print("wrote %s" % args.out)


if __name__ == "__main__":
    main()
