#!/usr/bin/env python3
"""Read, or turn off, Wipeout HD's Pilot Assist option on the live `rpcs3-drive.py serve` session.

    source data/scratch/<lane>/env.sh
    python3 scripts/rpcs3-hd-pilot-assist.py          # prints the option byte
    python3 scripts/rpcs3-hd-pilot-assist.py --off    # writes 00 00 over +0x473 and +0x474

The option lives in the save, so a profile that has it on draws the indicator's bracket under every craft of a
race frame, where the `hd-fury` compare profile renders ours with `--pilot-assist off`. Write it off before
`rpcs3-drive.py restart`: the race the restart builds and the HUD both read the byte afresh
(`docs/ghidra/functions/ps3-hdfury-eu/pilot-assist.md`). The save is not written.
"""
import argparse
import importlib.util
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import rpcs3_hd_field as field  # noqa: E402

spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--off", action="store_true", help="write the option off (both bytes, as `rpcs3-hd-autopilot.py` does)")
    args = ap.parse_args()
    info = drive.live_session()
    if not info:
        sys.exit("no live serve session")
    m = field.Mem(drive.RemoteMem(info["socket"]))
    at = field.pilot_assist_at(m)
    if at is None:
        sys.exit("no options block yet")
    before = m.rd(at, 2)
    if args.off:
        m.wr(at, b"\x00\x00")
    print("pilot assist %s%s" % (before.hex(), " -> %s" % m.rd(at, 2).hex() if args.off else ""))


if __name__ == "__main__":
    main()
