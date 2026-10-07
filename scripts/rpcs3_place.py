"""Teleport the followed craft in a running Wipeout HD race on RPCS3.

The PS3 counterpart of `scripts/psp-drive.py place`. Every offset below is
measured, see "Teleporting the craft in HD" in
`docs/reverse-engineering/rpcs3-capture.md`.

The chain, all big-endian guest memory with no rebasing:

    0x0098d7c0            u32[8], pointers to the eight craft ("ships")
    ship+0x7a60           role word, 0 on the player (0xffffffff on the others)
    ship+0x6944           u32, the player's rigid body
    ship+0x5fac           u32, the craft-class entry; entry+0x270 == the body

and in the rigid body:

    +0x110 +0x120 +0x130  the 3x3 transpose, one 16-byte row each (w = 0)
    +0x190                linear velocity (xyz)
    +0x1a0 +0x1b0 +0x1c0  three more xyz vectors, their w lanes 1.0 (angular state)
    +0x1d0 +0x1e0 +0x1f0  basis rows: row0 = cross(row1, row2), row1 up,
                          row2 forward (w = 0)
    +0x200                position (w = 1)

Every world-space copy on the ship and the entry (hull points, probes, the
camera's follow target) is recomputed from the body on the next tick, which
the live run measured: after a body-only write, 11 of 14 position copies on the
ship and all 13 on the entry had followed.
"""

import math
import struct
import subprocess

CRAFT_ARRAY = 0x0098D7C0
OFF_ROLE = 0x7A60
OFF_BODY = 0x6944
OFF_ENTRY = 0x5FAC
ENTRY_BODY = 0x270
CRAFT_VTABLE = 0x008636E0

B_TRANSPOSE = 0x110
B_VELOCITY = 0x190
B_ANGULAR = (0x1A0, 0x1B0, 0x1C0)
B_ROWS = 0x1D0
B_POS = 0x200
BODY_SPAN = 0x210


def u32(blob, at=0):
    return struct.unpack_from(">I", blob, at)[0]


def parse_pose(text):
    """`x,y,z` or `x,y,z,yaw`; yaw in degrees off the track's own direction."""
    parts = [float(p) for p in text.split(",")]
    if len(parts) not in (3, 4):
        raise SystemExit("--pose takes x,y,z or x,y,z,yaw: %r" % text)
    return parts


def find_player(gdb):
    """`(ship, body)` of the craft whose role word is 0, target stopped."""
    ships = struct.unpack(">8I", gdb.read(CRAFT_ARRAY, 32))
    for ship in ships:
        if not ship:
            continue
        if u32(gdb.read(ship + OFF_ROLE, 4)) == 0:
            body = u32(gdb.read(ship + OFF_BODY, 4))
            entry = u32(gdb.read(ship + OFF_ENTRY, 4))
            if not body or u32(gdb.read(entry + ENTRY_BODY, 4)) != body:
                raise RuntimeError("ship %#x: entry+0x270 is not the body "
                                   "pointer, the chain has moved" % ship)
            return ship, body
    raise RuntimeError("no craft with role 0 in the array at %#x" % CRAFT_ARRAY)


def read_pose(gdb, body):
    """The body's position, basis rows and velocity as plain tuples."""
    blob = gdb.read(body + B_ROWS, BODY_SPAN - B_ROWS)
    rows = [struct.unpack_from(">3f", blob, i * 16) for i in range(3)]
    pos = struct.unpack_from(">3f", blob, B_POS - B_ROWS)
    vel = struct.unpack(">3f", gdb.read(body + B_VELOCITY, 12))
    return {"pos": pos, "rows": rows, "vel": vel}


def basis_rows(forward, up):
    """The recorded row order: row2 forward, row1 up re-orthogonalised."""
    f = _unit(forward)
    d = sum(a * b for a, b in zip(up, f))
    u = _unit(tuple(a - d * b for a, b in zip(up, f)))
    r = _cross(u, f)
    return [r, u, f]


def _unit(v):
    n = math.sqrt(sum(a * a for a in v))
    return tuple(a / n for a in v)


def _cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0])


def write_pose(gdb, body, pos, rows, speed=0.0):
    """Write one pose into the stopped target's rigid body.

    Rows, transpose, position, velocity along the new forward and zeroed xyz
    of the three angular vectors, whose w lanes stay as they are.
    """
    pad = (0.0,)
    blob = b"".join(struct.pack(">4f", *(r + pad)) for r in rows)
    gdb.write(body + B_ROWS, blob)
    gdb.write(body + B_POS, struct.pack(">4f", pos[0], pos[1], pos[2], 1.0))
    cols = [tuple(rows[r][c] for r in range(3)) for c in range(3)]
    gdb.write(body + B_TRANSPOSE,
              b"".join(struct.pack(">4f", *(c + pad)) for c in cols))
    f = rows[2]
    gdb.write(body + B_VELOCITY,
              struct.pack(">3f", f[0] * speed, f[1] * speed, f[2] * speed))
    for at in B_ANGULAR:
        gdb.write(body + at, struct.pack(">3f", 0.0, 0.0, 0.0))


def our_attitude(oag_game, image, track, pose, scratch):
    """Forward and up of `oag-game --pose` at `pose`, off its own trace row 0.

    Both sides then take their attitude from one rule (the nearest spline
    sample, plus yaw), instead of the script re-deriving a tangent.
    """
    import csv
    import os
    out = scratch / "attitude.csv"
    env = dict(os.environ)
    for key, sub in (("XDG_CONFIG_HOME", "cfg"), ("XDG_DATA_HOME", "data"),
                     ("XDG_STATE_HOME", "state")):
        env[key] = str(scratch / "profile" / sub)
    arg = ",".join("%g" % v for v in pose)
    result = subprocess.run(
        [str(oag_game), str(image), "--race", "--no-audio", "--track", track,
         "--pose=%s" % arg, "--ticks", "1", "--trace-out", str(out)],
        capture_output=True, text=True, env=env)
    if result.returncode != 0 or not out.exists():
        raise SystemExit("oag-game could not give an attitude: %s"
                         % result.stderr[-300:])
    with open(out) as handle:
        row = next(csv.DictReader(handle))
    return ([float(row["fwd_" + a]) for a in "xyz"],
            [float(row["up_" + a]) for a in "xyz"])
