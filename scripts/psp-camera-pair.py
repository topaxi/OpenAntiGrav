#!/usr/bin/env python3
"""Read the original's live camera, projection fov and craft display matrix at a matched state.

The camera half of `psp-weapon-pair.py`: the same restart, the same held thrust
through the countdown, the same stop once a frame in `Weapons_DispatchFire`, and the
first frame the throttle word is non-zero is GO. Nothing is fired. For every frame in
`--from-go .. --to-go` after GO it logs, with the CPU stopped:

- the player camera node (`*(s7 + 0x3c)` at `Camera_UpdatePlayerView`, learned once),
  whose fourth row is the negated eye - `camera.md`;
- the rigid body's four rows (`craft+0x794`), the craft's velocity;
- `g_camera_fov_degrees` (`0x08b34310`) and both tan-half-fov globals;
- the three tripods' own fov words (`craft+0xa0/+0x200/+0x2b0` + `0x50`);
- the craft's drawn 4x4, `*(*(craft+0x8b0)+0x3c)+0x40`, whose row 3 reproduces the
  body position (the check `input-bindings.md` used to find it);
- the eye's offset in the body's own frame, so the view in use reads off as
  `(0, 3, 3)` internal, `-11.25 / +3` close, `-14.25 / +3` far.

A screenshot of the emulator window (480x272) is taken at each `--shots` frame after
GO, counted as the harness counts them. Run it with `--camera-only` for the pose log
alone.

    python3 scripts/psp-camera-pair.py --port 45682 --display :92 \\
        --out data/scratch/<lane>/orig-cam --place-window

Raw captures are derived game data: write them under `data/`, never commit them.
"""

import argparse
import importlib.util
import json
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from ppsspp_debugger import Debugger  # noqa: E402

_spec = importlib.util.spec_from_file_location("psp_weapon_pair", HERE / "psp-weapon-pair.py")
wp = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(wp)

G_CAMERA_FOV = 0x08B34310
TRIPODS = {"internal": 0x0A0, "close": 0x200, "far": 0x2B0}
DISPLAY_NODE = 0x8B0


def f32s(dbg, address, count):
    return list(struct.unpack("<%df" % count, dbg.read(address, 4 * count)))


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def read_row(dbg, craft, body, node):
    row = {}
    m = f32s(dbg, body, 16)
    row["body"] = m
    row["vel"] = f32s(dbg, body + 0x140, 3)
    row["fov"] = f32s(dbg, G_CAMERA_FOV, 3)
    row["tripod_fov"] = {k: f32s(dbg, craft + off + 0x50, 1)[0] for k, off in TRIPODS.items()}
    if node:
        c = f32s(dbg, node, 16)
        row["cam"] = c
        eye = [-c[12], -c[13], -c[14]]
        pos = m[12:15]
        d = [eye[i] - pos[i] for i in range(3)]
        row["eye"] = eye
        row["eye_in_body"] = [dot(d, m[0:3]), dot(d, m[4:7]), dot(d, m[8:11])]
        row["eye_dist"] = dot(d, d) ** 0.5
    holder = dbg.read_u32(craft + DISPLAY_NODE)
    if wp.ram(holder):
        inner = dbg.read_u32(holder + 0x3C)
        if wp.ram(inner):
            row["display"] = f32s(dbg, inner + 0x40, 16)
    return row


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--display", required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--from-go", type=int, default=0)
    parser.add_argument("--to-go", type=int, default=130)
    parser.add_argument("--shots", default="60,120", help="frames after GO to photograph")
    parser.add_argument("--no-restart", action="store_true")
    parser.add_argument("--place-window", action="store_true")
    parser.add_argument("--select", type=int, default=0,
                        help="press SELECT this many times at the countdown (cycles the view)")
    parser.add_argument("--timeout", type=float, default=60.0)
    parser.add_argument("--max-frames", type=int, default=900)
    args = parser.parse_args()

    args.out.mkdir(parents=True, exist_ok=True)
    shots = {int(v) for v in args.shots.split(",") if v}
    if args.place_window and not wp.place_window(args.display):
        raise SystemExit("no emulator window found on %s" % args.display)

    dbg = Debugger(args.port)
    log = {"args": {k: str(v) for k, v in vars(args).items()}, "frames": []}
    try:
        if not args.no_restart:
            wp.restart_to_countdown(dbg, hold=True)
        else:
            dbg.resume()
            dbg.hold(cross=True)
        for _ in range(args.select):
            dbg.hold(cross=True, select=True)
            import time
            time.sleep(0.2)
            dbg.hold(cross=True, select=False)
            time.sleep(0.3)
        node = wp.camera_node(dbg)
        print("camera node 0x%08x" % (node or 0), file=sys.stderr)
        go = None
        frame = 0
        for _, _ in dbg.each_hit(wp.WEAPONS_DISPATCH_FIRE, 6000, timeout=args.timeout):
            manager = dbg.read_u32(wp.RACE_MANAGER)
            if not wp.ram(manager):
                continue
            player = dbg.read_u32(manager + wp.PLAYER_OFFSET)
            if not wp.ram(player):
                continue
            craft = dbg.read_u32(player + wp.ENTITY_CRAFT)
            if not wp.ram(craft):
                continue
            body = dbg.read_u32(craft + wp.BODY_OFFSET)
            if not wp.ram(body):
                continue
            if frame > args.max_frames:
                print("gave up: no GO within %d frames" % frame, file=sys.stderr)
                break
            throttle = f32s(dbg, craft + wp.THROTTLE, 1)[0]
            if go is None and throttle > 0.0:
                go = frame
                print("GO at stop frame %d" % go, file=sys.stderr)
            if go is not None:
                since = frame - go
                if args.from_go <= since <= args.to_go or since in shots:
                    row = read_row(dbg, craft, body, node)
                    row["frame"] = frame
                    row["since_go"] = since
                    row["speed"] = f32s(dbg, craft + wp.SPEED_CACHED, 1)[0]
                    if since in shots:
                        name = "go%03d.png" % since
                        row["shot"] = name if wp.shoot(args.display, args.out / name) else None
                    log["frames"].append(row)
                if since > max(args.to_go, max(shots, default=0)):
                    break
            frame += 1
        dbg.resume()
    finally:
        dbg.hold(cross=False)
        (args.out / "log.json").write_text(json.dumps(log, indent=1))
        dbg.close()
    print("wrote %s" % args.out)


if __name__ == "__main__":
    main()
