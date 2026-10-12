#!/usr/bin/env python3
"""HD's engine light on the player's own hull, read and injected live on RPCS3.

Attach to a running `rpcs3-drive.py serve` started with `--interpreter` (the only
PPU decoder where a `Z0` breakpoint fires) and a GDB stub (`OAG_RPCS3_GDB`).

  rpcs3-hd-hull-light.py team
      The player's team directory and `EngineLightData` Distance/Radius
      (`ship+0xfc`/`+0x100`).
  rpcs3-hd-hull-light.py flare [N] [--json OUT]
      The flare node `EngineFlare_SubmitSpuLight` reads (break `0x0029ff8c`,
      `r11` = node world matrix, `r7+0xfc` = Distance/Radius), then N frames of
      the player's staged light at the `SVC1` branch (`0x003eb368`, `r3` list,
      `r31` count, `r19` model record) mapped into hull model space through the
      main-hull `EdgeGeom` job matrix (`0x005d8488`, `r11`).
  rpcs3-hd-hull-light.py inject OUT.png R G B D [--off dx dy dz] [--abs]
                                [--frames K] [--none] [--pair]
      Overwrite the player's light in the staged list for K consecutive frames
      and grab the screen while stopped, so the presented frame carries it.
  rpcs3-hd-hull-light.py boost OUT_DIR [--pose x,y,z] [--speed U] [--shots N]
                               [--skip K] [--ab K] [--min-colour C]
                               [--oag-game PATH]
      Native boost frames, nothing written to the light: put the player's
      body at `--pose` (attitude off this project's spline through
      `oag-game --trace-out`, as `rpcs3-drive.py place` does) moving at
      `--speed` along it, then shoot every frame whose own staged light has a
      channel at or above `--min-colour` (a boosted light: `base * (1 + 10 *
      blend)`), and write each frame's pose and light beside it.

Addresses and method: docs/ghidra/functions/ps3-hdfury-eu/renderer.md, the
`hd-hull-wash` sections (2026-10-12).
"""
import argparse
import importlib.util
import json
import os
import struct
import sys
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import rpcs3_place as place  # noqa: E402
from rpcs3_debugger import Debugger, REG_GPR  # noqa: E402

_spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)

SVC1_TAKEN = 0x003EB368
FLARE_SUBMIT = 0x0029FF8C
EDGEGEOM_MATRIX = 0x005D8488
MODEL_RECORDS = 0x008B7DA0


def gpr(regs, i):
    return int.from_bytes(regs[REG_GPR + i * 8:REG_GPR + i * 8 + 8], "big") & 0xFFFFFFFF


def u32(gdb, a):
    return struct.unpack(">I", gdb.read(a, 4))[0]


def stop_at(gdb, addr, timeout=20.0):
    gdb.resume()
    if gdb.wait_for_stop(timeout) is None:
        gdb.pause()
        gdb.drain()
        return None, None
    gdb.drain()
    for tid in gdb.threads():
        regs = gdb.registers(tid)
        if regs is not None and int.from_bytes(regs[512:520], "big") == addr:
            return tid, regs
    return None, None


def port():
    return int(os.environ.get("OAG_RPCS3_GDB", "127.0.0.1:2345").rsplit(":", 1)[1])


def player(gdb):
    ship, body = place.find_player(gdb)
    team = gdb.read(u32(gdb, u32(gdb, ship + 0x6298) + 0x1B4), 64).split(b"\0")[0].decode()
    dist, rad = struct.unpack(">2f", gdb.read(ship + 0xFC, 8))
    hull_model = u32(gdb, ship + 0x6AE0)
    hull_rec = u32(gdb, MODEL_RECORDS) + u32(gdb, hull_model + 0x204C) * 0x1B0
    return ship, body, team, dist, rad, hull_rec


def staged(gdb, regs):
    slot, count = gpr(regs, 3), gpr(regs, 31)
    return slot, [list(struct.unpack(">8f", gdb.read(slot + 32 * k, 32))) for k in range(count)]


def nearest(recs, p):
    return min(range(len(recs)), key=lambda i: sum((recs[i][j] - p[j]) ** 2 for j in range(3)))


def cmd_team(gdb, _args):
    _, _, team, dist, rad, rec = player(gdb)
    print("team %s Distance %.3f Radius %.3f hull record 0x%08x" % (team, dist, rad, rec))


def cmd_flare(gdb, args):
    _, body, team, dist, rad, hull_rec = player(gdb)
    print("team", team, "Distance", dist, "Radius", rad)
    gdb.add_breakpoint(FLARE_SUBMIT)
    nodes = {}
    for _ in range(16):
        tid, regs = stop_at(gdb, FLARE_SUBMIT)
        if tid is None:
            continue
        d = struct.unpack(">2f", gdb.read(gpr(regs, 7) + 0xFC, 8))
        node = np.array(struct.unpack(">16f", gdb.read(gpr(regs, 11), 64))).reshape(4, 4)
        nodes.setdefault(gpr(regs, 7), (d, node))
    gdb.remove_breakpoint(FLARE_SUBMIT)
    rows = []
    got = tries = 0
    while got < args.n and tries < 80:
        tries += 1
        gdb.add_breakpoint(SVC1_TAKEN)
        tid, regs = stop_at(gdb, SVC1_TAKEN)
        gdb.remove_breakpoint(SVC1_TAKEN)
        if tid is None or gpr(regs, 19) != hull_rec:
            continue
        _, recs = staged(gdb, regs)
        pose = place.read_pose(gdb, body)
        seen = {}
        gdb.add_breakpoint(EDGEGEOM_MATRIX)
        for _ in range(12):
            tidb, regsb = stop_at(gdb, EDGEGEOM_MATRIX)
            if tidb is None or tidb != tid:
                break
            raw = gdb.read(gpr(regsb, 11), 64)
            seen[raw] = seen.get(raw, 0) + 1
        gdb.remove_breakpoint(EDGEGEOM_MATRIX)
        if not seen or max(seen.values()) < 3:
            print("no main-hull matrix this frame")
            continue
        raw = max(seen.items(), key=lambda kv: kv[1])[0]
        main = np.array(struct.unpack(">16f", raw)).reshape(4, 4)
        inv = np.linalg.inv(main)
        k = nearest(recs, pose["pos"])
        light = np.array(recs[k][:3] + [1.0])
        local = light @ inv
        # The flare node owned by the player: the one nearest the body.
        owner = min(nodes.values(),
                    key=lambda dn: np.linalg.norm(dn[1][3, :3] - np.array(pose["pos"])))
        nl = owner[1] @ inv
        print("node model row3 %s row2 %s (Distance %.3f Radius %.3f); light %s D %.2f "
              "colour %s -> model %s" % (
                  np.round(nl[3, :3], 4), np.round(nl[2, :3], 4), owner[0][0], owner[0][1],
                  np.round(light[:3], 2), recs[k][7], np.round(recs[k][4:7], 1),
                  np.round(local[:3], 3)))
        rows.append({"team": team, "node_model": nl.tolist(), "light": recs[k],
                     "light_model": local[:3].tolist(), "matrix": main.tolist(),
                     "pos": pose["pos"], "rows": pose["rows"]})
        got += 1
    if args.json:
        Path(args.json).write_text(json.dumps(rows, indent=1))


def cmd_inject(gdb, args):
    _, body, team, _, _, hull_rec = player(gdb)
    gdb.add_breakpoint(SVC1_TAKEN)
    done = tries = 0
    while done < args.frames and tries < 80:
        tries += 1
        tid, regs = stop_at(gdb, SVC1_TAKEN)
        if tid is None or gpr(regs, 19) != hull_rec:
            continue
        slot, recs = staged(gdb, regs)
        p = place.read_pose(gdb, body)["pos"]
        k = nearest(recs, p)
        r = recs[k]
        pos = list(args.off) if args.abs else [r[j] + args.off[j] for j in range(3)]
        new = pos + [1.0] + list(args.rgbd)
        if not args.none:
            gdb.write(slot + 32 * k, struct.pack(">8f", *new))
        if done == 0:
            print("%s: light %d of %d was %s; wrote %s; body %s" % (
                team, k, len(recs), ["%.2f" % v for v in r],
                "nothing" if args.none else ["%.2f" % v for v in new],
                ["%.2f" % v for v in p]))
        done += 1
    drive.screenshot(Path(args.out))
    print("shot", args.out, "after", done, "frames")
    if args.pair:
        # The same number of frames again with nothing written: a control
        # registered to the injected shot, since the original's frame drifts.
        done = tries = 0
        while done < args.frames and tries < 80:
            tries += 1
            tid, regs = stop_at(gdb, SVC1_TAKEN)
            if tid is not None and gpr(regs, 19) == hull_rec:
                done += 1
        control = Path(args.out).with_name(Path(args.out).stem + "-control.png")
        drive.screenshot(control)
        print("shot", control, "after", done, "more frames")
    gdb.remove_breakpoint(SVC1_TAKEN)


def cmd_boost(gdb, args):
    _, body, team, _, _, hull_rec = player(gdb)
    out = Path(args.out_dir)
    out.mkdir(parents=True, exist_ok=True)
    pose = place.parse_pose(args.pose)
    gdb.resume()
    forward, up = place.our_attitude(Path(args.oag_game).resolve(), Path(args.image).resolve(),
                                     args.track, pose, out)
    gdb.pause()
    gdb.drain()
    place.write_pose(gdb, body, pose[:3], place.basis_rows(forward, up), args.speed)
    gdb.add_breakpoint(SVC1_TAKEN)
    rows = []
    tries = skipped = 0
    while len(rows) < args.shots and tries < args.tries:
        tries += 1
        tid, regs = stop_at(gdb, SVC1_TAKEN)
        if tid is None or gpr(regs, 19) != hull_rec:
            continue
        _, recs = staged(gdb, regs)
        now = place.read_pose(gdb, body)
        k = nearest(recs, now["pos"])
        if max(recs[k][4:7]) < args.min_colour:
            continue
        if skipped < args.skip:
            skipped += 1
            continue
        shot = out / ("%02d.png" % len(rows))
        if args.ab:
            # The light's boost undone for `--ab` frames (colour back to the
            # rest base, `colour / (1 + 10 * blend)` taken as max / 40), then
            # the same count native: an A/B of the boost term inside one
            # real boosted moment.
            slot = gpr(regs, 3) + 32 * k
            rest = [c * 40.0 / max(recs[k][4:7]) for c in recs[k][4:7]]
            gdb.write(slot + 16, struct.pack(">3f", *rest))
            n = 1
            while n < args.ab:
                tid2, regs2 = stop_at(gdb, SVC1_TAKEN)
                if tid2 is None or gpr(regs2, 19) != hull_rec:
                    continue
                slot2, recs2 = staged(gdb, regs2)
                k2 = nearest(recs2, place.read_pose(gdb, body)["pos"])
                gdb.write(slot2 + 32 * k2 + 16, struct.pack(">3f", *rest))
                n += 1
            drive.screenshot(out / ("%02d-rest.png" % len(rows)))
            n = 0
            while n < args.ab:
                tid2, regs2 = stop_at(gdb, SVC1_TAKEN)
                if tid2 is not None and gpr(regs2, 19) == hull_rec:
                    n += 1
        drive.screenshot(shot)
        speed = sum(v * v for v in now["vel"]) ** 0.5
        print("%s: %s light %s D %.2f at %s, %.1f u/s" % (
            team, shot.name, ["%.0f" % v for v in recs[k][4:7]], recs[k][7],
            ["%.2f" % v for v in now["pos"]], speed))
        rows.append({"team": team, "light": recs[k], "pos": now["pos"], "rows": now["rows"],
                     "vel": now["vel"], "shot": shot.name})
    gdb.remove_breakpoint(SVC1_TAKEN)
    (out / "frames.json").write_text(json.dumps(rows, indent=1))
    print("%d boosted frames in %d stops" % (len(rows), tries))


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("team")
    f = sub.add_parser("flare")
    f.add_argument("n", nargs="?", type=int, default=4)
    f.add_argument("--json")
    i = sub.add_parser("inject")
    i.add_argument("out")
    i.add_argument("rgbd", nargs=4, type=float)
    i.add_argument("--off", nargs=3, type=float, default=[0.0, 0.0, 0.0])
    i.add_argument("--abs", action="store_true")
    i.add_argument("--frames", type=int, default=6)
    i.add_argument("--none", action="store_true", help="no write: a control shot")
    i.add_argument("--pair", action="store_true",
                   help="also shoot a control the same number of frames later")
    b = sub.add_parser("boost")
    b.add_argument("out_dir")
    b.add_argument("--pose", default="-237,-74,55", help="Talon's Junction's first pad")
    b.add_argument("--speed", type=float, default=160.0)
    b.add_argument("--shots", type=int, default=4)
    b.add_argument("--skip", type=int, default=0,
                   help="boosted frames to let pass first, so the chase camera settles")
    b.add_argument("--tries", type=int, default=200)
    b.add_argument("--min-colour", type=float, default=300.0)
    b.add_argument("--ab", type=int, default=0,
                   help="per shot, first K frames with the light at its rest colour (NN-rest.png)")
    b.add_argument("--oag-game", default=str(HERE.parent / "target/debug/oag-game"))
    b.add_argument("--image", default=str(HERE.parent / "data/images/hdfury-ps3-eu-dec.iso"))
    b.add_argument("--track", default=r"Data\Environments\Talons_Junction\track.vex")
    args = ap.parse_args()
    with Debugger(port=port()) as gdb:
        gdb.pause()
        gdb.drain()
        try:
            {"team": cmd_team, "flare": cmd_flare, "inject": cmd_inject,
             "boost": cmd_boost}[args.cmd](gdb, args)
        finally:
            gdb.resume()


if __name__ == "__main__":
    main()
