#!/usr/bin/env python3
"""Sample the player's pose with HD's two live environment maps, paused each time, while the
craft drives (buttons held): <out-dir> <ibl-addr> <refl-addr> <count> <gap-s> [button ...].
Writes <out-dir>/NN.json (pose) and NN-ibl.bin / NN-refl.bin (A8R8G8B8, big endian)."""
import importlib.util, json, os, sys, time
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec); spec.loader.exec_module(drive)
import rpcs3_place as place
from rpcs3_debugger import Debugger
out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
ibl, refl = int(sys.argv[2], 16), int(sys.argv[3], 16)
count, gap = int(sys.argv[4]), float(sys.argv[5])
hold = sys.argv[6:]
host, port = os.environ["OAG_RPCS3_GDB"].split(":")
pad = drive.RemotePad(drive.read_session()["socket"])
for b in hold: pad.set(b, True)
try:
    time.sleep(2)
    for i in range(count):
        with Debugger(port=int(port), host=host, timeout=60.0) as g:
            g.pause()
            ship, body = place.find_player(g)
            pose = place.read_pose(g, body)
            a = g.read(ibl, 0x2000); b = g.read(refl, 0x8000)
            g.resume()
        (out / ("%02d.json" % i)).write_text(json.dumps(pose))
        (out / ("%02d-ibl.bin" % i)).write_bytes(a); (out / ("%02d-refl.bin" % i)).write_bytes(b)
        print(i, [round(x, 1) for x in pose["pos"]], flush=True)
        time.sleep(gap)
finally:
    for b in hold: pad.set(b, False)
