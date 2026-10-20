#!/usr/bin/env python3
"""Poll HD's two live environment maps (64x32 IBL, 128x64 reflection) without pausing
RPCS3 and print when each changes. Attached run:

    OAG_RPCS3_ATTACH=1 python3 scripts/hd-ibl-poll.py <ibl-addr> <refl-addr> [seconds]
"""
import hashlib, importlib.util, sys, time
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec); spec.loader.exec_module(drive)
GUEST = 0x300000000
ibl, refl = int(sys.argv[1], 16), int(sys.argv[2], 16)
secs = float(sys.argv[3]) if len(sys.argv) > 3 else 20
hold = sys.argv[4:]
pad = drive.RemotePad(drive.read_session()["socket"])
for b in hold: pad.set(b, True)
import os
keep = os.environ.get("KEEP")
if keep: os.makedirs(keep, exist_ok=True)
mem = drive.RemoteMem(drive.read_session()["socket"])
def rd(a, n):
    mem.seek(GUEST + a); return mem.read(n)
last = {}
t0 = time.time()
n = 0
while time.time() - t0 < secs:
    t = time.time() - t0
    for name, a, sz in (("ibl", ibl, 0x2000), ("refl", refl, 0x8000)):
        d = rd(a, sz); h = hashlib.md5(d).hexdigest()[:8]
        if last.get(name) != h:
            print("%.3f %s %s" % (t, name, h), flush=True); last[name] = h
            if keep: open("%s/%s-%s.bin" % (keep, name, h), "wb").write(d)
    n += 1
print("samples", n)
for b in hold: pad.set(b, False)
