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
            any(max(abs(a - b) for a, b in zip(c, ax)) < 1e-5 for ax in axes) for c in cached
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
