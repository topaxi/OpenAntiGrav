#!/usr/bin/env python3
"""HD's Zone end camera on a live RPCS3: frames and the RSX camera at set times after the blast.

    source data/scratch/<lane>/env.sh
    scripts/emu-hd-zone-walk.sh <lane> 0 0 0 ; scripts/emu-save-state.sh <lane> <name>   # once
    scripts/emu-restore-state.sh <lane> data/saves/hd-fury/<name>.SAVESTAT.zst          # with the GDB stub
    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-zone-end-orbit.py \\
        --image data/images/hdfury-ps3-eu-dec.iso --out data/scratch/<lane>/orbit1

Taps START RACE, waits until the Zone craft is at speed, writes its shield `ship+0x5fa0` to 0 and polls
`ship+0x5fa4` until it reads state 4 (the blast), then pauses the game through the GDB stub. From that
pause it steps the game forward by resuming for a wall-clock interval chosen from the craft's age
`ship+0x6a30` (`+= dt` every tick), and at each of `--at` seconds after the blast it pauses, waits for the
RSX to present, grabs the frame through `grab_with_clock` and reads the pushbuffer (the camera is picked
across the shots, `ps3_pose.pick_camera`). `shots.json` holds, per shot, the craft's age since the blast, its
state, its body position and rows, and the camera's eye, so the eye-to-craft distance is read off directly.
`docs/ghidra/functions/ps3-hdfury-eu/destroy-camera.md`, "Measured on RPCS3".
"""
import argparse
import importlib.util
import json
import struct
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec)
sys.modules["rpcs3_drive"] = drive
spec.loader.exec_module(drive)
import ps3_pose  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402

G = 0x300000000
SHIP_TABLE = 0x0098D7C0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--at", default="0.0,0.25,0.5,1.0,1.5,2.5,3.5", help="seconds of craft age after the blast")
    ap.add_argument("--race-secs", type=float, default=120.0)
    ap.add_argument("--flush", type=float, default=2.5, help="seconds a paused frame is given to be presented")
    ap.add_argument("--shield", type=float, default=0.0)
    args = ap.parse_args()
    out = Path(args.out)
    (out / "frames").mkdir(parents=True, exist_ok=True)
    targets = [float(x) for x in args.at.split(",")]
    with drive.Session(args.image, str(out / "logs")) as session:
        mem = session.open_mem()

        def rd(addr, n):
            mem.seek(G + addr)
            return mem.read(n)

        def u32(addr):
            return struct.unpack(">I", rd(addr, 4))[0]

        def f32(addr):
            return struct.unpack(">f", rd(addr, 4))[0]

        ships = [u32(SHIP_TABLE + 4 * k) for k in range(8)]
        player = next(s for s in ships if s and u32(s + 0x7A60) == 0)
        body = u32(player + 0x6944)
        print("player %#x body %#x" % (player, body), flush=True)

        def speed():
            return sum(v * v for v in struct.unpack(">3f", rd(body + 0x190, 12))) ** 0.5

        t0 = time.time()
        while time.time() - t0 < args.race_secs and speed() < 90.0:
            if speed() < 10.0:
                session.tap("cross", settle=0.0)
            time.sleep(1.0)
        print("at speed %.1f after %.0f s" % (speed(), time.time() - t0), flush=True)
        poke = struct.pack(">f", args.shield)
        state = -1
        t1 = time.time()
        with Debugger(port=int(drive.GDB_SERVER.rsplit(":", 1)[1])) as gdb:
            gdb.resume()
            while time.time() - t1 < 60.0:
                mem.seek(G + player + 0x5FA0)
                mem.write(poke)
                state = struct.unpack(">i", rd(player + 0x5FA4, 4))[0]
                if state >= 4:
                    break
                time.sleep(0.002)
            gdb.pause()
            if state < 4:
                sys.exit("the craft never reached state 4")

            def age():
                return f32(player + 0x6A30)

            def read_shot(target, age0):
                time.sleep(args.flush)
                path = out / "frames" / ("t%04d.png" % round(target * 1000))
                _, clock = drive.grab_with_clock(path, trim=True)
                blobs = []
                for chain, size in drive.PUSHBUFFER_REGIONS:
                    at = drive.resolve_chain(gdb, chain)
                    blobs.append((at, gdb.read(at, size)))
                cands = []
                for at, blob in blobs:
                    cands.extend(ps3_pose.packet_candidates(blob, base=at))
                rows = [struct.unpack(">3f", rd(body + off, 12)) for off in (0x1D0, 0x1E0, 0x1F0)]
                return {
                    "target": target,
                    "age_since_blast": age() - age0,
                    "state": struct.unpack(">i", rd(player + 0x5FA4, 4))[0],
                    "state_timer": f32(player + 0x6A78),
                    "craft": struct.unpack(">3f", rd(body + 0x200, 12)),
                    "rows": rows,
                    "velocity": struct.unpack(">3f", rd(body + 0x190, 12)),
                    "frame": str(path),
                    "world_clock": clock,
                }, cands

            age0 = age()
            print("blast: state %d, age %.4f" % (state, age0), flush=True)
            shots, cand_sets = [], []
            rate = None
            for target in targets:
                while True:
                    done = age() - age0
                    if abs(done - target) < 0.03 or done > target:
                        break
                    wait_game = target - done
                    wall = wait_game / rate if rate else min(wait_game, 0.2)
                    a = age()
                    w0 = time.time()
                    gdb.resume()
                    time.sleep(max(wall - 0.03, 0.01))
                    gdb.pause()
                    gone = time.time() - w0
                    if gone > 0 and age() > a:
                        rate = (age() - a) / gone
                shot, cands = read_shot(target, age0)
                print("  target %.2f: age %.3f state %d craft %s" % (
                    target, shot["age_since_blast"], shot["state"], [round(v, 1) for v in shot["craft"]]), flush=True)
                shots.append(shot)
                cand_sets.append(cands)
            for shot, cands in zip(shots, cand_sets):
                near = []
                for c in cands:
                    eye = c[3]
                    d = sum((e - v) ** 2 for e, v in zip(eye, shot["craft"])) ** 0.5
                    shared = sum(1 for other in cand_sets if any(o[3] == eye for o in other))
                    if d < 60.0:
                        near.append({"register": c[5], "eye": list(eye), "dist": d, "unit_error": c[2], "shots_sharing": shared})
                shot["near_candidates"] = near
                uniq = sorted((n for n in near if n["shots_sharing"] == 1 and n["unit_error"] < 1e-3), key=lambda n: n["dist"])
                print("  %.2f: %d near, frame-unique nearest %s" % (
                    shot["target"], len(near), [(n["register"], round(n["dist"], 2)) for n in uniq[:4]]), flush=True)
            picks = ps3_pose.pick_camera(cand_sets)
            for shot, (camera, reason, count) in zip(shots, picks):
                shot["camera"] = None if camera is None else {"eye": camera["eye"], "unit_error": camera["unit_error"],
                                                              "registers": camera["registers"]}
                shot["camera_reason"] = reason
                if camera is not None:
                    d = [e - c for e, c in zip(camera["eye"], shot["craft"])]
                    shot["eye_to_craft"] = sum(v * v for v in d) ** 0.5
                print("  %.2f -> eye %s dist %s (%s)" % (shot["target"], shot.get("camera") and [round(v, 2) for v in camera["eye"]],
                                                       shot.get("eye_to_craft"), reason), flush=True)
            gdb.resume()
        (out / "shots.json").write_text(json.dumps(shots, indent=1))
    print("done: %d shots" % len(shots))


if __name__ == "__main__":
    main()
