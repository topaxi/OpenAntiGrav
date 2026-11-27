#!/usr/bin/env python3
"""Fire a weapon from a loaded save state and record the GE frame K updates later (Pulse USA).

    . <instance>/env.sh
    uv run --with websocket-client scripts/psp-ge-fire-dump.py PORT STATE.ppst OUTDIR K [K...] --weapon-bit 0x10000

The state is reloaded for every K, so each dump is the same run of the game up to that point. The shot is a bit
written into the player's fire word at `Weapons_DispatchFire`'s first entry after the load; `0x10000` fires the
Repulser, `0x8` the Quake (`docs/ghidra/functions/psp-pulse-usa/mine.md`, "Weapon_RequestFire's jump table read as a table"). Writes `OUTDIR/kNNN.ppdmp`, derived
game data: keep it under data/, never commit it.
"""

import argparse
import base64
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
from ppsspp_debugger import Debugger  # noqa: E402

DISPATCH_FIRE = 0x08861814
# Off `Weapons_DispatchFire`'s first argument: the player's record, then its fire word and held word.
PLAYER_RECORD, FIRE_WORD, HELD_WORD = 0x70, 0x1B8, 0x1BC


def first_argument(dbg):
    regs = dbg.call("cpu.getAllRegs")
    gpr = next(c for c in regs["categories"] if c["name"] == "GPR")
    return dict(zip(gpr["registerNames"], gpr["uintValues"]))["a0"]


def run_to_update(dbg, k, bit):
    """Stop at the K-th `Weapons_DispatchFire` after the one the shot is written at."""
    dbg.brk()
    dbg.add_breakpoint(DISPATCH_FIRE)
    try:
        for hit in range(k + 1):
            dbg.pending = []
            dbg.call("cpu.resume")
            dbg.wait_for_break(DISPATCH_FIRE, timeout=60)
            if hit == 0:
                record = first_argument(dbg) + PLAYER_RECORD
                dbg.write_u32(record + HELD_WORD, 0xFFFFFFFF)
                dbg.write_u32(record + FIRE_WORD, dbg.read_u32(record + FIRE_WORD) | bit)
    finally:
        if not dbg.is_stepping():
            dbg.brk()
        dbg.remove_breakpoint(DISPATCH_FIRE)


def record_frame(dbg):
    ticket = dbg.send("gpu.record.dump")
    dbg.call("cpu.resume")
    deadline = time.time() + 90
    while time.time() < deadline:
        message = dbg._recv()
        if message is not None and message.get("ticket") == ticket:
            if message.get("event") == "error":
                raise SystemExit(message.get("message"))
            return base64.b64decode(message["uri"].split(",", 1)[1])
    raise SystemExit("no reply to gpu.record.dump")


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("port", type=int)
    parser.add_argument("state", type=Path)
    parser.add_argument("outdir", type=Path)
    parser.add_argument("k", type=int, nargs="+")
    parser.add_argument("--weapon-bit", type=lambda v: int(v, 0), required=True)
    args = parser.parse_args()
    args.outdir.mkdir(parents=True, exist_ok=True)
    for k in args.k:
        subprocess.run(
            ["python3", str(ROOT / "scripts/psp-state.py"), "--port", str(args.port), "load", "1",
             "--file", str(args.state.resolve()), "--stop-at", hex(DISPATCH_FIRE)],
            check=True, stdout=subprocess.DEVNULL,
        )
        with Debugger(args.port) as dbg:
            run_to_update(dbg, k, args.weapon_bit)
            path = args.outdir / f"k{k:03d}.ppdmp"
            path.write_bytes(record_frame(dbg))
        print(path, path.stat().st_size, flush=True)


if __name__ == "__main__":
    main()
