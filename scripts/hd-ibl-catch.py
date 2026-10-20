#!/usr/bin/env python3
"""Catch the frame in which HD's live environment map changes: poll the map (no pause),
and on the first change pause the target and dump the RSX draw state with rpcs3_draw_hook.

    OAG_RPCS3_ATTACH=1 python3 scripts/hd-ibl-catch.py <out-dir> <stem> <ibl-addr> <refl-addr>
"""
import hashlib, importlib.util, os, sys, time, types
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec); spec.loader.exec_module(drive)
from rpcs3_debugger import Debugger
GUEST = 0x300000000
out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
stem = sys.argv[2]
ibl, refl = int(sys.argv[3], 16), int(sys.argv[4], 16)
host, port = os.environ["OAG_RPCS3_GDB"].split(":")
mem = drive.RemoteMem(drive.read_session()["socket"])
def h():
    mem.seek(GUEST + ibl); a = mem.read(0x2000)
    mem.seek(GUEST + refl); b = mem.read(0x8000)
    return hashlib.md5(a + b).hexdigest()
pad = drive.RemotePad(drive.read_session()["socket"])
for b in sys.argv[5:]: pad.set(b, True)
time.sleep(float(os.environ.get("HOLD", "3")))
first = h(); t0 = time.time()
while h() == first:
    if time.time() - t0 > 30: sys.exit("no change in 30 s")
t1 = time.time()
args = types.SimpleNamespace(dump=["0x%x:0x2000" % ibl, "0x%x:0x8000" % refl], dump_attempts=12, dump_gap=0.5,
                             hook=os.environ.get("HOOKPY", str(HERE / "rpcs3_draw_hook.py")))
with Debugger(port=int(port), host=host, timeout=90.0) as g:
    g.pause()
    print("paused %.3f s after the change" % (time.time() - t1), flush=True)
    drive.dump_frame(g, out, stem, args)
    g.resume()
for b in sys.argv[5:]: pad.set(b, False)
