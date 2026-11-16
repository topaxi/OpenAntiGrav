#!/usr/bin/env python3
"""Pulse PSP: how long the original's AI field stays in craft-to-craft contact, one row a frame.

One stop a frame at `AiManager_Update` (`0x08834b14`), before any driver runs, so every body's contact ring
(`body+0x170`, stride `0x40`, count `body+0x370`) still holds what the last world step recorded: kind `0` from
`Body_RecordContact` (the world), kind `1` from `Body_RecordPairContact` (another craft). Per racer (the table at
`0x08b34420`, stride `0x370`: `+0x00` craft, `+0x04` driver; body `craft+0x794`) it logs the pose and velocity,
every ring record (kind, `+0x10` point, `+0x20` impulse, `+0x30` friction) and the driver's lane target (`+0x44`),
rail blend (`+0x48`), contact push (`+0x3b8`) and player flag (`+0xa8`). `docs/gameplay/ai.md`, "The field's
contact time against the original's".

`--restart` first restarts the race from the pause menu and dismisses the track description, so the log starts
in the countdown; nothing is held, so the player sits parked on the grid.

    uv run --with websocket-client python3 scripts/psp-ai-contact-field.py --port 45092 --restart \\
        --frames 2400 --out data/scratch/<lane>/orig/run1.jsonl
    python3 -I scripts/psp-ai-contact-field.py --summary data/scratch/<lane>/orig/run1.jsonl

Raw captures are derived game data: keep them under `data/`, never commit them.
"""

import argparse
import json
import math
import struct
import sys
import time
from pathlib import Path

AI_MANAGER_UPDATE = 0x08834B14
RACERS = 0x08B34420
RACER_STRIDE = 0x370
RACER_COUNT = 0x08B35FA0
CLOCK = 0x08B344B0
G_WEAPONS = 0x08B30FA8
G_SKILL_LEVEL = 0x08B31044
CRAFT_BODY = 0x794
BODY_SPAN = 0x3A0
DRIVER_SPAN = 0x3C0
RING = 0x170
RING_COUNT = 0x370
RECORD = 0x40


def f32(blob, at, n=1):
    v = struct.unpack_from("<%df" % n, blob, at)
    return list(v) if n > 1 else v[0]


def u32(blob, at):
    return struct.unpack_from("<I", blob, at)[0]


def restart(dbg):
    import importlib.util

    here = Path(__file__).resolve().parent
    spec = importlib.util.spec_from_file_location("psp_drive", here / "psp-drive.py")
    drive = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(drive)
    import input_script

    dbg.resume()
    dbg.hold(**input_script.button_payload(input_script.State()))
    dbg.analog(0.0, 0.0)
    if "Pause" not in dbg.state_name():
        dbg.press("start", duration=6)
        time.sleep(1.5)
    if "Pause" not in dbg.state_name():
        raise SystemExit("never reached the pause menu (%r)" % dbg.state_name())
    for _ in range(4):
        dbg.press("down", duration=4)
        time.sleep(0.35)
    for _ in range(6):
        dbg.press("cross", duration=6)
        time.sleep(1.5)
        if "Pause" not in dbg.state_name():
            break
    else:
        raise SystemExit("RESTART RACE never took")
    time.sleep(drive.RESTART_TO_DESCRIPTION)
    dbg.press("cross", duration=6)
    time.sleep(0.5)
    print("restarted, description dismissed", file=sys.stderr)


def capture(args):
    here = Path(__file__).resolve().parent
    sys.path.insert(0, str(here))
    from ppsspp_debugger import Debugger

    dbg = Debugger(args.port)
    if args.restart:
        restart(dbg)
    out = open(args.out, "w")
    head = None
    players = None
    started = time.time()
    for index, _ in dbg.each_hit(AI_MANAGER_UPDATE, args.frames, timeout=30):
        if head is None:
            head = {"weapons": dbg.read(G_WEAPONS, 1)[0], "skill_level": dbg.read_u32(G_SKILL_LEVEL),
                    "racers": dbg.read_u32(RACER_COUNT)}
            out.write(json.dumps({"head": head}) + "\n")
            print("head %s" % head, file=sys.stderr)
        count = head["racers"]
        if index == 0 or not args.lean:
            table = dbg.read(RACERS, RACER_STRIDE * count)
            pointers = []
            for i in range(count):
                craft = u32(table, i * RACER_STRIDE)
                driver = u32(table, i * RACER_STRIDE + 4)
                pointers.append((craft, driver, dbg.read_u32(craft + CRAFT_BODY)))
        rows = []
        for i in range(count):
            craft, driver, body = pointers[i]
            b = dbg.read(body, BODY_SPAN)
            d = dbg.read(driver, DRIVER_SPAN) if not args.lean or index == 0 else bytes(DRIVER_SPAN)
            n = min(u32(b, RING_COUNT), 8)
            ring = []
            for k in range(n):
                at = RING + k * RECORD
                ring.append([u32(b, at), f32(b, at + 0x10, 3), f32(b, at + 0x20, 3), f32(b, at + 0x30)])
            rows.append({
                "i": i, "body": body, "left": f32(b, 0x00, 3), "fwd": f32(b, 0x20, 3), "pos": f32(b, 0x30, 3),
                "vel": f32(b, 0x140, 3), "ring": ring, "target": f32(d, 0x44), "blend": f32(d, 0x48),
                "push": f32(d, 0x3B8), "player": players[i] if players else d[0xA8],
                "progress": f32(table, i * RACER_STRIDE + 0x240),
            })
        if players is None:
            players = [r["player"] for r in rows]
        out.write(json.dumps({"f": index, "clock": dbg.read_f32(CLOCK), "r": rows}) + "\n")
        if index % 300 == 0:
            print("frame %d (%.0f s)" % (index, time.time() - started), file=sys.stderr, flush=True)
    out.close()
    dbg.resume()


def summary(path, window):
    """Per-slot contact frames from the first moving frame (GO), over `window` frames."""
    head, frames = None, []
    for line in open(path):
        rec = json.loads(line)
        if "head" in rec:
            head = rec["head"]
        else:
            frames.append(rec)
    go = next(k for k, fr in enumerate(frames)
              if any(math.hypot(*r["vel"]) > 5.0 for r in fr["r"] if not r["player"]))
    span = frames[go:go + window]
    print("head %s; GO at logged frame %d, window %d of %d frames" % (head, go, window, len(span)))
    per = {}
    for fr in span:
        for r in fr["r"]:
            s = per.setdefault(r["i"], {"player": r["player"], "pair": 0, "wall": 0, "records": 0,
                                        "push_moves": 0, "blend_drop": 0})
            kinds = [rec[0] for rec in r["ring"]]
            if 1 in kinds:
                s["pair"] += 1
                s["records"] += kinds.count(1)
                if r["blend"] == 0.0:
                    s["blend_drop"] += 1
            if 0 in kinds:
                s["wall"] += 1
            if abs(r["push"]) > 1e-6:
                s["push_moves"] += 1
    ai = [s for s in per.values() if not s["player"]]
    for i, s in sorted(per.items()):
        print("racer %d%s: pair-contact frames %d (records %d, rail blend 0 on %d), wall frames %d, push != 0 on %d"
              % (i, " (player)" if s["player"] else "", s["pair"], s["records"], s["blend_drop"], s["wall"],
                 s["push_moves"]))
    print("AI mean pair-contact frames per craft: %.1f" % (sum(s["pair"] for s in ai) / max(len(ai), 1)))


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--port", type=int, default=45092)
    p.add_argument("--restart", action="store_true")
    p.add_argument("--lean", action="store_true",
                   help="read only the bodies after the first frame (pointers cached; target, blend, push, progress "
                   "and the player flag read 0 after frame 0), about twice the frame rate")
    p.add_argument("--frames", type=int, default=2400)
    p.add_argument("--out", type=Path)
    p.add_argument("--summary", type=Path)
    p.add_argument("--window", type=int, default=1800)
    args = p.parse_args()
    if args.summary:
        summary(args.summary, args.window)
    else:
        capture(args)


if __name__ == "__main__":
    main()
