#!/usr/bin/env python3
"""Put Wipeout HD's player craft inside a rival's engine trail on an attached RPCS3 race, then step.

    source data/scratch/<lane>/env.sh
    scripts/emu-restore-state.sh <lane> data/saves/hd-fury/hd-race-talons-venom-pack.SAVESTAT.zst --pause
    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-trail-tuck.py OUT BACK N STEP FLUSH [LATERAL]

Pauses, finds the nearest rival ahead of the player, writes the player's body BACK units behind
it on its own basis rows and speed (LATERAL units along its right row), then N times: resume for
STEP s, pause, wait FLUSH s for the presented frame, grab `OUT/sNN.png`. Writes `OUT/tuck.json`
(the rival, gaps and positions). The craft array and body layout are `rpcs3_place`'s. Used for
the trail-hit burst pair, `docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`.
"""
import importlib.util, sys, time, struct, math, json
from pathlib import Path
REPO = Path(__file__).resolve().parent
sys.path.insert(0, str(REPO))
spec = importlib.util.spec_from_file_location("rpcs3_drive", REPO / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec); spec.loader.exec_module(drive)
from rpcs3_debugger import Debugger
import rpcs3_place as P
out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
back = float(sys.argv[2]); n = int(sys.argv[3]); step = float(sys.argv[4]); flush = float(sys.argv[5])
lateral = float(sys.argv[6]) if len(sys.argv) > 6 else 0.0
port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
log = []
with Debugger(port=port) as gdb:
    gdb.pause()
    ship, body = P.find_player(gdb)
    me = P.read_pose(gdb, body)
    ships = struct.unpack(">8I", gdb.read(P.CRAFT_ARRAY, 32))
    best = None
    for s in ships:
        if not s or s == ship: continue
        b = P.u32(gdb.read(s + P.OFF_BODY, 4)); p = P.read_pose(gdb, b)
        d = [p["pos"][i] - me["pos"][i] for i in range(3)]
        ahead = sum(d[i] * me["rows"][2][i] for i in range(3)); dist = math.sqrt(sum(x*x for x in d))
        if ahead > 0 and (best is None or dist < best[0]): best = (dist, s, b, p)
    dist, rs, rb, rp = best
    f = rp["rows"][2]; r = rp["rows"][0]
    pos = tuple(rp["pos"][i] - f[i] * back + r[i] * lateral for i in range(3))
    speed = math.sqrt(sum(v*v for v in rp["vel"]))
    P.write_pose(gdb, body, pos, [tuple(x) for x in rp["rows"]], speed)
    meta = {"player": hex(ship), "rival": hex(rs), "rival_dist_before": dist, "back": back, "lateral": lateral,
            "speed": speed, "rival_pos": rp["pos"], "fury": gdb.read(ship + 0x7D2C, 1)[0], "rival_fury": gdb.read(rs + 0x7D2C, 1)[0]}
    print(meta, flush=True)
    for i in range(n):
        gdb.resume(); time.sleep(step); gdb.pause()
        mp = P.read_pose(gdb, body); rq = P.read_pose(gdb, rb)
        d = [rq["pos"][k] - mp["pos"][k] for k in range(3)]
        time.sleep(flush)
        drive.screenshot(out / ("s%02d.png" % i))
        rec = {"i": i, "gap": math.sqrt(sum(x*x for x in d)), "me": mp["pos"], "rival": rq["pos"]}
        log.append(rec); print(rec, flush=True)
    gdb.resume()
json.dump({"meta": meta, "steps": log}, open(out / "tuck.json", "w"), indent=1)
