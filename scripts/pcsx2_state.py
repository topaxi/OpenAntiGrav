#!/usr/bin/env python3
"""Read a Pulse PS2 PCSX2 savestate offline: find the player's craft, record its circuit.

    python3 scripts/pcsx2_state.py find STATE.p2s [--circuit "03_Track reversed"] [--write]

A `.p2s` is a zip whose `eeMemory.bin` is the 32 MiB of EE RAM, so the craft
pointer `scripts/pcsx2-trace.py` needs can be read out of the file with no
emulator running, and it is fixed for as long as that state is what every capture
starts from (PCSX2's `loadstate` is bit-exact). `--write` stores the answer, with
the circuit and the craft's start position, in `STATE.p2s.json` beside the state,
which `pcsx2-trace.py --state-file STATE.p2s` reads in place of `--craft`: a state
carries no circuit of its own, so this sidecar is where it is recorded.

How the player's craft is told from the other seven: a craft is a struct whose
`+0x1ec` word points at a 16-aligned body in EE RAM and whose cached basis at
`+0x180`, `+0x190` and `+0x1a0` is the body's own right/up/forward vectors in some
order (`pcsx2_trace_fields.py`, the method of 2026-09-16). The player's is the one
whose camera rig exists: some struct `R` with `*(R+0x824) == body` and the rig's
far and close eyes (`R+0x850`, `R+0x860`) 8 to 20 units from the body's position
(`pcsx2-camera-eye.py`). Both tests are on the state's memory alone.
"""

import argparse
import json
import math
import struct
import sys
import zipfile
from pathlib import Path

RAM_LO = 0x100000
BODY_POINTER = 0x1EC
CAMERA_BODY = 0x824
CAMERA_EYES = (0x850, 0x860)
#: The craft's cached basis (`+0x180`...) trails the body's by a tick: 2e-4 on a craft left resting for three minutes (a resting
#: craft keeps turning, `docs/physics/grid-state.md`), under 1e-5 on one saved a few seconds after GO.
CACHE_TOLERANCE = 5e-3
#: The race clock is not at one address: `0x70EE9C` in the Moa Therma White state, `0x70DB7C` in a later boot's. It sits 0x12084 to
#: 0x120a4 below the player's craft struct on all 16 states of `docs/tools/compare-page.md`, "Pulse PS2: a race pair for every circuit",
#: and 0x100 above it is the clock the HUD prints (the RAM clock less 4.1 to 4.25 s).
CLOCK_WINDOW = (0x13000, 0x11000)
CLOCK_PARTNER = 0x100


def _ram(path):
    return zipfile.ZipFile(path).read("eeMemory.bin")


def _vec(ram, address):
    return struct.unpack_from("<3f", ram, address)


def _plausible_body(ram, body):
    if body & 15 or not RAM_LO <= body < len(ram) - 0x40:
        return None
    pos = _vec(ram, body + 0x30)
    if not all(math.isfinite(x) and abs(x) < 1e5 for x in pos):
        return None
    # a table row that happens to hold a body-shaped pointer and a position like (16.6, 0, 0) is not a craft on a circuit
    if sum(1 for x in pos if x == 0.0) >= 2:
        return None
    return pos


def crafts(path):
    """Every craft-shaped struct in the state: `[(craft, body, position)]`."""
    ram = _ram(path)
    found = []
    for craft in range(RAM_LO, len(ram) - 0x200, 4):
        (body,) = struct.unpack_from("<I", ram, craft + BODY_POINTER)
        pos = _plausible_body(ram, body)
        if pos is None:
            continue
        axes = [_vec(ram, body + o) for o in (0x00, 0x10, 0x20)]
        cached = [_vec(ram, craft + o) for o in (0x180, 0x190, 0x1A0)]
        if not all(all(math.isfinite(x) for x in v) for v in axes + cached):
            continue
        if all(abs(sum(x * x for x in v) - 1.0) < 1e-3 for v in axes) and all(
            any(max(abs(a - b) for a, b in zip(c, ax)) < CACHE_TOLERANCE for ax in axes) for c in cached
        ):
            found.append((craft, body, pos))
    return found


def player_craft(path):
    """`(craft, body, position)` of the player's craft, or `None`."""
    ram = _ram(path)
    candidates = {body: (craft, body, pos) for craft, body, pos in crafts(path)}
    for rig in range(RAM_LO, len(ram) - 0x870, 4):
        (body,) = struct.unpack_from("<I", ram, rig + CAMERA_BODY)
        if body not in candidates:
            continue
        pos = candidates[body][2]
        dists = [math.dist(pos, _vec(ram, rig + o)) for o in CAMERA_EYES]
        if all(8.0 <= d <= 20.0 for d in dists):
            return candidates[body]
    return None


def race_clock_address(path, craft):
    """The EE address of the race clock (f32 seconds from the start of the countdown), or `None` when it is not the only word in
    the window below `craft` that is 3.9 to 4.45 s above the word `CLOCK_PARTNER` bytes after it (the HUD's lap clock), the HUD clock
    being at least a second (before GO it reads 0, and the countdown's own timers sit 4 s above a zero)."""
    ram = _ram(path)
    found = []
    for a in range(craft - CLOCK_WINDOW[0], craft - CLOCK_WINDOW[1], 4):
        (v,) = struct.unpack_from("<f", ram, a)
        (w,) = struct.unpack_from("<f", ram, a + CLOCK_PARTNER)
        if math.isfinite(v) and math.isfinite(w) and 1.0 <= w and v < 3600.0 and 3.9 <= v - w <= 4.45:
            found.append(a)
    return found[0] if len(found) == 1 else None


FRAME_COUNTER = 0x27A7E8


def race_clock_between(path_before, path_after):
    """The EE address of the race clock found from two savestates of one running race, or `None` unless exactly one word qualifies: a
    float that rose by `Delta frames * 0.015 .. 0.025` s (the frame counter at `FRAME_COUNTER` says how many frames lay between the
    states), whose partner `CLOCK_PARTNER` bytes after it is 3.9 to 4.45 s below it in both states and at least a second. Needs no
    craft, so it does not depend on `player_craft` finding the right one."""
    from array import array
    a_ram, b_ram = _ram(path_before), _ram(path_after)
    frames = struct.unpack_from("<I", b_ram, FRAME_COUNTER)[0] - struct.unpack_from("<I", a_ram, FRAME_COUNTER)[0]
    if frames <= 0:
        return None
    fa, fb = array("f"), array("f")
    fa.frombytes(a_ram[: len(a_ram) // 4 * 4])
    fb.frombytes(b_ram[: len(b_ram) // 4 * 4])
    step = CLOCK_PARTNER // 4
    lo, hi = frames * 0.015, frames * 0.025
    found = []
    for i in range(RAM_LO // 4, len(fa) - step):
        v = fa[i]
        if not 4.0 < v < 3600.0:
            continue
        d = fb[i] - v
        if not lo < d < hi:
            continue
        wa, wb = fa[i + step], fb[i + step]
        if wa >= 1.0 and 3.9 <= v - wa <= 4.45 and 3.9 <= fb[i] - wb <= 4.45:
            found.append(i * 4)
    return found[0] if len(found) == 1 else None


def sidecar_race_clock(path, craft):
    """The race clock's address for a state: the one its sidecar recorded when it still reads as the clock in the state itself (a float
    in 1..3600 with a partner 3.9 to 4.45 s below it), else the window search below the craft."""
    side = read_sidecar(path)
    if side and side.get("race_clock_address"):
        address = int(side["race_clock_address"], 0)
        ram = _ram(path)
        v = struct.unpack_from("<f", ram, address)[0]
        w = struct.unpack_from("<f", ram, address + CLOCK_PARTNER)[0]
        if 4.0 < v < 3600.0 and w >= 1.0 and 3.9 <= v - w <= 4.45:
            return address
    return race_clock_address(path, craft)


#: `g_ingame` (`docs/ghidra/functions/ps2-pulse-eu/anim-transform.md`): a pointer to the session object, whose `+0x40` is the one clock every
#: `Anim Transform` node and the world meshes' texture animation (the neon strips) run on. Static data, the same address in every state.
G_INGAME = 0x27DDBC
INGAME_CLOCK = 0x40


def world_clock(path):
    """The world animation clock (f32 seconds, `*(g_ingame + 0x40)`) in a state, or `None` when `g_ingame` is not a pointer into RAM."""
    ram = _ram(path)
    (session,) = struct.unpack_from("<I", ram, G_INGAME)
    if not RAM_LO <= session < len(ram) - 0x80:
        return None
    (seconds,) = struct.unpack_from("<f", ram, session + INGAME_CLOCK)
    return seconds if math.isfinite(seconds) and 0.0 <= seconds < 36000.0 else None


def ours_world_clock_offset(world_s, state_clock_s, grab_clock_s, ticks):
    """The `--world-clock-offset` that puts ours' mesh clock (`ticks / 60` plus the offset) on the original's at the grab: the world clock of the
    state, run on by the race clock's advance from the state to the grab (the two run at one rate), less ours' own `ticks / 60`."""
    return world_s + (grab_clock_s - state_clock_s) - ticks / 60


def record_world_clock_offset(rec, root):
    """The `--world-clock-offset` a Pulse PS2 grid pair's record implies, from its state file (under `root`) and the grab bracket it kept,
    or `None` when the record names no readable state or no bracket (a pair made before the pair script recorded the world clock)."""
    orig = rec.get("original") or {}
    live = orig.get("race_clock_live_s") or {}
    state = (orig.get("state") or {}).get("file")
    ticks = (rec.get("ours") or {}).get("ticks")
    if not state or ticks is None or not {"before_grab", "after_grab", "state_file"} <= live.keys() or not (Path(root) / state).is_file():
        return None
    world = world_clock(Path(root) / state)
    if world is None:
        return None
    return ours_world_clock_offset(world, live["state_file"], (live["before_grab"] + live["after_grab"]) / 2, ticks)


#: Offsets inside the camera object `Camera_SubmitScene` runs on (measured live, `docs/ghidra/functions/ps2-pulse-eu/camera.md`):
#: the camera's world matrix (rows right, up, backward, then the eye) at `+0x40`, its twin at `+0x90`, the perspective
#: matrix at `+0xd0` and the vertical field in degrees at `+0x110`.
CAMERA_WORLD = 0x40
CAMERA_PROJECTION = 0xD0
CAMERA_FOV = 0x110


def camera(path, craft_position, max_eye_distance=25.0):
    """The race camera of a state, found by signature: `{address, eye, right, up, back, fov_deg, aspect}`, or `None`.

    A camera object is the 16-aligned address whose field of view at `+0x110` agrees with the perspective matrix's
    `m11 = 1 / tan(fov / 2)`, whose world matrix at `+0x40` is orthonormal, whose twin at `+0x90` equals it and whose
    eye is within `max_eye_distance` of the player's craft. `aspect` is `m11 / m00`, the projection's own: 10/7 with
    the game's `Aspect Ratio` option on `4:3`, 10/7 * 4/3 on `16:9`."""
    ram = _ram(path)
    for a in range(RAM_LO, len(ram) - 0x120, 16):
        (fov,) = struct.unpack_from("<f", ram, a + CAMERA_FOV)
        if not (math.isfinite(fov) and 20.0 < fov < 120.0):
            continue
        m00, m11 = struct.unpack_from("<f", ram, a + CAMERA_PROJECTION)[0], struct.unpack_from("<f", ram, a + CAMERA_PROJECTION + 0x14)[0]
        if not (math.isfinite(m11) and m00 > 0 and abs(m11 * math.tan(math.radians(fov) / 2) - 1.0) < 1e-3):
            continue
        rows = [_vec(ram, a + CAMERA_WORLD + 16 * i) for i in range(4)]
        twin = [_vec(ram, a + 0x90 + 16 * i) for i in range(4)]
        if rows != twin or not all(all(math.isfinite(x) for x in r) for r in rows):
            continue
        if not all(abs(sum(x * x for x in r) - 1.0) < 1e-3 for r in rows[:3]):
            continue
        if math.dist(rows[3], craft_position) > max_eye_distance:
            continue
        return {"address": a, "eye": rows[3], "right": rows[0], "up": rows[1], "back": rows[2], "fov_deg": fov,
                "aspect": m11 / m00}
    return None


def sidecar(path):
    return Path(str(path) + ".json")


def read_sidecar(path):
    """The recorded `{craft, circuit, ...}` for a state, or `None`."""
    side = sidecar(path)
    return json.loads(side.read_text()) if side.exists() else None


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("command", choices=["find"])
    ap.add_argument("state", type=Path)
    ap.add_argument("--circuit", help='what the state is, e.g. "03_Track reversed" (Moa Therma Black)')
    ap.add_argument("--write", action="store_true", help="store the answer in STATE.p2s.json")
    args = ap.parse_args()
    found = player_craft(args.state)
    if found is None:
        sys.exit("no craft with a camera rig in %s" % args.state)
    craft, body, pos = found
    print("craft 0x%x body 0x%x position (%.2f, %.2f, %.2f)" % (craft, body, *pos))
    if args.write:
        record = {"craft": "0x%x" % craft, "body": "0x%x" % body,
                  "position": [round(x, 3) for x in pos]}
        if args.circuit:
            record["circuit"] = args.circuit
        sidecar(args.state).write_text(json.dumps(record, indent=2) + "\n")
        print("wrote", sidecar(args.state))


if __name__ == "__main__":
    main()
