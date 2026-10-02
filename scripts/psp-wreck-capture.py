#!/usr/bin/env python3
"""Put a craft into Ship_SetState(entity, 4) on a running PPSSPP and photograph what follows.

The emulator half of the wreck comparison (`docs/ghidra/functions/psp-pulse-usa/ship-wreck-model.md`):
a craft in state 4 explodes for 0.5 s, state 5 swaps its wreck model in (the frame the death
sparks spawn), state 6 follows 1.5 s later. A real race reaches this only by draining a shield,
which is hard to time, so this calls the function itself.

How the call is made: at a `Ship_UpdateCraft` stop for the target craft the registers are saved,
`a0`/`a1`/`ra`/`pc` are pointed at `Ship_SetState(entity, 4)` with `ra` back at the same
breakpoint, the CPU runs until it returns there, and every register is restored. The craft
carries on as if its shield had just run out.

    python3 scripts/psp-wreck-capture.py --port 45682 --display :92 --out DIR \\
        [--restart --nearest --inject-frame 40]            # an opponent, during the countdown
    python3 scripts/psp-wreck-capture.py ... --spawns      # log every Psys_Spawn_q instead

`--restart` walks the pause menu's RESTART RACE first (the same walk
`scripts/psp-weapon-pair.py` uses), so the opponents are still on the grid and the chase camera
sees them. Without it the player's own craft is wrecked and the original's destroy camera
(`Camera_SetMode(5)`) takes over. Photographs are the emulator window at 480x272, the CPU
stopped, so `kNNN.png` shows the world about two frames before the stop frame.

Raw captures are derived game data: write them under `data/`, never commit them.
"""

import argparse
import base64
import importlib.util
import json
import struct
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger  # noqa: E402

_spec = importlib.util.spec_from_file_location(
    "pair", Path(__file__).resolve().parent / "psp-weapon-pair.py"
)
pair = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(pair)

SHIP_UPDATE_CRAFT = 0x08849618
SET_STATE = 0x08844100
PSYS_SPAWN_Q = 0x08915484
DRAW_EMITTER_POOL = 0x08918BF8
DRAW_PARTICLE = 0x089186BC
RACE_MANAGER = 0x08B317B4
CAMERA_OBJECT = 0x08B32C64
CAMERA_NODE_BASE = 0x08AB10B0
G_HUD = 0x08AB0838
HUD_WORDS = (0x2C, 0x3C, 0x40, 0x168, 0x274, 0x278)
CYCLES_PER_FRAME = 222_000_000 / 59.940059940059946


def regs(dbg):
    c = dbg.call("cpu.getAllRegs")["categories"][0]
    return dict(zip(c["registerNames"], c["uintValues"]))


def camera_row(dbg):
    """The camera controller's own fields (`camera.md`, "destroy camera") and the node it drives."""
    cam = dbg.read_u32(CAMERA_OBJECT)
    row = {"mode": dbg.read_u32(cam + 0x1DC), "subject": hex(dbg.read_u32(cam + 0x1E4))}
    f = lambda off, n=1: list(struct.unpack("<%df" % n, dbg.read(cam + off, 4 * n)))
    # The director's own state (`Camera_UpdateSpectator`): the ten second timer, the subject it
    # picks (`+0x1e0`, `subject` above is the one drawn, `+0x1e4`) and the 2026-10-02 additions
    # that tell a timer re-pick from a fixed delay.
    row["timer_3c"] = f(0x3C)[0]
    row["subject_1e0"] = hex(dbg.read_u32(cam + 0x1E0))
    row["flag_274"] = dbg.read(cam + 0x274, 1)[0]
    manager = dbg.read_u32(RACE_MANAGER)
    if pair.ram(manager):
        row["race_time"] = struct.unpack("<f", dbg.read(manager + 0x2B8, 4))[0]
    row["fov"], row["fov_target"], row["fov_rate"] = f(0x220)[0], f(0x224)[0], f(0x228)[0]
    row["focus"], row["focus_target"] = f(0x230, 3), f(0x240, 3)
    row["focus_rate"], row["frame_size"] = f(0x250)[0], f(0x268)[0]
    node = dbg.read_u32(cam + 0x1D4)
    row["node"] = hex(node)
    if node:
        row["node_floats_0x40_0xc0"] = list(struct.unpack("<32f", dbg.read(node + 0x40, 128)))
    node_base = dbg.read_u32(CAMERA_NODE_BASE)
    row["flags_10e8"] = hex(dbg.read_u32(dbg.read_u32(0x08AB10E8) + 0x2C))
    row["flags_10b0"] = hex(dbg.read_u32(node_base + 0x2C))
    row["view_node_0x40"] = list(struct.unpack("<16f", dbg.read(node_base + 0x40, 64)))
    return row


def camera_nodes(dbg):
    """Every authored `Camera` node the controller knows: eye (`+0x90`) and aim (`+0xa0`)."""
    cam = dbg.read_u32(CAMERA_OBJECT)
    out = []
    for i in range(dbg.read_u32(cam + 0x1D0)):
        node = dbg.read_u32(cam + 0x40 + 4 * i)
        f = struct.unpack("<8f", dbg.read(node + 0x90, 32))
        out.append({"node": hex(node), "eye": list(f[:3]), "aim": list(f[4:7])})
    return out


def position(dbg, entity):
    craft = dbg.read_u32(entity + 0x94)
    return struct.unpack("<3f", dbg.read(dbg.read_u32(craft + 0x1CC) + 0x30, 12))


def pick_entity(dbg, args):
    manager = dbg.read_u32(RACE_MANAGER)
    player = dbg.read_u32(manager + 0x2C0)
    if not args.nearest:
        return dbg.read_u32(manager + 0x78 + 4 * args.slot) if args.slot >= 0 else player
    origin = position(dbg, player)
    best = None
    for index in range(8):
        entity = dbg.read_u32(manager + 0x78 + 4 * index)
        if entity == player or not pair.ram(entity):
            continue
        p = position(dbg, entity)
        distance = sum((p[k] - origin[k]) ** 2 for k in range(3)) ** 0.5
        print("slot %d entity %#x at %.1f" % (index, entity, distance), file=sys.stderr)
        if best is None or distance < best[0]:
            best = (distance, entity)
    return best[1]


def inject(dbg, entity, state):
    """Call Ship_SetState(entity, state) from the current Ship_UpdateCraft stop."""
    saved = regs(dbg)
    for name, value in (("a0", entity), ("a1", state), ("ra", SHIP_UPDATE_CRAFT), ("pc", SET_STATE)):
        dbg.call("cpu.setReg", name=name, value=value)
    dbg.call("cpu.status")
    dbg.pending = []
    dbg.call("cpu.resume")
    dbg.wait_for_break(SHIP_UPDATE_CRAFT, timeout=30.0)
    for name, value in saved.items():
        if name != "zero":
            dbg.call("cpu.setReg", name=name, value=value)


def log_spawns(dbg, entity, seconds):
    start = dbg.call("cpu.status")["ticks"]
    log = []
    try:
        for _ in dbg.each_hit(PSYS_SPAWN_Q, 100000, timeout=20.0):
            now = (dbg.call("cpu.status")["ticks"] - start) / CYCLES_PER_FRAME
            if now > seconds * 60:
                break
            r = regs(dbg)
            name = dbg.read_cstring(r["a1"], 40) if pair.ram(r["a1"]) else None
            log.append(
                {
                    "frame": round(now, 1),
                    "name": name,
                    "parent": hex(dbg.read_u32(r["a0"] + 8)),
                    "state": dbg.read_u32(entity + 0x8C),
                }
            )
    except TimeoutError:
        pass
    return log


def read_pool(dbg, instance):
    """Every live particle of an emitter instance's pool (`particle-system.md`, "A pool particle")."""
    head = dbg.read(instance, 0x180)
    out = {
        "instance": instance,
        "resource": struct.unpack_from("<I", head, 0x20)[0],
        "scale_params": list(struct.unpack_from("<7f", head, 0x28)),
        "age_words": list(struct.unpack_from("<4I", head, 0x138)),
        "matrix": list(struct.unpack_from("<16f", head, 0xF0)),
        "words_0x20_0x70": list(struct.unpack_from("<20f", head, 0x20)),
        "particles": [],
    }
    pool = struct.unpack_from("<I", head, 0x74)[0]
    while pair.ram(pool):
        block = dbg.read(pool, 0x10 + 32 * 0xA0)
        mask = struct.unpack_from("<I", block, 0)[0]
        for slot in range(32):
            if not mask & (0x80000000 >> slot):
                continue
            base = 0x10 + slot * 0xA0
            out["particles"].append(
                {
                    "pos": struct.unpack_from("<3f", block, base + 0x40),
                    "size": struct.unpack_from("<f", block, base + 0x70)[0],
                    "rgba": list(block[base + 0x74 : base + 0x78]),
                    "frame": struct.unpack_from("<I", block, base + 0x78)[0],
                    "second": struct.unpack_from("<4f", block, base + 0x50),
                    "raw": block[base : base + 0xA0].hex(),
                }
            )
        pool = dbg.read_u32(pool + 0x1410)
    return out


def log_pools(dbg, from_frame, to_frame):
    """Every `ParticleSystem_DrawEmitterPool` call from `from_frame` to `to_frame` frames after now.

    Run at a stop. The frame is the PSP cycle counter over 222 MHz / 59.94, as `log_spawns` has it.
    The resource record's own words that say how it draws (`+0xb8` render mode, `+0xc0` blend
    class) are read once per resource.
    """
    start = dbg.call("cpu.status")["ticks"]
    log, resources = [], {}
    try:
        for _ in dbg.each_hit(DRAW_EMITTER_POOL, 1000000, timeout=20.0):
            now = (dbg.call("cpu.status")["ticks"] - start) / CYCLES_PER_FRAME
            if now > to_frame - from_frame:
                break
            r = regs(dbg)
            if not pair.ram(r["a0"]):
                continue
            row = read_pool(dbg, r["a0"])
            res = row["resource"]
            if pair.ram(res) and res not in resources:
                resources[res] = {
                    "mode": dbg.read_u32(res + 0xB8),
                    "blend": dbg.read_u32(res + 0xC0),
                    "texture_words": [dbg.read_u32(res + 0x890 + 4 * i) for i in range(4)],
                }
            row["frame"] = round(now + from_frame, 2)
            log.append(row)
    except TimeoutError:
        pass
    return {"draws": log, "resources": resources}


def log_templates(dbg, from_frame, to_frame):
    """Every `ParticleSystem_DrawParticle` call (a sprite template's draw) in the window.

    `a0` is the template particle itself: `+0x00` position, `+0x30` size, `+0x34` colour word,
    `+0x38` atlas frame, `+0x5c` roll, `+0x64` aspect (`particle-system.md`, "The stretch factor").
    The first 0x90 bytes are kept raw as floats and as one hex string.
    """
    start = dbg.call("cpu.status")["ticks"]
    log = []
    try:
        for _ in dbg.each_hit(DRAW_PARTICLE, 1000000, timeout=20.0):
            now = (dbg.call("cpu.status")["ticks"] - start) / CYCLES_PER_FRAME
            if now > to_frame - from_frame:
                break
            r = regs(dbg)
            if not pair.ram(r["a0"]):
                continue
            raw = dbg.read(r["a0"], 0x90)
            log.append(
                {
                    "frame": round(now + from_frame, 2),
                    "particle": r["a0"],
                    "floats": list(struct.unpack("<36f", raw)),
                    "colour": raw[0x34:0x38].hex(),
                    "hex": raw.hex(),
                }
            )
    except TimeoutError:
        pass
    return log


def log_hits(dbg, address, from_frame, to_frame):
    """Every hit of `address` in the window, with `a0`/`a1`, `f12` and 0xb0 bytes at each pointer.

    The generic probe: for `FUN_0885efc4` (the ship explosion's shockwave update, `a0` the
    object) it reads the object's age (`+0x88`), scale (`+0x8c`), alpha (`+0x98`) and matrix.
    """
    start = dbg.call("cpu.status")["ticks"]
    log = []
    try:
        for _ in dbg.each_hit(address, 1000000, timeout=20.0):
            now = (dbg.call("cpu.status")["ticks"] - start) / CYCLES_PER_FRAME
            if now > to_frame - from_frame:
                break
            r = regs(dbg)
            row = {"frame": round(now + from_frame, 2), "a0": r["a0"], "a1": r["a1"], "ra": r["ra"]}
            row["f12"] = struct.unpack("<f", struct.pack("<I", pair.fpu(dbg)["f12"]))[0]
            for name in ("a0", "a1"):
                if pair.ram(r[name]):
                    row[name + "_words"] = list(struct.unpack("<44I", dbg.read(r[name], 0xB0)))
            log.append(row)
    except TimeoutError:
        pass
    return log


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--port", type=int, required=True)
    ap.add_argument("--display", required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--slot", type=int, default=-1, help="manager grid slot to wreck (default the player)")
    ap.add_argument("--nearest", action="store_true", help="wreck the opponent nearest the player")
    ap.add_argument("--restart", action="store_true", help="RESTART RACE first; inject during the countdown")
    ap.add_argument("--inject-frame", type=int, default=5)
    ap.add_argument("--state", type=int, default=4)
    ap.add_argument("--shots", default="0,5,10,20,30,36,40,50,60,75,90,105,120,140,160,180")
    ap.add_argument("--spawns", action="store_true", help="log Psys_Spawn_q for 3.3 s instead of photographing")
    ap.add_argument("--place-window", action="store_true")
    ap.add_argument("--place", help="X,Y,Z: write the craft's body and node position there at the injection stop")
    ap.add_argument("--pools", help="K0:K1: after the call, log every emitter pool drawn from frame K0 to K1 (pools.json)")
    ap.add_argument("--templates", help="K0:K1: log every sprite template drawn from frame K0 to K1 (templates.json)")
    ap.add_argument("--ge-dump-k", type=int, help="K: ask for a GE dump (the next frame drawn) at frame K after the call -> ge.ppdmp")
    ap.add_argument("--edram", action="store_true", help="write both EDRAM framebuffers beside each shot (needs SoftwareRenderer = True)")
    ap.add_argument("--hits", help="ADDR:K0:K1: log every hit of a function (a0, a1, f12 and the objects they point at) -> hits.json")
    ap.add_argument("--camera", action="store_true", help="log the camera controller's fields each frame")
    ap.add_argument("--timeout", type=float, default=30.0, help="wall seconds to wait for each Ship_UpdateCraft stop (a software-rendered emulator needs more)")
    ap.add_argument("--ui-state", action="store_true",
                    help="log the front end's state name (`Race End Photo`, `EndRace Results`...) and the race manager's mode state each frame")
    ap.add_argument("--hud", action="store_true", help="log the HUD object's visibility words (`g_hud` +0x2c, +0x3c, +0x40, +0x120, +0x121) each frame")
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    shots = sorted({int(v) for v in args.shots.split(",")})
    if args.place_window:
        pair.place_window(args.display)

    dbg = Debugger(args.port)
    dump_reply = []
    dump_ticket = None
    log = []
    try:
        if args.restart:
            pair.restart_to_countdown(dbg, hold=False)
        dbg.brk()
        entity = pick_entity(dbg, args)
        craft = dbg.read_u32(entity + 0x94)
        print("entity %#x craft %#x" % (entity, craft), file=sys.stderr)
        injected = None
        frame = 0
        for _ in dbg.each_hit(SHIP_UPDATE_CRAFT, 100000, timeout=args.timeout):
            if regs(dbg)["a0"] != craft:
                continue
            if injected is None and frame >= args.inject_frame:
                if args.camera:
                    log.append({"nodes": camera_nodes(dbg), "entity_0xc3c": hex(dbg.read_u32(entity + 0xC3C))})
                if args.place:
                    where = [float(v) for v in args.place.split(",")]
                    dbg.write_f32s(dbg.read_u32(craft + 0x1CC) + 0x30, where)
                    dbg.write_f32s(dbg.read_u32(entity + 0x794) + 0x30, where)
                inject(dbg, entity, args.state)
                injected = frame
                print("Ship_SetState(%d) injected at frame %d" % (args.state, frame), file=sys.stderr)
                if args.spawns:
                    break
            row = {"frame": frame, "state": dbg.read_u32(entity + 0x8C)}
            live, hull, wreck = (dbg.read_u32(entity + o) for o in (0x8B0, 0x8B4, 0x8B8))
            row["live"] = "wreck" if live == wreck else "hull"
            row["hull_flags"] = dbg.read_u32(hull + 0x2C)
            row["wreck_flags"] = dbg.read_u32(wreck + 0x2C) if wreck else None
            row["timer"] = struct.unpack("<f", dbg.read(entity + 0x874, 4))[0]
            if args.camera:
                row["camera"] = camera_row(dbg)
            if args.ui_state:
                row["ui_state"] = dbg.state_name()
                manager = dbg.read_u32(RACE_MANAGER)
                if pair.ram(manager):
                    row["mode_state"] = dbg.read_u32(manager + 0x7C8)
            if args.hud:
                hud = dbg.read_u32(G_HUD)
                row["hud"] = hex(hud)
                if pair.ram(hud):
                    row["hud_words"] = {hex(o): hex(dbg.read_u32(hud + o)) for o in HUD_WORDS}
                    row["hud_bytes"] = {hex(o): dbg.read(hud + o, 1)[0] for o in (0x120, 0x121)}
            if injected is not None:
                k = frame - injected
                row["since"] = k
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
                    row["shot"] = name if pair.shoot(args.display, args.out / name) else None
                    if args.edram:
                        for index, base in enumerate((0x04000000, 0x04088000)):
                            (args.out / ("k%03d.fb%d.bin" % (k, index))).write_bytes(dbg.read(base, 0x88000))
                if k >= max(shots):
                    log.append(row)
                    break
            log.append(row)
            frame += 1
        if args.spawns:
            log = log_spawns(dbg, entity, 3.3)
        elif args.hits:
            address, k0, k1 = args.hits.split(":")
            log = log_hits(dbg, int(address, 16), int(k0), int(k1))
        elif args.templates:
            k0, k1 = (int(v) for v in args.templates.split(":"))
            log = log_templates(dbg, k0, k1)
        elif args.pools:
            k0, k1 = (int(v) for v in args.pools.split(":"))
            log = log_pools(dbg, k0, k1)
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
        (args.out / ("spawns.json" if args.spawns else "pools.json" if args.pools else "templates.json" if args.templates else "hits.json" if args.hits else "log.json")).write_text(json.dumps(log, indent=1))
        dbg.close()
    print("wrote", args.out)


if __name__ == "__main__":
    main()
