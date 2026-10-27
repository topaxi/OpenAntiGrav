#!/usr/bin/env python3
"""The time HD's scenery animation nodes run on, read live beside the HUD lap clock.

    source data/scratch/<lane>/env.sh
    OAG_RPCS3_ATTACH=1 UV_CACHE_DIR=~/.cache/uv UV_OFFLINE=1 \\
    uv run --with evdev python3 scripts/rpcs3-hd-world-clock.py [--samples 6] [--gap 1.0] [--json out.json]

Attached to a `serve` that sits in a race (cold boot, `scripts/emu-hd-glass-boot.sh`). Finds every
`MeshImporter` node (vtable `0x00869fd0`, `docs/ghidra/functions/ps3-hdfury-eu/gantry-clock.md`) once, then
reads, without pausing the game, each node's animation time (`+0xc0`) and the global clock stamp
(`+0x15c`, `clock+0x44` at its last advance) next to the player's lap clock (`ship+0x7014`). The nodes
that move sit at one common time (the scenery's); the gantry's own nodes sit inside one of the race
manager's windows and static ones at 0.0. Prints, per sample, the dominant non-gantry time, how many nodes
read it, the lap clock, and `time - lap clock` (the `world_clock_offset_s` of a pair, on the
`scripts/psp-world-anim-clock.py` convention: the offset over the HUD clock).
"""

import argparse
import importlib.util
import json
import struct
import sys
import time
from collections import Counter
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import rpcs3_hd_field as hf  # noqa: E402

_spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)

STAMP_OFFSET = 0x15C


def find_nodes(m, lo=0x30000000, hi=0x36000000, chunk=0x100000):
    needle = struct.pack(">I", hf.MESH_IMPORTER_VTABLE)
    nodes = []
    for base in range(lo, hi, chunk):
        try:
            block = m.rd(base, chunk)
        except OSError:
            continue
        at = block.find(needle)
        while at >= 0:
            if at % 4 == 0 and at + STAMP_OFFSET + 4 <= len(block):
                nodes.append(base + at)
            at = block.find(needle, at + 4)
    return nodes


def in_gantry_window(t):
    return any(a <= t < b for a, b in hf.GANTRY_WINDOWS)


def sample(m, nodes, clock_addr):
    """The lap clock, the global clock (the newest `+0x15c` stamp), and the non-gantry nodes grouped by `time - stamp`.

    A node that advances every frame has `time = global clock - (clock when it was last set to 0)`, so `time - stamp` is
    constant per node however the reads interleave with frames; the group's value is the scenery's offset below the global clock.
    """
    clock = m.f32(clock_addr)
    groups, stamps = Counter(), []
    for node in nodes:
        t = m.f32(node + hf.TIME_OFFSET)
        stamp = m.f32(node + STAMP_OFFSET)
        if stamp != 0.0:
            stamps.append(stamp)
        if t != 0.0 and stamp != 0.0 and not in_gantry_window(t):
            groups[round(t - stamp, 2)] += 1
    return clock, max(stamps) if stamps else None, groups


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--samples", type=int, default=6)
    ap.add_argument("--gap", type=float, default=1.0)
    ap.add_argument("--json")
    ap.add_argument("--log-dir", default=".")
    ap.add_argument("--image", default="/home/topaxi/projects/OpenAntiGrav/data/images/hdfury-ps3-eu-dec.iso")
    args = ap.parse_args()
    out = []
    with drive.Session(args.image, args.log_dir) as session:
        m = hf.Mem(session.open_mem())
        nodes = find_nodes(m)
        rows = hf.field_rows(m)
        player = next(r for r in rows if r["player"])
        clock_addr = player["ship"] + 0x7014
        print("%d MeshImporter nodes; player ship 0x%08x" % (len(nodes), player["ship"]), flush=True)
        for _ in range(args.samples):
            clock, glob, groups = sample(m, nodes, clock_addr)
            top = groups.most_common(3)
            print("lap clock %9.4f | global clock %s | time - stamp groups %s" % (clock, glob, top), flush=True)
            out.append({"lap_clock_s": clock, "global_clock_s": glob, "groups": top, "host_time": time.time()})
            time.sleep(args.gap)
    if args.json:
        Path(args.json).write_text(json.dumps(out, indent=1))


if __name__ == "__main__":
    main()
