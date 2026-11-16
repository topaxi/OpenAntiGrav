#!/usr/bin/env python3
"""Pause the attached RPCS3, read RSX or main memory spans, resume.

    OAG_RPCS3_ATTACH=1 python3 scripts/hd-rsx-read.py <out-dir> <addr>:<len> [...]

`<addr>` is the PPU/RSX address: `0xC0000000 + offset` for local memory (a texture whose
`SET_TEXTURE_OFFSET` location nibble is 1), `0x40000000 + offset` for main memory. Each span is written to
`<out-dir>/<addr as 8 hex digits>.bin`. Textures and vertex data are static in a paused race, so a read
after `hd-draw-dump.py` sees the same bytes the draws used.
"""
import os, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from rpcs3_debugger import Debugger

out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
host, port = os.environ["OAG_RPCS3_GDB"].split(":")
with Debugger(port=int(port), host=host, timeout=90.0) as g:
    g.pause()
    try:
        for span in sys.argv[2:]:
            a, n = span.split(":")
            a = int(a, 0); n = int(n, 0)
            (out / ("%08x.bin" % a)).write_bytes(g.read(a, n))
            print("read %#x+%#x" % (a, n), flush=True)
    finally:
        g.resume()
