#!/usr/bin/env python3
"""Log the start gantry's Mesh animation time (mesh+0x40) on the original, per frame.

Breaks on the `lwc1 f0, 0x40(s0)` inside `Model_GetAnimTime` (0x089128fc), where
`s0` is the `Mesh` node and the CPU has not yet loaded the field, so the node's
`+0x40` is the value `RaceManager_Update` (0x08829778) is about to compare with
its window. The caller is the saved `ra` at `sp+0x48`. Every hit is one row:
tick-relative cycle, node, caller, `mesh+0x40`, the player craft's crossing
count (`craft+0xac8`).

With --force-n the crossing count is overwritten between phases (the CPU is
stopped at a hit), to read the `Board`, `FINAL LAP` and chequered windows
without flying a lap; the row's `n` column shows whether the write held.

    uv run --with websocket-client python3 scripts/psp-gantry-clock.py \\
        --port 45001 --out gantry.csv --hits 400 --force-n 1,LAPS-1,LAPS,LAPS+1
"""

import argparse
import csv
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger

GET_ANIM_TIME_LOAD = 0x089128FC
SHIP_UPDATE_CRAFT = 0x08849618
CRAFT_CROSSINGS = 0xAC8
LAPS = 0x08B30F9C
RACE_MANAGER_RANGE = (0x08829778, 0x08829E00)


def gpr(dbg):
    regs = dbg.call("cpu.getAllRegs")
    cat = next(c for c in regs["categories"] if c["name"] == "GPR")
    return dict(zip(cat["registerNames"], cat["uintValues"]))


def restart_to_description(dbg):
    if "Pause" not in dbg.state_name():
        dbg.press("start", duration=6)
        time.sleep(1.5)
    if "Pause" not in dbg.state_name():
        raise SystemExit("never reached the pause menu (state %r)" % dbg.state_name())
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
    end = time.time() + 90
    while time.time() < end:
        if "Description" in (dbg.state_name() or ""):
            return
        time.sleep(0.5)
    raise SystemExit("never reached the track description screen")


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--port", type=int, default=45001)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--hits", type=int, default=400, help="manager-side hits per phase")
    ap.add_argument("--force-n", default="", help="comma list of n to force, e.g. 1,LAPS-1,LAPS,LAPS+1")
    ap.add_argument("--shot-dir", type=Path, help="screenshot the X display at --shot-hit manager hits into each phase")
    ap.add_argument("--shot-hit", type=int, default=60)
    ap.add_argument("--no-restart", action="store_true")
    args = ap.parse_args()

    dbg = Debugger(args.port)
    dbg.resume()
    dbg.hold(cross=False)
    if not args.no_restart:
        restart_to_description(dbg)

    craft = None
    for _, _ in dbg.each_hit(SHIP_UPDATE_CRAFT, 1, timeout=60.0):
        craft = gpr(dbg)["a0"]
    laps = dbg.read_u32(LAPS)
    print("craft 0x%08x laps %d state %r" % (craft, laps, dbg.state_name()), file=sys.stderr)

    phases = [None]
    for tok in [t for t in args.force_n.split(",") if t]:
        phases.append(eval(tok, {}, {"LAPS": laps}))

    dbg.resume()
    dbg.press("cross", duration=6)
    dbg.hold(cross=False)
    dbg.brk()
    dbg.add_breakpoint(GET_ANIM_TIME_LOAD)
    rows = []
    t0 = None
    player = craft
    for pi, force in enumerate(phases):
        mgr_hits = 0
        first = True
        while mgr_hits < args.hits:
            dbg.call("cpu.status")
            dbg.pending = []
            dbg.call("cpu.resume")
            dbg.wait_for_break(GET_ANIM_TIME_LOAD, timeout=60.0)
            cyc = dbg.call("cpu.status")["ticks"]
            if t0 is None:
                t0 = cyc
            r = gpr(dbg)
            node = r["s0"]
            ra = dbg.read_u32(r["sp"] + 0x48)
            val = dbg.read_f32(node + 0x40)
            from_mgr = RACE_MANAGER_RANGE[0] <= ra < RACE_MANAGER_RANGE[1]
            if from_mgr:
                # Model_GetAnimTime saved the caller's s0 (the race manager) at sp+0x40;
                # the window test reads manager->player (+0x2c0) -> +0xac8.
                manager = dbg.read_u32(r["sp"] + 0x40)
                player = dbg.read_u32(manager + 0x2C0)
            n = dbg.read_u32(player + CRAFT_CROSSINGS)
            if from_mgr:
                mgr_hits += 1
                if force is not None:
                    dbg.write_u32(player + CRAFT_CROSSINGS, force & 0xFFFFFFFF)
                    first = False
            if args.shot_dir and from_mgr and mgr_hits == args.shot_hit:
                args.shot_dir.mkdir(parents=True, exist_ok=True)
                subprocess.run(["import", "-window", "root", str(args.shot_dir / ("phase%d_n%s.png" % (pi, force)))], check=False)
            rows.append(
                dict(player='0x%08x' % player, craft='0x%08x' % craft, phase=pi, force="" if force is None else force, cycle=cyc - t0,
                     node="0x%08x" % node, ra="0x%08x" % ra, mesh40=round(val, 5), n=n,
                     mgr=int(from_mgr), state=dbg.state_name())
            )
    dbg.remove_breakpoint(GET_ANIM_TIME_LOAD)
    dbg.resume()
    dbg.close()
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    print("%d rows -> %s" % (len(rows), args.out))


if __name__ == "__main__":
    main()
