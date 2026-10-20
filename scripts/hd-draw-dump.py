#!/usr/bin/env python3
"""Pause the attached RPCS3 and dump one frame's RSX draw state with `rpcs3_draw_hook`.

    OAG_RPCS3_ATTACH=1 python3 scripts/hd-draw-dump.py <out-dir> <stem> [button ...]

Buttons are held for `HOLD` seconds (default 0) first. The draws read back with
`scripts/rsx-draw-list.py <out-dir> <stem>`, the fragment programs sit in `<out-dir>`
as `<stem>-<address>.bin` for `scripts/ps3-fp-live.py`.
"""
import importlib.util, os, sys, time, types
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec); spec.loader.exec_module(drive)
from rpcs3_debugger import Debugger
out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
stem = sys.argv[2]; hold = sys.argv[3:]
host, port = os.environ["OAG_RPCS3_GDB"].split(":")
pad = drive.RemotePad(drive.read_session()["socket"])
for b in hold: pad.set(b, True)
time.sleep(float(os.environ.get("HOLD", "0")))
args = types.SimpleNamespace(dump=[], dump_attempts=12, dump_gap=0.5,
                             hook=os.environ.get("HOOKPY", str(HERE / "rpcs3_draw_hook.py")))
try:
    with Debugger(port=int(port), host=host, timeout=90.0) as g:
        g.pause()
        drive.dump_frame(g, out, stem, args)
        g.resume()
finally:
    for b in hold: pad.set(b, False)
