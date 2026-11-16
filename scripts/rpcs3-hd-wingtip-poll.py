#!/usr/bin/env python3
"""Watch HD's wingtip vortices live on RPCS3: every craft's two `wingtip` nodes, their trigger and their ribbon.

    source data/scratch/<lane>/env.sh
    OAG_RPCS3_ATTACH=1 UV_CACHE_DIR=~/.cache/uv UV_OFFLINE=1 \\
    uv run --with evdev python3 scripts/rpcs3-hd-wingtip-poll.py --seconds 60 --out <dir> [--hold-cross] [--shots N]

Attached to a `serve` in a running race (`emu-restore-state.sh <lane> hd-race-talons-venom-pack`). Finds every
`ShipWingtipVortices` node (vtable `0x0086a340`, `+0xec` one of the eight craft at `0x0098d7c0`) once, then
samples without pausing, per node: the timer `+0xe4`, the ribbon handle `+0xe0`, the side byte `+0xe8`; per craft
entry (`ship+0x5fac`): the pad force time `+0x2e8`, the flag word `+0x264`, the forward speed `+0x340`; per held
ribbon element: the strength `+0x60` and phase `+0x10`. On each rise of a node's timer from zero it dumps the
node's world matrix (`+0x38`) and, a few samples later, the newest ribbon node of each of the element's chains
(`element+0x18`, oldest/newest pairs): position `+0x30`, velocity `+0x40`, life `+0x60`, width `+0x68`.
Law and offsets: `docs/ghidra/functions/ps3-hdfury-eu/wingtip-vortices.md`.

Writes `<out>/samples.json` and `<out>/dumps.json`; `--shots N` grabs a root-window frame at the first N rises.
Raw reads are game memory and stay under `data/`.
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

NODE_VTABLE = 0x0086A340


def find_nodes(m, ships, lo=0x30000000, hi=0x38000000, chunk=0x100000):
    needle = struct.pack(">I", NODE_VTABLE)
    out = []
    for base in range(lo, hi, chunk):
        try:
            block = m.rd(base, chunk + 0x100)
        except OSError:
            continue
        at = block.find(needle)
        while 0 <= at < chunk:
            if at % 4 == 0:
                craft = struct.unpack_from(">I", block, at + 0xEC)[0]
                if craft in ships:
                    out.append((base + at, craft))
            at = block.find(needle, at + 4)
    return out


def chain_dump(m, element):
    chains = m.u32(element + 0x44)
    table = m.u32(element + 0x18)
    out = []
    for i in range(min(chains, 8)):
        oldest, newest = struct.unpack(">II", m.rd(table + 8 * i, 8))
        if not newest:
            out.append(None)
            continue
        blob = m.rd(newest, 0x78)
        out.append({
            "node": newest,
            "rows": [list(struct.unpack_from(">4f", blob, 16 * r)) for r in range(4)],
            "vel": list(struct.unpack_from(">4f", blob, 0x40)),
            "alpha_u": list(struct.unpack_from(">4f", blob, 0x50)),
            "life": list(struct.unpack_from(">2f", blob, 0x60)),
            "width": struct.unpack_from(">f", blob, 0x68)[0],
        })
    return {"element": element, "chains": chains, "phase": m.f32(element + 0x10),
            "strength": m.f32(element + 0x60), "newest": out}


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--image", default="/home/topaxi/projects/OpenAntiGrav/data/images/hdfury-ps3-eu-dec.iso")
    ap.add_argument("--out", required=True)
    ap.add_argument("--seconds", type=float, default=60.0)
    ap.add_argument("--hold-cross", action="store_true")
    ap.add_argument("--shots", type=int, default=0)
    ap.add_argument("--dumps", type=int, default=12, help="stop dumping chains after this many")
    ap.add_argument("--wait", type=float, default=0.0, help="rescan every 3 s for up to this long until a race's nodes exist")
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    with drive.Session(args.image, str(out / "logs")) as session:
        m = hf.Mem(session.open_mem())
        waited = time.time()
        while True:
            ships = [s for s in struct.unpack(">8I", m.rd(hf.CRAFT_ARRAY, 32)) if s]
            nodes = find_nodes(m, set(ships)) if ships else []
            if nodes or time.time() - waited > args.wait:
                break
            print("no wingtip nodes yet", flush=True)
            if session.guard is not None:
                session.guard.stage("wait-race")
                session.guard.beat()
            time.sleep(3.0)
        if not nodes:
            sys.exit("no wingtip nodes: not in a race")
        player = next(r["ship"] for r in hf.field_rows(m) if r["player"])
        print("%d craft, %d wingtip nodes; player 0x%08x" % (len(ships), len(nodes), player), flush=True)
        for node, craft in nodes:
            print("  node 0x%08x craft 0x%08x side %d" % (node, craft, m.rd(node + 0xE8, 1)[0]), flush=True)
        if args.hold_cross:
            session.pad.set("cross", True)
        rows = []
        dumps = []
        pending = []
        prev = {node: 0.0 for node, _ in nodes}
        shots = 0
        t0 = time.time()
        while time.time() - t0 < args.seconds:
            host = time.time() - t0
            if session.guard is not None:
                session.guard.stage("capture")
                session.guard.beat()
            sample = {"t": host, "nodes": {}}
            for node, craft in nodes:
                blob = m.rd(node + 0xE0, 12)
                handle, timer = struct.unpack_from(">If", blob, 0)
                side = blob[8]
                entry = m.u32(craft + 0x5FAC)
                e = m.rd(entry + 0x264, 4)
                flags = struct.unpack(">I", e)[0]
                pad = m.f32(entry + 0x2E8)
                speed = m.f32(entry + 0x340)
                row = {"craft": craft, "player": craft == player, "side": side, "handle": handle, "timer": timer,
                       "pad": pad, "flags": flags, "speed": speed, "state": m.u32(craft + 0x5FA4),
                       "clock": m.f32(craft + 0x7014)}
                if handle:
                    row["strength"] = m.f32(handle + 0x60)
                    row["phase"] = m.f32(handle + 0x10)
                sample["nodes"][hex(node)] = row
                if timer > prev[node] + 0.05:
                    print("t %.2f node 0x%08x (%s side %d) rise %.3f -> %.3f pad %.3f flags 0x%x speed %.1f" % (
                        host, node, "player" if craft == player else "craft 0x%08x" % craft, side, prev[node], timer,
                        pad, flags, speed), flush=True)
                    if len(dumps) < args.dumps:
                        pending.append((host + 0.15, node))
                    if shots < args.shots:
                        shots += 1
                        drive.screenshot(str(out / ("rise%02d.png" % shots)))
                prev[node] = timer
            for due, node in list(pending):
                if host >= due:
                    pending.remove((due, node))
                    handle = m.u32(node + 0xE0)
                    if handle:
                        d = chain_dump(m, handle)
                        d["t"] = host
                        d["wingtip"] = node
                        d["matrix"] = [list(struct.unpack(">4f", m.rd(node + 0x38 + 16 * r, 16))) for r in range(4)]
                        d["side"] = m.rd(node + 0xE8, 1)[0]
                        d["speed"] = m.f32(m.u32(m.u32(node + 0xEC) + 0x5FAC) + 0x340)
                        d["timer"] = m.f32(node + 0xE4)
                        dumps.append(d)
            rows.append(sample)
        if args.hold_cross:
            session.pad.release_all()
    (out / "samples.json").write_text(json.dumps(rows))
    (out / "dumps.json").write_text(json.dumps(dumps, indent=1))
    print("%d samples over %.1f s host, %d chain dumps" % (len(rows), rows[-1]["t"] if rows else 0, len(dumps)),
          flush=True)


if __name__ == "__main__":
    main()
