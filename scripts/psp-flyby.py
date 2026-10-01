#!/usr/bin/env python3
"""What the original does between a race's load and its countdown (the "pre-race flyby").

The maintainer's play report (2026-10-01): before a race the original flies the circuit. This
walks the front end into a race on a running Pulse PSP exactly as `psp-drive.py menu` does, but
stops at the ship confirmation and *watches the load* instead of sitting it out: every
`--interval` seconds it records the front-end state name, the PSP cycle counter, the camera
controller's mode word and a 480x272 screenshot, and it presses nothing until `--skip-at`.

    python3 scripts/psp-flyby.py --port 45681 --display :91 --out data/scratch/<lane>/survey-a \\
        [--single-race] [--track-down N] [--skip-at SECONDS]

Raw captures are derived game data: write them under `data/`, never commit them.
"""

import argparse
import importlib.util
import json
import struct
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from ppsspp_debugger import Debugger  # noqa: E402


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


drive = load("psp_drive", "psp-drive.py")
pair = load("psp_weapon_pair", "psp-weapon-pair.py")

CAMERA_OBJECT = 0x08B32C64
CYCLES_PER_FRAME = 222_000_000 / 59.940059940059946


def ticks(dbg):
    return dbg.call("cpu.status")["ticks"]


def camera_row(dbg):
    cam = dbg.read_u32(CAMERA_OBJECT)
    if not 0x08800000 <= cam < 0x0A000000:
        return None
    return {"mode": dbg.read_u32(cam + 0x1DC), "subject": dbg.read_u32(cam + 0x1E0)}


# `FUN_08882cbc`, the render-view publisher's body: the first write of the published view matrix
# (`g_render_view + 0x70`) in a frame. `s1` is the camera node, `*(s1 + 0x3c)` the tripod whose
# pose and field of view are copied into the render view. Its entry is not hit in the race, only
# while the track-description screen's camera runs - measured 2026-10-01.
VIEW_PUBLISH = 0x08882E9C
G_INGAME = 0x08AB0818
G_CAMERA_FOV = 0x08B34310
RACE_MANAGER = 0x08B317B4
G_HUD = 0x08AB0838
VIEW_BASE_POINTER = 0x08AB10B0


def gpr(dbg):
    c = dbg.call("cpu.getAllRegs")["categories"][0]
    return dict(zip(c["registerNames"], c["uintValues"]))


WEAPONS_DISPATCH_FIRE = 0x08861814


def after_flyby(dbg, args, t0, clock0):
    """Once the publisher stops: one stop per `Weapons_DispatchFire`, the race's own frame."""
    log = open(args.out / "after.jsonl", "w")
    for index, _ in dbg.each_hit(WEAPONS_DISPATCH_FIRE, args.after, timeout=30.0):
        manager = dbg.read_u32(RACE_MANAGER)
        row = {"i": index, "tick": round((ticks(dbg) - t0) / CYCLES_PER_FRAME, 2)}
        row["mode_state"], row["mode_sub"] = struct.unpack("<2I", dbg.read(manager + 0x7C8, 8))
        row["race_time"] = dbg.read_f32(manager + 0x2B8)
        player = dbg.read_u32(manager + 0x2C0)
        craft = dbg.read_u32(player + 0x94)
        row["throttle"] = dbg.read_f32(craft + 0x2B8)
        row["clock"] = dbg.read_f32(dbg.read_u32(G_INGAME) + 0x40)
        hud = dbg.read_u32(G_HUD)
        row["hud_hidden"] = dbg.read_u8(hud + 0x168)
        cam = dbg.read_u32(CAMERA_OBJECT)
        row["cam_mode"] = dbg.read_u32(cam + 0x1DC)
        if index % 5 == 0:
            row["state"] = dbg.state_name()
            pair.shoot(args.display, args.out / ("a%04d.png" % index)) if index % 30 == 0 else None
        log.write(json.dumps(row) + "\n")
        log.flush()
    log.close()


def per_frame(dbg, args):
    """One stop per published view: the tripod's pose and fov, the render view, the clocks."""
    pair.place_window(args.display)
    args.out.mkdir(parents=True, exist_ok=True)
    log = open(args.out / "frames.jsonl", "w")
    t0 = ticks(dbg)
    shots = set(range(0, args.frames, args.shot_every))
    skipped = False
    try:
        _frames(dbg, args, log, t0, shots)
    except TimeoutError:
        print("publisher stopped hitting", file=sys.stderr)
    log.close()
    if args.after:
        after_flyby(dbg, args, t0, None)
    raise SystemExit(0)


def _frames(dbg, args, log, t0, shots):
    skipped = False
    for index, _ in dbg.each_hit(args.break_at, args.frames, timeout=20.0):
        if args.skip_frame is not None and index >= args.skip_frame and not skipped:
            skipped = True
            dbg.call("input.buttons.press", button=args.skip_button, duration=4)
            print("pressed %s at hit %d" % (args.skip_button, index), file=sys.stderr)
        r = gpr(dbg)
        node = r["s1"]
        tripod = dbg.read_u32(node + 0x3C)
        raw = dbg.read(tripod + 0x40, 0xC0)
        f = list(struct.unpack("<48f", raw))
        base = dbg.read_u32(VIEW_BASE_POINTER)
        ingame = dbg.read_u32(G_INGAME)
        row = {
            "i": index,
            "tick": round((ticks(dbg) - t0) / CYCLES_PER_FRAME, 2),
            "node": hex(node), "tripod": hex(tripod),
            "fov": f[4], "pose": f[8:24], "tail": f[24:48],
            "view": list(struct.unpack("<16f", dbg.read(base + 0x40, 64))),
            "fov_global": dbg.read_f32(G_CAMERA_FOV),
            "clock": dbg.read_f32(ingame + 0x40),
            "ingame_0x3c": dbg.read_u32(ingame + 0x3C),
        }
        parent = dbg.read_u32(tripod + 8)
        row["cam_t"] = dbg.read_f32(tripod + 0x40)
        row["cam_delay"] = dbg.read_f32(tripod + 0xC8)
        if 0x08800000 <= parent < 0x0A000000:
            row["anim_t"] = dbg.read_f32(parent + 0x40)
            row["anim_end"] = dbg.read_f32(parent + 0x58)
            row["anim_paused"] = dbg.read_u8(parent + 0x84)
        manager = dbg.read_u32(RACE_MANAGER)
        if 0x08800000 <= manager < 0x0A000000:
            row["counter_1a04"] = dbg.read_u32(manager + 0x1A04)
            row["mode_state"], row["mode_sub"] = struct.unpack("<2I", dbg.read(manager + 0x7C8, 8))
            row["race_time"] = dbg.read_f32(manager + 0x2B8)
            player = dbg.read_u32(manager + 0x2C0)
            if 0x08800000 <= player < 0x0A000000:
                craft = dbg.read_u32(player + 0x94)
                body = dbg.read_u32(craft + 0x1CC)
                row["player_pos"] = list(struct.unpack("<3f", dbg.read(body + 0x30, 12)))
                row["player_throttle"] = dbg.read_f32(craft + 0x2B8)
        hud = dbg.read_u32(G_HUD)
        if 0x08800000 <= hud < 0x0A000000:
            row["hud_hidden"] = dbg.read_u8(hud + 0x168)
        if index % 10 == 0:
            row["state"] = dbg.state_name()
        log.write(json.dumps(row) + "\n")
        if index in shots:
            pair.shoot(args.display, args.out / ("f%04d.png" % index))


def restart_race(dbg):
    """Pause -> RESTART RACE (confirm pressed until the state leaves the pause menu)."""
    dbg.resume()
    dbg.hold(cross=False)
    if "Pause" not in (dbg.state_name() or ""):
        dbg.press("start", duration=6)
        time.sleep(1.5)
    for _ in range(4):
        dbg.press("down", duration=4)
        time.sleep(0.35)
    for _ in range(6):
        dbg.press("cross", duration=6)
        time.sleep(1.5)
        if "Pause" not in (dbg.state_name() or ""):
            return
    raise SystemExit("RESTART RACE never took")


def survey(args):
    def run(dbg, describe=True):
        if args.frames:
            per_frame(dbg, args)
        pair.place_window(args.display)
        args.out.mkdir(parents=True, exist_ok=True)
        log = open(args.out / "survey.jsonl", "w")
        start = time.time()
        t0 = ticks(dbg)
        index = 0
        skipped = False
        while time.time() - start < args.duration:
            wall = time.time() - start
            if args.skip_at is not None and not skipped and wall >= args.skip_at:
                dbg.press(args.skip_button, duration=6)
                skipped = True
                print("pressed %s at %.1f s" % (args.skip_button, wall), file=sys.stderr)
            row = {
                "wall": round(wall, 2),
                "frame": round((ticks(dbg) - t0) / CYCLES_PER_FRAME, 1),
                "state": dbg.state_name(),
                "camera": camera_row(dbg),
            }
            pair.shoot(args.display, args.out / ("s%03d.png" % index))
            log.write(json.dumps(row) + "\n")
            log.flush()
            print(row, file=sys.stderr)
            index += 1
            time.sleep(args.interval)
        raise SystemExit(0)

    drive.settle_into_race = run


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--display", required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--single-race", action="store_true")
    parser.add_argument("--track-down", type=int, default=0)
    parser.add_argument("--duration", type=float, default=60.0)
    parser.add_argument("--interval", type=float, default=1.0)
    parser.add_argument("--skip-at", type=float)
    parser.add_argument("--frames", type=int, default=0, help="per-frame mode: this many spectator updates")
    parser.add_argument("--shot-every", type=int, default=30)
    parser.add_argument("--restart", action="store_true", help="RESTART RACE from a live race instead of the menu walk")
    parser.add_argument("--break-at", type=lambda v: int(v, 0), default=VIEW_PUBLISH)
    parser.add_argument("--after", type=int, default=0, help="frames of Weapons_DispatchFire to log after the flyby")
    parser.add_argument("--skip-frame", type=int)
    parser.add_argument("--skip-button", default="cross")
    args = parser.parse_args()
    if args.restart:
        dbg = Debugger(args.port)
        restart_race(dbg)
        if args.frames:
            per_frame(dbg, args)
        args.skip_at = args.skip_at
        survey(args)
        drive.settle_into_race(dbg)
    survey(args)
    menu_args = argparse.Namespace(port=args.port, single_race=args.single_race,
                                   race_type=None, track_down=args.track_down, any_track=True)
    drive.menu(menu_args)


if __name__ == "__main__":
    main()
