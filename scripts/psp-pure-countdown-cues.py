#!/usr/bin/env python3
"""Log every cue Pure (PSP, USA) starts across a restart, and the craft's launch gate after `go`.

`psp-countdown-cues.py` is Pulse's: its `Scream_StartSound`, `Ship_UpdateCraft` and state
machine addresses are Pulse's. Pure's `Scream_StartSound` is `0x0883defc`
(`docs/ghidra/functions/psp-pure-usa/countdown-voice.md`) and `Ship_UpdateEngine` is
`0x0892f1f0`. Pure has no tick axis this script trusts, so rows are PSP frames from the first
cue start, and the craft is dumped on every `Ship_UpdateEngine` hit after the cue named by
`--stop-bank/--stop-cue` (`go`, `SPEECH` 13), so the update that flips the launch gate shows.

Start the game with `scripts/ppsspp-start.sh`, walk the menus to a Time Trial, bring up the
post-race or pause menu with RESTART or RACE AGAIN highlighted, then:

    uv run --with websocket-client python3 scripts/psp-pure-countdown-cues.py --port 45711 \\
        --dump craft.json

It confirms RESTART with cross and holds cross for the run.
"""

import argparse
import importlib.util
import json
import struct
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from ppsspp_debugger import Debugger  # noqa: E402

spec = importlib.util.spec_from_file_location("pc", HERE / "psp-countdown-cues.py")
pc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pc)

START_SOUND = 0x0883DEFC
UPDATE_ENGINE = 0x0892F1F0


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--port", type=int, required=True)
    ap.add_argument("--stop-bank", default="SPEECH")
    ap.add_argument("--stop-cue", type=int, default=13)
    ap.add_argument("--updates", type=int, default=80)
    ap.add_argument("--dump", type=Path, help="write the craft dumps as JSON (hex per update)")
    args = ap.parse_args()

    dbg = Debugger(args.port)
    dbg.brk()
    dbg.add_breakpoint(START_SOUND)
    dbg.call("cpu.resume")
    dbg.press("cross", duration=6)
    time.sleep(0.3)
    dbg.hold(cross=True)

    first = None
    stop = None
    while True:
        dbg.wait_for_break(START_SOUND, timeout=180.0)
        regs = pc.gpr(dbg)
        cycle = dbg.call("cpu.status")["ticks"]
        bank, names = pc.bank_names(dbg, regs["a0"])
        index = regs["a1"]
        first = first or cycle
        print("%8.1f %s %d" % ((cycle - first) / pc.CYCLES_PER_FRAME, bank, index), flush=True)
        if (bank, index) == (args.stop_bank, args.stop_cue):
            stop = cycle
            break
        dbg.call("cpu.resume")

    dbg.remove_breakpoint(START_SOUND)
    dbg.add_breakpoint(UPDATE_ENGINE)
    rows = []
    for _ in range(args.updates):
        dbg.call("cpu.resume")
        dbg.wait_for_break(UPDATE_ENGINE, timeout=30.0)
        cycle = dbg.call("cpu.status")["ticks"]
        craft = pc.gpr(dbg)["a0"]
        rows.append({"frames": (cycle - stop) / pc.CYCLES_PER_FRAME, "craft": craft,
                     "bytes": dbg.read(craft, 0x900).hex()})
    dbg.remove_breakpoint(UPDATE_ENGINE)
    dbg.resume()
    dbg.hold(cross=False)
    dbg.close()

    def word(row, off):
        return struct.unpack_from("<I", bytes.fromhex(row["bytes"]), off)[0]

    for off in (0x260, 0x264, 0x270, 0x290):
        print("craft+0x%x" % off, [(round(r["frames"]), word(r, off)) for r in rows[:4]])
    if args.dump:
        args.dump.write_text(json.dumps(rows))


if __name__ == "__main__":
    main()
