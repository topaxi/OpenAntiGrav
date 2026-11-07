#!/usr/bin/env python3
"""Poll HD's weapon pads' cooldown timers live on RPCS3, while a race runs, without pausing it.

    source data/scratch/<lane>/env.sh
    OAG_RPCS3_ATTACH=1 UV_CACHE_DIR=~/.cache/uv UV_OFFLINE=1 \\
    uv run --with evdev python3 scripts/rpcs3-hd-weapon-pad-poll.py --seconds 120 --out <dir> [--start-race]

Attached to a `serve` that sits on a race's grid (`scripts/emu-hd-race-walk.sh`, or the Racebox walk in
`docs/gameplay/pickups.md`, "How long a pad shows grey"). Finds every `WeaponPad_Importer` (vtable `0x0086a730`,
`docs/ghidra/functions/ps3-hdfury-eu/pads.md`) once, then samples each pad's refresh timer (`+0x160`, float,
seconds) and its packed colour (`+0xf0`, `0xff3f3f3f` while cooling) next to the player's lap clock (`ship+0x7014`)
as fast as the memory channel answers (about 100 Hz). Writes `<out>/samples.json` and prints, per pad, every stamp
it saw: the value the timer jumped to, and how long (lap-clock seconds, host seconds) it then took to reach zero.

With `--start-race` it taps START RACE and holds throttle (cross) for the run, so the field's own craft and the
player's cross the pads. Hold nothing else: this measures a timer, not a physics law.
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
import rpcs3_hd_field as hf  # noqa: E402

_spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)

WEAPON_PAD_VTABLE = 0x0086A730
TIMER = 0x160
COLOUR = 0xF0
SPAN_LO, SPAN_HI = COLOUR, TIMER + 4
GREY = 0xFF3F3F3F


def find_pads(m, lo=0x30000000, hi=0x40000000, chunk=0x100000):
    needle = struct.pack(">I", WEAPON_PAD_VTABLE)
    pads = []
    for base in range(lo, hi, chunk):
        try:
            block = m.rd(base, chunk)
        except OSError:
            continue
        at = block.find(needle)
        while at >= 0:
            if at % 4 == 0 and at + SPAN_HI <= len(block):
                pads.append(base + at)
            at = block.find(needle, at + 4)
    return pads


def stamps(series):
    """Every jump of the timer upward: `(first sample index, value, samples until it is zero again)`."""
    out = []
    prev = 0.0
    for i, (_, _, timer, _) in enumerate(series):
        if timer > prev + 1e-4:
            out.append([i, timer])
        prev = timer
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--image", default="/home/topaxi/projects/OpenAntiGrav/data/images/hdfury-ps3-eu-dec.iso")
    ap.add_argument("--out", required=True)
    ap.add_argument("--seconds", type=float, default=60.0)
    ap.add_argument("--start-race", action="store_true")
    ap.add_argument("--scan-only", action="store_true")
    ap.add_argument("--teleport-pad", type=int, help="before sampling, put the player on this pad (index in the scan)")
    ap.add_argument("--pin", type=float, default=0.0, help="keep rewriting the --teleport-pad position for this many host seconds")
    ap.add_argument("--empty-slot", action="store_true", help="empty the player's pickup slot first (a full slot is never tested against a pad)")
    ap.add_argument("--dump-pad", type=int, help="also keep this pad's whole 0x200-byte object per sample")
    ap.add_argument("--lift", type=float, default=3.4, help="height above the pad's origin for --teleport-pad")
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    with drive.Session(args.image, str(out / "logs")) as session:
        m = hf.Mem(session.open_mem())
        pads = find_pads(m)
        player = next(r for r in hf.field_rows(m) if r["player"])
        clock_addr = player["ship"] + 0x7014
        print("%d weapon pads; player ship 0x%08x" % (len(pads), player["ship"]), flush=True)
        for pad in pads:
            blob = m.rd(pad + SPAN_LO, SPAN_HI - SPAN_LO)
            colour = struct.unpack_from(">I", blob, 0)[0]
            timer = struct.unpack_from(">f", blob, TIMER - COLOUR)[0]
            print("  pad 0x%08x timer %.4f colour 0x%08x" % (pad, timer, colour), flush=True)
        if args.scan_only:
            return
        body = m.u32(m.u32(player["ship"] + 0x5FAC) + 0x270)
        slot = m.u32(player["ship"] + 0x5EDC)
        if args.empty_slot:
            m.wr(slot + 0x204, struct.pack(">ii", -1, -1))
        target = None
        if args.teleport_pad is not None:
            x, y, z = struct.unpack(">3f", m.rd(pads[args.teleport_pad] + 0x70, 12))
            # the pad's box is in its own frame (`+0x50` is its up axis, `+0x100`/`+0x110` the box)
            ux, uy, uz = struct.unpack(">3f", m.rd(pads[args.teleport_pad] + 0x50, 12))
            x, y, z = x + ux * args.lift, y + uy * args.lift, z + uz * args.lift
            target = struct.pack(">4f", x, y, z, 1.0)
            m.wr(body + 0x200, target)
            m.wr(body + 0x190, struct.pack(">3f", 0.0, 0.0, 0.0))
            print("player put at pad %d (%.1f, %.1f, %.1f)" % (args.teleport_pad, x, y, z), flush=True)
        if args.start_race:
            session.tap("cross", settle=1.0)
            session.pad.set("cross", True)
        rows = {pad: [] for pad in pads}
        clocks = []
        field = []
        pad_pos = [struct.unpack(">3f", m.rd(pad + 0x70, 12)) for pad in pads]
        dump = []
        step = 0
        t0 = time.time()
        while time.time() - t0 < args.seconds:
            host = time.time() - t0
            if args.empty_slot and host < args.pin:
                m.wr(slot + 0x204, struct.pack(">ii", -1, -1))
            if target is not None and host < args.pin:
                m.wr(body + 0x200, target)
                m.wr(body + 0x190, struct.pack(">3f", 0.0, 0.0, 0.0))
            step += 1
            if step % 25 == 0:
                snap = [(r["slot"], r["pos"]) for r in hf.field_rows(m)]
                field.append((host, snap))
            if args.dump_pad is not None:
                dump.append((host, m.rd(pads[args.dump_pad], 0x200).hex()))
            clock = m.f32(clock_addr)
            clocks.append((host, clock, struct.unpack(">3f", m.rd(body + 0x200, 12))))
            for pad in pads:
                blob = m.rd(pad + SPAN_LO, SPAN_HI - SPAN_LO)
                colour = struct.unpack_from(">I", blob, 0)[0]
                timer = struct.unpack_from(">f", blob, TIMER - COLOUR)[0]
                rows[pad].append((host, clock, timer, colour))
        if args.start_race:
            session.pad.release_all()
    (out / "samples.json").write_text(json.dumps({"pads": {hex(p): r for p, r in rows.items()}, "player": clocks, "dump": dump, "field": field, "pad_pos": pad_pos}))
    print("%d samples per pad over %.1f s host" % (len(clocks), clocks[-1][0]), flush=True)
    for pad, series in rows.items():
        for first, value in stamps(series):
            end = next((j for j in range(first, len(series)) if series[j][2] <= 0.0), None)
            grey = [s for s in series[first:end] if s[3] == GREY] if end else []
            print("pad 0x%08x stamp %.4f at lap clock %.3f; zero after %s lap-clock s, %s host s; "
                  "grey samples %d" % (
                      pad, value, series[first][1],
                      "%.3f" % (series[end][1] - series[first][1]) if end else "?",
                      "%.3f" % (series[end][0] - series[first][0]) if end else "?", len(grey)), flush=True)


if __name__ == "__main__":
    main()
