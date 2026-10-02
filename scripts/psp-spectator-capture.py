#!/usr/bin/env python3
"""The spectator camera's modes, measured on a running Pulse PSP.

`Camera_UpdateSpectatorView` (`0x08880c04`) is one `switch` on the camera object's mode word
(`cam+0x1dc`), and every path leaves through one `jr ra` at `0x08882ae8`. This breaks there
once a frame while the post-finish director is running and logs, for that frame:

- the camera object's mode, subject (`+0x1e0`), previous subject (`+0x1e4`), node index
  (`+0x1e8`), timer (`+0x3c`) and the free-look angles (`+0x340`, `+0x344`);
- the matrix the view wrote, at `[*0x08ab10b0] + 0x40` (four rows of four floats);
- the drawn craft's matrix, at `*(entity + 0x794)` (the same four rows), its body matrix and
  position (`*(craft + 0x1cc)`) and the byte the view stores back into `cam+0x1e8`
  (`*(*(entity + 0xae4) + 0x60)`);
- the three globals case 3 scales its axes by, `0x08ab10c8/cc/d0`, once.

Because the breakpoint is on the view's own exit, the camera's output and the craft's matrix are
from the same instant. `--force 2:100:260,3:300:460` writes the mode word from stop frame 100
to 260 as mode 2 and from 300 to 460 as mode 3, so each mode is seen however the director's own
random rolls fall; the write lands at the exit and so takes effect on the next frame.

The race is finished the way `psp-postrace.py` does it (the Autopilot pickup, `--laps-hack 1`).

    uv run --with websocket-client scripts/psp-spectator-capture.py --port 45493 \\
        --display :93 --out data/scratch/<lane>/cap --laps-hack 1 --force 2:100:260,3:300:460

Raw captures are derived game data: write them under `data/`, never commit them.
"""

import argparse
import importlib.util
import json
import struct
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger  # noqa: E402


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).resolve().parent / file)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


pair = load("pair", "psp-weapon-pair.py")
post = load("post", "psp-postrace.py")

VIEW_EXIT = 0x08882AE8
VIEW_OUT_POINTER = 0x08AB10B0
CAM_GLOBALS = 0x08AB10C8


def f32s(dbg, address, count):
    return list(struct.unpack("<%df" % count, dbg.read(address, 4 * count)))


def frame_row(dbg, frame):
    cam = dbg.read_u32(post.CAMERA_OBJECT)
    words = struct.unpack("<8I", dbg.read(cam + 0x1D8, 32))
    row = {
        "frame": frame,
        "mode": dbg.read_u32(cam + 0x1DC),
        "subject": dbg.read_u32(cam + 0x1E0),
        "previous": dbg.read_u32(cam + 0x1E4),
        "node": struct.unpack("<i", dbg.read(cam + 0x1E8, 4))[0],
        "timer": f32s(dbg, cam + 0x3C, 1)[0],
        "pitch_340": f32s(dbg, cam + 0x340, 1)[0],
        "yaw_344": f32s(dbg, cam + 0x344, 1)[0],
        "width_268": f32s(dbg, cam + 0x268, 1)[0],
        "words_1d8": list(words),
    }
    out_base = dbg.read_u32(VIEW_OUT_POINTER)
    if post.ram(out_base):
        row["out"] = f32s(dbg, out_base + 0x40, 16)
    fov = dbg.read_u32(0x08B34310)
    row["fov_bits"] = fov
    row["fov"] = struct.unpack("<f", struct.pack("<I", fov))[0]
    subject = row["previous"] or row["subject"]
    if post.ram(subject):
        matrix = dbg.read_u32(subject + 0x794)
        if post.ram(matrix):
            row["matrix"] = f32s(dbg, matrix, 16)
        craft = dbg.read_u32(subject + post.ENTITY_CRAFT)
        if post.ram(craft):
            body = dbg.read_u32(craft + 0x1CC)
            if post.ram(body):
                row["body"] = f32s(dbg, body, 16)
        node_holder = dbg.read_u32(subject + 0xAE4)
        if post.ram(node_holder):
            row["node_byte"] = struct.unpack("<b", dbg.read(node_holder + 0x60, 1))[0]
        row["subject_entity"] = subject
    return row


def parse_force(text):
    spans = []
    for part in text.split(","):
        if part:
            mode, start, end = (int(v) for v in part.split(":"))
            spans.append((mode, start, end))
    return spans


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--display", required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--arm-after", type=int, default=60)
    parser.add_argument("--laps-hack", type=int)
    parser.add_argument("--frames", type=int, default=700, help="view frames to log")
    parser.add_argument("--force", default="", help="MODE:FROM:TO[,...] in view frames")
    parser.add_argument("--shots", default="", help="view frames to photograph, comma separated")
    parser.add_argument("--attach", action="store_true", help="the race is already past the line")
    parser.add_argument("--timeout", type=float, default=1200.0)
    parser.add_argument("--place-window", action="store_true")
    args = parser.parse_args()

    args.out.mkdir(parents=True, exist_ok=True)
    if args.place_window and not pair.place_window(args.display):
        raise SystemExit("no emulator window on %s" % args.display)
    spans = parse_force(args.force)
    shots = {int(v) for v in args.shots.split(",") if v}

    dbg = Debugger(args.port)
    log = {"args": {k: str(v) for k, v in vars(args).items()}, "frames": [], "events": []}
    try:
        if not args.attach:
            pair.restart_to_countdown(dbg, hold=True)
            go = None
            frame = 0
            record = None
            for _, _ in post.hits(dbg, post.WEAPONS_DISPATCH_FIRE, 3000, timeout=60.0):
                manager = dbg.read_u32(post.RACE_MANAGER)
                if not post.ram(manager):
                    continue
                player = dbg.read_u32(manager + post.PLAYER_OFFSET)
                if not post.ram(player):
                    continue
                craft = dbg.read_u32(player + post.ENTITY_CRAFT)
                record = dbg.read_u32(player + post.ENTITY_RECORD)
                if not (post.ram(craft) and post.ram(record)):
                    continue
                throttle = post.f32s(dbg, craft + 0x2B8, 1)[0]
                if go is None and throttle > 0.0:
                    go = frame
                if go is not None and frame - go >= args.arm_after:
                    if args.laps_hack:
                        dbg.write_u32(post.G_RACE_LAPS, args.laps_hack)
                    dbg.write_u32(record + post.HELD_WORD, 0xFFFFFFFF)
                    dbg.write_u32(record + post.FIRE_WORD,
                                  dbg.read_u32(record + post.FIRE_WORD) | post.AUTOPILOT_BIT)
                    break
                frame += 1
                if frame > 1500:
                    raise SystemExit("no GO within 1500 frames")
            for _, _ in post.hits(dbg, post.WEAPONS_DISPATCH_FIRE, 3, timeout=30.0):
                pass
            dbg.brk()
            dbg.write(record + post.AUTO_TIMER, struct.pack("<f", 1.0e9))
            dbg.resume()
            if not post.poll_final_lap(dbg, args.timeout):
                raise SystemExit("the player never reached the final lap")

        log["globals"] = f32s(dbg, CAM_GLOBALS, 3) if args.attach else None
        view_frame = 0
        manager = dbg.read_u32(post.RACE_MANAGER)
        player = dbg.read_u32(manager + post.PLAYER_OFFSET)
        finished = False
        # Wait out the finish with the cheap per-frame hook, then switch to the view's exit.
        if not args.attach:
            for _, _ in post.hits(dbg, post.WEAPONS_DISPATCH_FIRE, 400000, timeout=120.0):
                if dbg.read(player + 0x912, 1)[0]:
                    finished = True
                    break
            if not finished:
                raise SystemExit("no finish")
            print("finished", file=sys.stderr)
        dbg.brk()
        log["globals"] = f32s(dbg, CAM_GLOBALS, 3)
        gen = post.hits(dbg, VIEW_EXIT, args.frames + 10, timeout=120.0)
        for _, _ in gen:
            row = frame_row(dbg, view_frame)
            for mode, start, end in spans:
                if start <= view_frame < end:
                    cam = dbg.read_u32(post.CAMERA_OBJECT)
                    dbg.write_u32(cam + 0x1DC, mode)
                    row["forced"] = mode
            if view_frame in shots:
                name = "v%04d.png" % view_frame
                row["shot"] = name if pair.shoot(args.display, args.out / name) else None
            log["frames"].append(row)
            view_frame += 1
            if view_frame >= args.frames:
                break
        gen.close()
        dbg.resume()
    finally:
        dbg.hold(cross=False)
        (args.out / "log.json").write_text(json.dumps(log))
        dbg.close()
    print("wrote %s" % (args.out / "log.json"))


if __name__ == "__main__":
    main()
