#!/usr/bin/env python3
"""Read the original's render queue before it is sorted: every `Gfx_Enqueue` item and key of one frame.

    source <lane>/pp/env.sh; export DISPLAY
    python3 scripts/psp-queue-read.py OUT.json            # CLOCK=38.4082 by default

Loads `race-talons-venom-alongside.ppst` with a breakpoint on `Gfx_FlushRenderManager` (0x0891e3c0) armed, runs to the
state's own race clock, and writes `{flare, node, disp, ents}` where `ents` is `[index, item, key, first word of item]` for
each of `display+0x5520` entries at `display+0x16a0` (`g_display` is the pointer at 0x08abf5d4). `flare` and `node` are the
player's `ExhaustFlare` and its `Trail` (`*(flare+0x64)`, the queue item itself, `+0x60` is `node`). A key is `layer << 24 |
depth`, drawn ascending: on the race pair the player's `Trail` is `0x4d000000` (depth zero, every ship's is) and its flare
`0x4d0ff75e`, with 40-odd list-B meshes between them (`docs/ghidra/functions/psp-pulse-usa/exhaust.md`, "The ribbon is
queued first"). Output is derived game data: keep it under data/, never commit it.
"""

import importlib.util
import json
import os
import shutil
import struct
import sys
import time
from pathlib import Path

R = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(R / "scripts"))
from ppsspp_debugger import Debugger  # noqa: E402
from psp_trace_fields import FLARE_POINTER, RACER_POINTER  # noqa: E402

FLUSH = 0x0891E3C0
G_DISPLAY = 0x08ABF5D4
CLOCK = 0x08B344B0


def _load(name, rel):
    spec = importlib.util.spec_from_file_location(name, R / rel)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def main():
    st = _load("st", "scripts/psp-state.py")
    rf = _load("rf", "scripts/psp-race-field.py")
    d = Debugger(int(os.environ["OAG_PPSSPP_PORT"]))
    if d.is_stepping():
        d.call("cpu.resume")
        time.sleep(1)
    target = st.state_dir(os.environ["PP_HOME"]) / "UCUS98712_1.00_1.ppst"
    shutil.copy(R / os.environ.get("RIBBON_STATE", "data/saves/pulse-psp/race-talons-venom-alongside.ppst"), target)
    st.set_menu(d, True)
    d.brk()
    d.add_breakpoint(FLUSH)
    st.click(st.LOAD_X, st.SLOT_Y0 + st.SLOT_DY * 1)
    time.sleep(2.5)
    d.pending = []
    d.call("cpu.resume")
    d.wait_for_break(FLUSH, timeout=60)
    goal = float(os.environ.get("TARGET_CLOCK", "38.4082"))
    while d.read_f32(CLOCK) < goal - 0.0005:
        d.call("cpu.status")
        d.pending = []
        d.call("cpu.resume")
        d.wait_for_break(FLUSH, timeout=30)
    craft = rf.Field(d).player["craft"]
    racer = d.read_u32(craft + RACER_POINTER)
    flare = d.read_u32(racer + FLARE_POINTER)
    node = d.read_u32(flare + 0x64) + 0x60
    disp = d.read_u32(G_DISPLAY)
    count = d.read_u32(disp + 0x5520)
    raw = d.read(disp + 0x16A0, count * 8)
    ents = []
    for i in range(count):
        item, key = struct.unpack_from("<II", raw, i * 8)
        ents.append((i, item, key, d.read_u32(item) if item else 0))
    Path(sys.argv[1]).write_text(json.dumps({"flare": flare, "node": node, "disp": disp, "ents": ents}))
    d.brk()
    d.remove_breakpoint(FLUSH)
    print("clock", d.read_f32(CLOCK), "entries", count)


if __name__ == "__main__":
    main()
