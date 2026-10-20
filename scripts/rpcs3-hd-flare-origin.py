#!/usr/bin/env python3
"""One pause: dump a frame's flame draws, and in the same pause the live trail blocks and flare objects.

    OAG_RPCS3_ATTACH=1 HOOKPY=<hook> python3 scripts/rpcs3-hd-flare-origin.py <out-dir> <stem> [button ...]

The draws (`rsx-draw-list.py <out-dir> <stem>`) give the hull and flare model matrices of
that frame; `<stem>_trail<N>.bin` (the 0x1230 trail block of craft slot N, ring head first)
and `<stem>_flare<N>.bin` (0x800 bytes of its `EngineFlare`, found at `craft + 0x5f70`)
belong to the same paused instant. Used to place the flare's origin against the hull and the
`Engine Flare` locator (engine-trail.md, "The flare's origin at thrust").
"""
import importlib.util, os, struct, sys, time, types
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec); spec.loader.exec_module(drive)
from rpcs3_debugger import Debugger
MANAGER_SLOT = 0x00AED460 + 0x6C
FLARE_OFFSET = 0x5F70
u32 = lambda b, o=0: struct.unpack_from(">I", b, o)[0]
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
        manager = u32(g.read(MANAGER_SLOT, 4)); alloc = u32(g.read(manager + 0x40, 4))
        for i in range(8):
            blk = g.read(alloc + 0x84A0 + i * 0x1230, 0x1230)
            (out / f"{stem}_trail{i}.bin").write_bytes(blk)
            craft = u32(blk, 0x1204)
            if not 0x10000000 <= craft < 0x50000000:
                continue
            try:
                flare = u32(g.read(craft + FLARE_OFFSET, 4))
            except Exception:
                continue
            print(f"slot {i} craft {craft:08x} flare {flare:08x}", flush=True)
            if flare:
                (out / f"{stem}_flare{i}.bin").write_bytes(g.read(flare, 0x800))
            (out / f"{stem}_craft{i}.bin").write_bytes(g.read(craft, 0x400))
        g.resume()
finally:
    for b in hold: pad.set(b, False)
