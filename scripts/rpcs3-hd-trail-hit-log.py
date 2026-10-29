#!/usr/bin/env python3
"""Log every Wipeout HD trail-hit spawn on an attached RPCS3 served with `--interpreter`.

    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-trail-hit-log.py OUT.json MAX_HITS SECONDS

A breakpoint on `0x002e3b98` (`Trail_SpawnHitEffect`'s `bl Trail_HitShipEffect`); per hit: the
float `f1` (the trail block's `+0x11dc`), the attachment node `r3`, the variant `r5`, the craft at
the block's `+0x1204` (`r11`) and its grid slot, `+0x11d0`, `+0x11f0` and the player's position.
Consecutive hits one frame of travel apart are the per-frame rate (`engine-trail.md`).
"""
import importlib.util, sys, time, struct, json
from pathlib import Path
REPO = Path(__file__).resolve().parent
sys.path.insert(0, str(REPO))
spec = importlib.util.spec_from_file_location("rpcs3_drive", REPO / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec); spec.loader.exec_module(drive)
from rpcs3_debugger import Debugger, REG_PC, REG_GPR, REG_FPR
import rpcs3_place as P
SITE = 0x002E3B98
out = sys.argv[1]; max_hits = int(sys.argv[2]); secs = float(sys.argv[3]); skip = len(sys.argv) > 4 and sys.argv[4] == "skip"
g = lambda r, i: int.from_bytes(r[REG_GPR + 8*i:REG_GPR + 8*i + 8], "big") & 0xFFFFFFFF
f = lambda r, i: struct.unpack_from(">d", r, REG_FPR + 8*i)[0]
port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
hits = []
with Debugger(port=port) as gdb:
    gdb.pause()
    ship, body = P.find_player(gdb)
    ships = list(struct.unpack(">8I", gdb.read(P.CRAFT_ARRAY, 32)))
    gdb.add_breakpoint(SITE)
    t0 = time.time()
    while len(hits) < max_hits and time.time() - t0 < secs:
        tid, regs = None, None
        gdb.resume()
        if gdb.wait_for_stop(1.0) is None:
            gdb.pause()
        gdb.drain(0.1)
        for t in gdb.threads():
            rr = gdb.registers(t)
            if rr is not None and int.from_bytes(rr[REG_PC:REG_PC + 8], "big") == SITE:
                tid, regs = t, rr
                break
        if tid is None:
            continue
        r30 = g(regs, 30); craft = g(regs, 11)
        node = g(regs, 3)
        rec = {"t": round(time.time() - t0, 2), "f1": f(regs, 1), "node": hex(node), "variant": g(regs, 5),
               "craft": hex(craft), "craft_slot": ships.index(craft) if craft in ships else -1,
               "player": craft == ship, "block": hex(r30),
               "w11d0": gdb.read(r30 + 0x11d0, 8).hex(), "s11f0": struct.unpack(">I", gdb.read(r30 + 0x11f0, 4))[0],
               "f11dc": struct.unpack(">f", gdb.read(r30 + 0x11dc, 4))[0],
               "pos": P.read_pose(gdb, body)["pos"]}
        try:
            rec["node_pos"] = struct.unpack(">3f", gdb.read(node + 0x30, 12)) if node else None
        except Exception:
            pass
        hits.append(rec); print(rec, flush=True)
        gdb.remove_breakpoint(SITE)
        gdb.step(tid)
        gdb.drain(0.1)
        gdb.add_breakpoint(SITE)
    gdb.remove_breakpoint(SITE)
json.dump(hits, open(out, "w"), indent=1)
