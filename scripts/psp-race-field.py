#!/usr/bin/env python3
"""Wipeout Pulse (PSP): the whole field out of RAM, and a hunt for a mid-pack moment.

Two jobs, both on the pose the original puts every craft in at the player's own
`Ship_UpdateCraft` stop (the update order is the player first, then slots 6..0, so at that
stop all eight bodies are on the same tick boundary):

    hunt   fly the player with the original's own Autopilot (`psp-autopilot-rail.py`'s `arm`),
           stop at the first tick whose field matches the criteria, leave the CPU stopped there
    read   read the field, the camera and the HUD race state now (used by `psp-pulse-pair.py
           --field`, which owns the state load and the screenshot)

    uv run --with websocket-client python3 scripts/psp-race-field.py hunt --port 9317 --place 3-6

The field is a list of rows, one per racer in slot order: the body's left/up/forward/position
(`body+0x00`, the convention `psp-trace.py` writes), the racer entity's race position (`entity+0xbd4`, 1-based),
lap (`entity+0xacc`, the lap number plus one) and lap time (`entity+0x920`), and the racer's team index, so the craft
renderer in ours can be told what each one is. Derived game data: keep it under `data/`.
"""

import argparse
import importlib.util
import json
import math
import struct
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from ppsspp_debugger import Debugger  # noqa: E402
from psp_trace_fields import (  # noqa: E402
    CAMERA_FIELDS,
    CAMERA_NODE_OFFSET,
    CAMERA_UPDATE_BREAK,
    SHIP_UPDATE_CRAFT,
)

_spec = importlib.util.spec_from_file_location("rail", HERE / "psp-autopilot-rail.py")
rail = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(rail)

RACE_MANAGER = 0x08B317B4
GRID_SLOTS = 0x78
ENTITY_CRAFT = 0x94
ENTITY_BODY = 0x794
PLACE = 0xBD4
LAP = 0xACC
RACE_TIME = 0x920
RACER_COUNT = 0x08B35FA0
WEAPONS_ENABLED = 0x08B30FA8
SKILL_LEVEL = 0x08B31044
#: `Ship_SetShield`: the pool `entity+0x88` is clamped to `*(*(craft+0x6c) + g_skill_level*4 + 0x84)` (shield.md).
SHIELD = 0x88


#: `g_named_registry` (race-progress.md): 256 pointers from +0, the entry count at +0xc00. `PLAYER_HUD` is one of them, the
#: block `PlayerStatus_Update` refreshes: the speed readout in km/h at +0x00, the lap time (`craft+0x920`) at +0x24.
NAMED_REGISTRY = 0x08B317B8
HUD_SPEED = 0x00
HUD_LAP_TIME = 0x24


def ram(pointer):
    return 0x08800000 <= pointer < 0x0A000000


class Field:
    """Pointers of every racer, learnt once per race (they are heap objects that do not move)."""

    def __init__(self, dbg):
        self.dbg = dbg
        manager = dbg.read_u32(RACE_MANAGER)
        self.player_entity = dbg.read_u32(manager + rail.PLAYER_OFFSET)
        self.slots = []
        for i in range(8):
            entity = dbg.read_u32(manager + GRID_SLOTS + 4 * i)
            if not ram(entity):
                continue
            craft = dbg.read_u32(entity + ENTITY_CRAFT)
            body = dbg.read_u32(entity + ENTITY_BODY)
            self.slots.append({"slot": i, "entity": entity, "craft": craft, "body": body,
                               "player": entity == self.player_entity})
        self.player = next(s for s in self.slots if s["player"])

    def team_name(self, s):
        """`[[entity+0x370]+0x74]`, the racer's team-name string (grid.md, "Reproducing")."""
        if "team" not in s:
            try:
                block = self.dbg.read_u32(s["entity"] + 0x370)
                text = self.dbg.read_u32(block + 0x74)
                raw = self.dbg.read(text, 32) if ram(text) else b""
                s["team"] = raw.split(b"\0")[0].decode("ascii", "replace")
            except Exception as error:  # noqa: BLE001 - recorded, not hidden
                s["team"] = "?%s" % error
        return s["team"]

    def hud_kmh(self, lap_time):
        """The speed the HUD reads, `PLAYER_HUD+0x00`: the registry's block whose lap time (+0x24) is the player's own
        `entity+0x920` (a bit-exact float match; no name hash is needed). `None` when no block matches. Read at the same
        stop as the pose, so a pair imposes the value the original's own readout holds rather than `|velocity| * 3.6`
        of a body read a tick or two off the frame's (2026-10-21, `rerender-all`)."""
        if "hud_block" not in self.player:
            self.player["hud_block"] = None
            pointers = struct.unpack("<256I", self.dbg.read(NAMED_REGISTRY, 0x400))
            for ptr in pointers:
                if lap_time > 0.0 and ram(ptr) and struct.unpack("<f", self.dbg.read(ptr + HUD_LAP_TIME, 4))[0] == lap_time:
                    self.player["hud_block"] = ptr
                    break
        block = self.player["hud_block"]
        return None if block is None else struct.unpack("<f", self.dbg.read(block + HUD_SPEED, 4))[0]

    def shield_max(self, s):
        """The pool's maximum for this craft and the race's skill level, the same formula the game clamps with."""
        if "shield_max" not in s:
            stats = self.dbg.read_u32(s["craft"] + 0x6C)
            skill = self.dbg.read_u32(SKILL_LEVEL)
            s["shield_max"] = rail.f32s(self.dbg, stats + skill * 4 + 0x84, 1)[0]
        return s["shield_max"]

    def rows(self):
        out = []
        for s in self.slots:
            pose = struct.unpack("<16f", self.dbg.read(s["body"], 64))
            craft_words = self.dbg.read(s["entity"] + LAP, 4)
            out.append({
                "slot": s["slot"], "player": s["player"], "team": self.team_name(s),
                "left": list(pose[0:3]), "up": list(pose[4:7]), "fwd": list(pose[8:11]),
                "pos": list(pose[12:15]),
                #: `Body` linear velocity, units per second (`psp_trace_fields.BODY_FIELDS`, `vel_x` at +0x140)
                "vel": rail.f32s(self.dbg, s["body"] + 0x140, 3),
                "place": self.dbg.read_u32(s["entity"] + PLACE),
                "lap": struct.unpack("<I", craft_words)[0],
                "best_lap_cs": self.dbg.read_u32(s["entity"] + 0x930),
                "shield": rail.f32s(self.dbg, s["entity"] + SHIELD, 1)[0],
                "shield_max": self.shield_max(s),
                "lap_time": rail.f32s(self.dbg, s["entity"] + RACE_TIME, 1)[0],
            })
            if s["player"]:
                out[-1]["hud_kmh"] = self.hud_kmh(out[-1]["lap_time"])
        return out


def camera(dbg, node):
    blob = dbg.read(node, 0x40)
    cam = {name: struct.unpack("<f", blob[at:at + 4])[0] for name, at in CAMERA_FIELDS}
    for a in "xyz":
        cam["cam_pos_" + a] = -cam["cam_pos_" + a]
    return cam


def learn_camera_node(dbg):
    node = None
    for _, _ in dbg.each_hit(CAMERA_UPDATE_BREAK, 1, timeout=60):
        regs = dbg.call("cpu.getAllRegs")
        gpr = next(c for c in regs["categories"] if c["name"] == "GPR")
        s7 = dict(zip(gpr["registerNames"], gpr["uintValues"]))["s7"]
        node = dbg.read_u32(s7 + CAMERA_NODE_OFFSET)
    return node


def look_of(cam):
    """The camera's look direction: the node stores its rotation transposed, so the look is the NEGATED third
    COLUMN of the recorded rows (`oag_trace::replay::camera_orientation_of`), not the `cam_fwd` row."""
    return [-cam["cam_right_z"], -cam["cam_up_z"], -cam["cam_fwd_z"]]


def in_view(cam, pos, max_dist, half_fov_deg):
    eye = [cam["cam_pos_" + a] for a in "xyz"]
    fwd = look_of(cam)
    d = [p - e for p, e in zip(pos, eye)]
    dist = math.sqrt(sum(x * x for x in d))
    if dist < 6 or dist > max_dist:
        return False
    cos = sum(a * b for a, b in zip(d, fwd)) / dist
    return cos >= math.cos(math.radians(half_fov_deg))


def parse_range(text):
    lo, _, hi = text.partition("-")
    return int(lo), int(hi or lo)


def matches(rows, cam, args):
    player = next(r for r in rows if r["player"])
    lo, hi = args.place
    if not lo <= player["place"] + args.place_base <= hi:
        return None
    seen = [r for r in rows if not r["player"] and in_view(cam, r["pos"], args.max_dist, args.half_fov)]
    near = [r for r in seen if math.dist(r["pos"], player["pos"]) <= args.near_dist]
    if len(seen) < args.min_in_view or len(near) < args.min_near:
        return None
    return {"in_view": [r["slot"] for r in seen], "near": [r["slot"] for r in near]}


def save_state(dbg, args):
    """Save a state a few frames after the stop: the pause menu cannot be opened while the debugger
    holds the CPU (its `paused()` reads the stop as the menu), so the CPU runs on and Escape follows at
    once. The race moves a few ticks; the pair script reads the field again after loading, so the
    criteria are re-checked on the state itself and nothing here is trusted from the stop."""
    import os
    import shutil
    import subprocess
    st_spec = importlib.util.spec_from_file_location("psp_state", HERE / "psp-state.py")
    st = importlib.util.module_from_spec(st_spec)
    st_spec.loader.exec_module(st)
    dbg.resume()
    subprocess.run(["xdotool", "key", "Escape"], check=True, env=os.environ)
    time.sleep(1.0)
    path = st.do_save(dbg, args.home, args.slot)
    Path(args.save_state).parent.mkdir(parents=True, exist_ok=True)
    if Path(args.save_state).exists():
        raise SystemExit("%s exists: new files only" % args.save_state)
    shutil.copy(path, args.save_state)
    print("saved state kept at %s" % args.save_state, file=sys.stderr)
    return str(args.save_state)


def hunt(args):
    dbg = Debugger(args.port)
    log = {"events": [], "frames": []}
    dbg.resume()
    dbg.hold(cross=True)
    time.sleep(0.5)
    node = learn_camera_node(dbg)
    field = Field(dbg)
    print("camera node %#x, %d racers, player slot %d" % (node, len(field.slots), field.player["slot"]),
          file=sys.stderr)
    wp = dbg.read(WEAPONS_ENABLED, 1)[0]
    print("g_weapons_enabled byte = %d" % wp, file=sys.stderr)
    p = rail.arm(dbg, argparse.Namespace(stint=args.stint, steer=None, arm_after=args.arm_after), log)
    dbg.brk()
    dbg.add_breakpoint(SHIP_UPDATE_CRAFT, "a0 == 0x%x" % field.player["craft"])
    start = time.time()
    found = None
    try:
        for n in range(args.frames):
            if time.time() - start > args.budget:
                print("budget spent after %d frames" % n, file=sys.stderr)
                break
            dbg.call("cpu.status")
            dbg.pending = []
            dbg.call("cpu.resume")
            try:
                dbg.wait_for_break(SHIP_UPDATE_CRAFT, timeout=30.0)
            except TimeoutError:
                st = dbg.call("cpu.status")
                if not (st["stepping"] and st["pc"] == SHIP_UPDATE_CRAFT):
                    raise
            # one read decides whether this frame can match at all: the player's place
            place_now = dbg.read_u32(field.player["entity"] + PLACE)
            if not args.place[0] <= place_now + args.place_base <= args.place[1] and n % args.every:
                continue
            rows = field.rows()
            cam = camera(dbg, node)
            player = next(r for r in rows if r["player"])
            if n % args.every == 0:
                print("frame %d place %d lap %d t=%.1f others-in-view %s" % (
                    n, player["place"], player["lap"], player["lap_time"],
                    [r["slot"] for r in rows if not r["player"] and in_view(cam, r["pos"], args.max_dist, args.half_fov)]),
                    file=sys.stderr)
            hit = matches(rows, cam, args)
            if hit and n >= args.min_frame:
                found = {"frame": n, "match": hit, "rows": rows, "camera": cam}
                break
    finally:
        dbg.remove_breakpoint(SHIP_UPDATE_CRAFT)
        if found is None:
            dbg.resume()
    saved = None
    if found and args.save_state:
        saved = save_state(dbg, args)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({"found": found, "weapons_byte": wp, "saved_state": saved}))
    if found:
        print("FOUND frame %d: %s; CPU left stopped at the player's Ship_UpdateCraft" % (found["frame"], found["match"]),
              file=sys.stderr)
        return 0
    return 1


def read_now(args):
    dbg = Debugger(args.port)
    field = Field(dbg)
    node = learn_camera_node(dbg) if args.camera_node is None else args.camera_node
    out = {"rows": field.rows(), "camera": camera(dbg, node), "camera_node": node,
           "weapons_byte": dbg.read(WEAPONS_ENABLED, 1)[0]}
    args.out.write_text(json.dumps(out))
    return 0


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    h = sub.add_parser("hunt")
    h.add_argument("--port", type=int, required=True)
    h.add_argument("--out", type=Path, required=True)
    h.add_argument("--place", type=parse_range, default=(3, 6), help="player's 1-based place, a-b")
    h.add_argument("--place-base", type=int, default=0, help="added to the RAM value (the RAM value is already 1-based)")
    h.add_argument("--min-in-view", type=int, default=2)
    h.add_argument("--max-dist", type=float, default=90.0, help="a craft counts as in view up to this far from the camera")
    h.add_argument("--near-dist", type=float, default=45.0, help="a craft in view this close to the player is `near`")
    h.add_argument("--min-near", type=int, default=1, help="at least this many in-view craft within --near-dist")
    h.add_argument("--half-fov", type=float, default=22.0)
    h.add_argument("--min-frame", type=int, default=0)
    h.add_argument("--stint", type=float)
    h.add_argument("--arm-after", type=int, default=60)
    h.add_argument("--frames", type=int, default=20000)
    h.add_argument("--budget", type=float, default=540.0)
    h.add_argument("--every", type=int, default=120)
    h.add_argument("--save-state", type=Path, help="keep a state saved right after the stop here (new file only)")
    h.add_argument("--home", help="the instance's HOME (PP_HOME), for --save-state")
    h.add_argument("--slot", type=int, default=3, help="the pause-menu slot --save-state writes through")
    r = sub.add_parser("read")
    r.add_argument("--port", type=int, required=True)
    r.add_argument("--out", type=Path, required=True)
    r.add_argument("--camera-node", type=lambda v: int(v, 0))
    args = ap.parse_args()
    sys.exit(hunt(args) if args.cmd == "hunt" else read_now(args))


if __name__ == "__main__":
    main()
