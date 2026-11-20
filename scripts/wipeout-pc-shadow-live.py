#!/usr/bin/env python3
"""Omega's track shadow `S` on the `wipeout_pc` native port (1.07 eboot): one knockout and a read of its inputs.

Walks to a grid as `wipeout-pc-omega-pair.py` does. On the grid (a still before GO) it grabs frames alternately with the
shadow on and with it knocked out, then freezes the game (SIGSTOP) and reads the shadow state:

    OAG_PAD_SEAT=N uv run --with evdev python3 scripts/emu-run.py -- \\
        python3 scripts/wipeout-pc-shadow-live.py --out data/scratch/<lane>/runs/capital-a --game <game dir> --port <port checkout> \\
        --game-mode 2048 --track-steps 2 --hud-labels-ref <png> --hud-timer-ref <png>

The knockout: `Scene_PrepareFrame` builds `shadowCoeffs`' fourth row as `(c0, c1, -80, 0)` with `c0` and `c1` scaled by the two
literals `-80.0` and `-160.0` at `0x018632c0` and `0x018632c4`, read by that function alone. Writing both to 0 makes the row
`(0, 0, 0, 0)`, so the receiver's `exp2(clamp(c0 + c1 z, c2, c3))` is 1 and `S = min(1, pcf(map))`, and the map holds
`exp(80 d) >= 1`: `S = 1` on every receiver. The write is to this run's own process and is written back.

Read while frozen (guest addresses; host = `0x800000000 + guest - 0x01000000`): the shadow module's state block
`0x020e4800..0x020e5200` (cascade rectangles, splits, nearest caster), `Lighting.Sun direction` and `ShadowLight direction`
(`0x01eb47d0`, `0x01eb47e0`), the light view matrix `0x0221af60`, the camera view `0x0221afa0`, the pushed direction
`0x0221ac20`, the visible model list (`0x01edc810` count, `0x01edb240` indices, `0x01edf3e0 + i * 0x360` records: sphere
`+0xf0`, flags `+0x110`), and the frame-constant table entries of `shadowCoeffs`, `shadowMatrix`, `liveLighting0direction`
and `shadowLightDirection` (entry `~crc32(name)`, value pointer at `+0x10`).

`docs/ghidra/functions/ps4-omega-eu/track-shadow.md` has the results.
"""

import argparse
import importlib.util
import json
import os
import signal
import struct
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sheen_spec = importlib.util.spec_from_file_location("sheen", HERE / "wipeout-pc-road-sheen-live.py")
sheen = importlib.util.module_from_spec(sheen_spec)
sheen_spec.loader.exec_module(sheen)
pp, el = sheen.pp, sheen.el

GUEST = 0x01000000
KNOCK = 0x018632C0
KNOCK_EXPECT = bytes.fromhex("0000a0c2000020c3")
FRAME = {"shadowCoeffs": 16, "shadowMatrix": 16, "liveLighting0direction": 4, "shadowLightDirection": 4}
BLOCKS = {"state": (0x020E4800, 0xA00), "sun_dir": (0x01EB47D0, 0x20), "light_view": (0x0221AF60, 0x40),
          "camera_view": (0x0221AFA0, 0x40), "pushed_dir": (0x0221AC20, 0x10), "push_k": (0x019A81D4, 4),
          "view_proj": (0x01EB3240, 0x40), "shadow_view": (0x01EB3280, 0x40), "second_gate": (0x020E4340, 4)}
REC, REC_BASE, REC_IDX, REC_N = 0x360, 0x01EDF3E0, 0x01EDB240, 0x01EDC810


#: `-160 * 1e4`: `c1` ten thousand times steeper, so `exp2(clamp(c0 + c1 z, -80, 0))` sits at the floor and `S` goes to 0 on
#: every receiver whose depth is positive: the frame shows which surfaces take `S` and how much light `S` gates.
DARK_C1 = -1.6e6
#: `Scene_BindFrameConstants`' bank: `*0x0204ab58`, `shadowCoeffs` at slot 104 (`0x10 + 104 * 0x28 = 0x1050`).
BANK, SLOT_COEFFS = 0x0204AB58, 0x1050


def bound_coeffs(fd):
    bank = struct.unpack("<Q", read(fd, host(BANK), 8))[0]
    ptr = struct.unpack("<Q", read(fd, bank + SLOT_COEFFS, 8))[0]
    raw = read(fd, ptr, 64)
    return [round(x, 4) for x in struct.unpack("<16f", raw)] if len(raw) == 64 else hex(ptr)


def host(guest):
    return pp.IMAGE_BASE + guest - GUEST


def read(fd, addr, n):
    try:
        return os.pread(fd, n, addr)
    except (OSError, OverflowError):
        return b""


def frame_constants(fd, hits):
    out = {}
    for name, count in FRAME.items():
        vals = {}
        for h in hits[name]:
            raw = read(fd, h + 0x10, 8)
            if len(raw) < 8:
                continue
            ptr = struct.unpack("<Q", raw)[0]
            if ptr < pp.IMAGE_BASE:
                continue
            data = read(fd, ptr, 4 * count)
            if len(data) == 4 * count:
                vals.setdefault(ptr, struct.unpack("<%df" % count, data))
        ptrs = [struct.unpack("<Q", read(fd, h + 0x10, 8))[0] for h in hits[name] if len(read(fd, h + 0x10, 8)) == 8]
        ptrs = [p for p in ptrs if p in vals]
        best = max(set(ptrs), key=ptrs.count) if ptrs else None
        out[name] = {"n_entries": len(ptrs), "ptr": hex(best) if best else None, "value": list(vals[best]) if best else None,
                     "others": [list(v) for p, v in list(vals.items())[:6] if p != best]}
    return out


def models(fd):
    n = struct.unpack("<I", read(fd, host(REC_N), 4))[0]
    idx = struct.unpack("<%dI" % min(n, 512), read(fd, host(REC_IDX), 4 * min(n, 512)))
    rows = []
    for i in idx:
        rec = read(fd, host(REC_BASE + i * REC), REC)
        if len(rec) < REC:
            continue
        sphere = struct.unpack_from("<4f", rec, 0xF0)
        flags = struct.unpack_from("<BBH", rec, 0x110)
        rows.append({"i": i, "sphere": [round(x, 3) for x in sphere], "flag110": flags[0], "w112": flags[2]})
    return {"count": n, "rows": rows}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--game", required=True)
    ap.add_argument("--port", required=True)
    ap.add_argument("--track-steps", type=int, default=2)
    ap.add_argument("--game-mode", default="2048")
    ap.add_argument("--class-steps", type=int, default=0)
    ap.add_argument("--hud-labels-ref", type=Path)
    ap.add_argument("--hud-timer-ref", type=Path)
    ap.add_argument("--load-wait", type=float, default=12)
    ap.add_argument("--load-wait2", type=float, default=14)
    ap.add_argument("--region-limit", type=int, default=8 << 30)
    ap.add_argument("--rounds", type=int, default=3)
    args = ap.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    rig = pp.Rig(out / "port", args.game, args.port, "shadow-live")
    if pp.pad_seat.own_pads(rig.seat):
        sys.exit("seat %s already has a pad" % rig.seat)
    note = {"args": {k: str(v) for k, v in vars(args).items()}}
    stopped, fd, knocked = [], None, False
    signal.signal(signal.SIGTERM, lambda *_: sys.exit("stopped by SIGTERM"))

    def save():
        (out / "note.json").write_text(json.dumps(note, indent=1))

    try:
        pick, seen = pp.capture_original(rig, args, out)
        note["grid"] = {"pick": str(pick), **seen}
        pid = el.game_pid(rig.pids["port"])
        if pid is None:
            sys.exit("STALLED: no descendant maps 0x800000000")
        fd = os.open("/proc/%d/mem" % pid, os.O_RDWR)
        have = read(fd, host(KNOCK), 8)
        note["knock_bytes"] = have.hex()
        if have != KNOCK_EXPECT:
            sys.exit("STALLED: 0x018632c0 holds %s, not the -80/-160 literals" % have.hex())
        shots, rows = [], []
        for r in range(args.rounds):
            shots.append(str(rig.grab(out / ("base-%d.png" % r))))
            os.pwrite(fd, KNOCK_EXPECT[:4] + struct.pack("<f", DARK_C1), host(KNOCK))
            knocked = True
            time.sleep(0.3)
            rows.append(("dark", bound_coeffs(fd)))
            shots.append(str(rig.grab(out / ("dark-%d.png" % r))))
            os.pwrite(fd, KNOCK_EXPECT, host(KNOCK))
            knocked, t_back = False, time.time()
            for k in range(8):
                shots.append(str(rig.grab(out / ("back-%d-%d-%03d.png" % (r, k, round(100 * (time.time() - t_back)))))))
                time.sleep(0.1)
            rows.append(("on", bound_coeffs(fd)))
            os.pwrite(fd, bytes(8), host(KNOCK))
            knocked = True
            time.sleep(0.3)
            shots.append(str(rig.grab(out / ("ko-%d.png" % r))))
            os.pwrite(fd, KNOCK_EXPECT, host(KNOCK))
            knocked = False
        note["shots"] = shots
        note["bound_rows"] = rows
        note["shots_timer"] = [round(pp.hud2048(rig, args, Path(s))[1], 3) for s in shots] if args.game_mode == "2048" else None
        os.kill(pid, signal.SIGSTOP)
        stopped.append(pid)
        rig.grab(out / "frozen.png")
        note["blocks"] = {k: read(fd, host(a), n).hex() for k, (a, n) in BLOCKS.items()}
        note["models"] = models(fd)
        save()
        t0 = time.time()
        pats = {n: struct.pack("<I", sheen.name_hash(n)) for n in FRAME}
        hits = sheen.scan(fd, pid, pats, args.region_limit)
        note["scan_seconds"] = round(time.time() - t0, 1)
        note["frame_constants"] = frame_constants(fd, hits)
        save()
        print("[shadow] done %s" % json.dumps(note["frame_constants"])[:600], flush=True)
    finally:
        if fd is not None and knocked:
            os.pwrite(fd, KNOCK_EXPECT, host(KNOCK))
        for pid in stopped:
            try:
                os.kill(pid, signal.SIGCONT)
            except ProcessLookupError:
                pass
        save()
        rig.stop()


if __name__ == "__main__":
    main()
