#!/usr/bin/env python3
"""Matched pairs of Wipeout HD's trail-hit burst: the player tucked in a rival's trail, ours beside it (RPCS3).

    source data/scratch/<lane>/env.sh
    scripts/emu-restore-state.sh <lane> data/saves/hd-fury/hd-race-talons-venom-pack.SAVESTAT.zst --pause
    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-trail-hit-pair.py OUT N \\
        [--back 8] [--frames 30] [--team feisar] [--track /data/environments/talons_junction/track.vex]

Restore the state (`--pause`) before every pair: the tuck needs a rival ahead, and a few seconds of the
Autopilot leave the player leading. N times: pause, re-arm the player's Autopilot for good (`*(ship+0x5edc)+0x200 |= 0x800`, `+0x158 = 10000`,
docs/ghidra/functions/ps3-hdfury-eu/autopilot.md), put the player's body `--back` units behind the nearest rival
ahead on its basis rows and at its speed (`rpcs3_place`, the tuck `rpcs3-hd-trail-tuck.py` does), re-placed so
before each of `--frames` game frames (counted off the lap clock) so the burst, the camera and the drawn hull
settle, pause again, then run `rpcs3-hd-pair.py --field --keep-autopilot --pack-min-in-view 1`
into `OUT/NN`: the frame, the camera, the whole field and ours rendered from the same pause, ours' field warm-up
running the trail-hit trigger (`Race::warm_up_field`). Never let the pair write the Autopilot's authored 5 s back
(`--keep-autopilot`): the game froze at 1.12 s left, twice, and every capture after read a frozen clock. Used for the matched pair on
docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md. Raw captures are derived game data: keep them under `data/`.
"""
import argparse
import importlib.util
import json
import math
import os
import struct
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)
from rpcs3_debugger import Debugger  # noqa: E402
import rpcs3_place as P  # noqa: E402
import rpcs3_hd_field  # noqa: E402

AUTOPILOT_SLOT = 0x5EDC


def nearest_ahead(gdb, ship, me):
    best = None
    for s in struct.unpack(">8I", gdb.read(P.CRAFT_ARRAY, 32)):
        if not s or s == ship:
            continue
        b = P.u32(gdb.read(s + P.OFF_BODY, 4))
        p = P.read_pose(gdb, b)
        d = [p["pos"][i] - me["pos"][i] for i in range(3)]
        ahead = sum(d[i] * me["rows"][2][i] for i in range(3))
        dist = math.sqrt(sum(x * x for x in d))
        if ahead > 0 and (best is None or dist < best[0]):
            best = (dist, s, b)
    return best


def place_behind(gdb, body, rival_body, back):
    """The player `back` units behind the rival along its *velocity*, sharing that velocity and its basis: a
    cornering craft drifts off its own forward row, and a tuck along the row slid the player 1.6 units off the
    ribbon within two frames."""
    rp = P.read_pose(gdb, rival_body)
    speed = math.sqrt(sum(v * v for v in rp["vel"]))
    f = [v / speed for v in rp["vel"]]
    pos = tuple(rp["pos"][i] - f[i] * back for i in range(3))
    P.write_pose(gdb, body, pos, [tuple(x) for x in rp["rows"]], speed)
    gdb.write(body + P.B_VELOCITY, struct.pack(">3f", *rp["vel"]))
    return speed


def tuck(gdb, back, frames):
    """Hold the player in the rival's wake for `frames` game frames, re-placed before every one, and leave the
    game paused after the last. One placement was not enough: the Autopilot steered the craft 2 to 10 units off
    the ribbon within a few frames, and the frame still showed the hull short of its new pose."""
    gdb.pause()
    ship, body = P.find_player(gdb)
    slot = P.u32(gdb.read(ship + AUTOPILOT_SLOT, 4))
    if slot:
        flags = P.u32(gdb.read(slot + 0x200, 4))
        gdb.write(slot + 0x200, struct.pack(">I", flags | 0x800))
        gdb.write(slot + 0x158, struct.pack(">f", 10000.0))
    best = nearest_ahead(gdb, ship, P.read_pose(gdb, body))
    if best is None:
        gdb.resume()
        return None
    dist, rival, rival_body = best
    # Frames are counted off the player's lap clock read live (`ship+0x7014`, 1:1 with emulated time): the
    # emulator's speed against the wall clock drifts.
    mem = rpcs3_hd_field.Mem(drive.RemoteMem(drive.read_session()["socket"]))
    speed = 0.0
    for _ in range(frames):
        speed = place_behind(gdb, body, rival_body, back)
        clock0 = mem.f32(ship + 0x7014)
        gdb.resume()
        deadline = time.time() + 5
        while time.time() < deadline:
            now = mem.f32(ship + 0x7014)
            if now - clock0 >= 1.0 / 60 - 1e-4 or now < clock0:
                break
            time.sleep(0.002)
        gdb.pause()
    return {"rival": hex(rival), "rival_dist_before": dist, "speed": speed, "frames": frames,
            "fury": gdb.read(ship + 0x7D2C, 1)[0], "rival_fury": gdb.read(rival + 0x7D2C, 1)[0]}


class Held:
    """The tuck's own connection, handed to `rpcs3-drive.py capture` in-process. The proxy resumes a target its
    client leaves paused, so a capture in a second connection found the game a second or more further on and the
    player out of the wake."""

    def __init__(self, gdb):
        self.gdb = gdb

    def __call__(self, *args, **kwargs):
        return self

    def __enter__(self):
        return self.gdb

    def __exit__(self, *_):
        return False


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("out", type=Path)
    ap.add_argument("n", type=int)
    ap.add_argument("--back", type=float, default=8.0)
    ap.add_argument("--frames", type=int, default=30, help="game frames the player is held in the wake before the pause")
    ap.add_argument("--interval", type=float, default=0.6,
                    help="the capture's gap between its two shots; RPCS3 ran this race at about a sixth of real time")
    ap.add_argument("--team", default="feisar")
    ap.add_argument("--track", default="/data/environments/talons_junction/track.vex")
    ap.add_argument("--start", type=int, default=0, help="first index, to extend a run")
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    os.environ.setdefault("OAG_CAPTURE_FLUSH", "3")
    pair = load("rpcs3_hd_pair", "rpcs3-hd-pair.py")
    import rpcs3_debugger
    log = []
    for i in range(args.start, args.start + args.n):
        dest = args.out / ("%02d" % i)
        pair.set_window(1920, 1080)
        with Debugger(port=port) as gdb:
            meta = tuck(gdb, args.back, args.frames)
            print("%02d tuck %s" % (i, meta), flush=True)
            real = rpcs3_debugger.Debugger
            rpcs3_debugger.Debugger = Held(gdb)
            try:
                drive.main(["capture", "--out", str(dest / "capture"), "--shots", "2", "--interval", str(args.interval),
                            "--load", "0", "--team", args.team, "--press", "none", "--field", "--in-race",
                            "--settle", "0", "--first-at-pause"])
            finally:
                rpcs3_debugger.Debugger = real
        cmd = [sys.executable, HERE / "rpcs3-hd-pair.py", "--out", dest, "--team", args.team,
               "--field", "--keep-autopilot", "--pack-min-in-view", "1", "--track", args.track,
               "--json", dest / "capture" / "00.json",
               "--accept-provenance", "autopilot_timer=re-armed for good before every tuck by this script"]
        rc = subprocess.run([str(c) for c in cmd], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode
        print("%02d pair rc %d" % (i, rc), flush=True)
        log.append({"i": i, "tuck": meta, "pair_rc": rc})
        (args.out / "loop.json").write_text(json.dumps(log, indent=1))


if __name__ == "__main__":
    main()
