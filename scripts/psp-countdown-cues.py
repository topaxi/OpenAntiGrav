#!/usr/bin/env python3
"""Log every SCREAM cue the original starts across a race's countdown, on a tick axis.

Breaks on `Scream_StartSound` (`0x0898f864`), the one entry every cue start passes
through whether it came from `Scream_PlaySoundByName`, a positional emitter or a
command list's own `PlayChild`. At each hit the CPU is stopped, which is where
reads are cheap, so the hit costs the game nothing: it records the PSP cycle
counter, the bank's own name, the cue's index and name, the return address (who
asked) and the craft's throttle and lap clock.

The tick axis is anchored on the throttle: `craft+0x2b8` first reads non-zero at
the `Ship_UpdateCraft` entry that the docs number tick 272
(`docs/gameplay/race-modes.md#the-countdown-is-measured`). Phase 1 logs every cue
start until the anchor cue (`go`), phase 2 steps `Ship_UpdateCraft` hits until the
throttle rises (in a full grid every craft hits it, so the first hit that reads a
non-zero throttle anchors), and every row is reported as `272 + (cycle -
anchor_cycle) / cycles_per_frame`. The lap clock (`craft+0x920`) is *not* used: it
reads 0.0 for ticks after the release on that address. Breakpoints only arm while the CPU is stepping, and v1.20.4 fires only
the most recently added one, so the craft address is learned first from one
`Ship_UpdateCraft` hit and the StartSound breakpoint is armed afterwards.

    uv run --with websocket-client scripts/psp-countdown-cues.py --port 45195 \\
        --out timetrial.csv
"""

import argparse
import csv
import struct
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import PSP_CLOCK_HZ, Debugger

START_SOUND = 0x0898F864
SHIP_UPDATE_CRAFT = 0x08849618
CRAFT_THROTTLE = 0x2B8
CRAFT_LAP_CLOCK = 0x920
VBLANK_HZ = 59.94
CYCLES_PER_FRAME = PSP_CLOCK_HZ / VBLANK_HZ


def gpr(dbg):
    regs = dbg.call("cpu.getAllRegs")
    cat = next(c for c in regs["categories"] if c["name"] == "GPR")
    return dict(zip(cat["registerNames"], cat["uintValues"]))


def bank_names(dbg, bank):
    """`(bank name, {cue index: name})` off a loaded SBlk bank in emulated RAM."""
    if dbg.read_u32(bank) != 0x6B6C4253:
        return None, {}
    names = dbg.read_u32(bank + 0x38)
    if not 0x08000000 <= names < 0x0A000000:
        return "?", {}
    bank_name = dbg.read_cstring(names, 8)
    entries = dbg.read_u32(names + 0x08)
    table = {}
    if 0x08000000 <= entries < 0x0A000000:
        blob = dbg.read(entries, 0x14 * 128)
        for i in range(128):
            e = blob[i * 0x14 : (i + 1) * 0x14]
            if e[0] == 0:
                break
            table[struct.unpack_from("<H", e, 0x10)[0]] = (
                e[:16].split(b"\0")[0].decode("latin-1")
            )
    return bank_name, table


def restart_to_description(dbg):
    """Pause menu -> RESTART RACE, then wait for the track description screen.

    The same key sequence as `psp-drive.py restart`, which cannot be reused
    because it sleeps through the description and the countdown.
    """
    state = dbg.state_name()
    if "Pause" not in state:
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
    ap.add_argument("--port", type=int, default=45195)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument(
        "--after-release",
        type=float,
        default=4.0,
        help="seconds of emulated time to keep logging past the first sign of the release",
    )
    ap.add_argument(
        "--bp",
        type=lambda v: int(v, 0),
        default=START_SOUND,
        help="break here instead of Scream_StartSound, logging raw a0-a3 (with "
        "--cues-only), e.g. Scream_KeyOnVoice 0x0899456c",
    )
    ap.add_argument("--anchor-bank", default="SPEECH")
    ap.add_argument("--anchor-cue", type=int, default=12)
    ap.add_argument("--craft-ticks", type=int, default=30)
    ap.add_argument(
        "--cues-only",
        type=int,
        metavar="FRAMES",
        help="log cue starts only, until FRAMES emulated frames after the first "
        "InGame-state cue, and print frames relative to that cue. Skips the "
        "throttle anchor, so a run is +-1 frame; for a mode whose anchor cue is "
        "not known.",
    )
    ap.add_argument("--quiet-craft", action="store_true",
                    help="print only the cues and the first craft rows past the release")
    ap.add_argument(
        "--no-restart",
        action="store_true",
        help="the emulator is already sitting on the track description screen",
    )
    args = ap.parse_args()

    dbg = Debugger(args.port)
    dbg.resume()

    dbg.hold(cross=False)
    if not args.no_restart:
        restart_to_description(dbg)

    # The craft address, from one Ship_UpdateCraft hit.
    craft = None
    for _, _ in dbg.each_hit(SHIP_UPDATE_CRAFT, 1, timeout=60.0):
        craft = gpr(dbg)["a0"]
    print("craft 0x%08x, state %r" % (craft, dbg.state_name()), file=sys.stderr)

    dbg.brk()
    dbg.add_breakpoint(args.bp)
    dbg.hold(cross=True)
    dbg.call("cpu.resume")

    banks = {}
    rows = []
    start_cycle = None

    def record(msg_regs, cycle):
        if args.bp != START_SOUND:
            rows.append(
                dict(
                    cycle=cycle,
                    kind="raw",
                    bank="a0=0x%08x" % msg_regs["a0"],
                    cue_index="a1=0x%x" % msg_regs["a1"],
                    cue="a2=0x%x a3=0x%x" % (msg_regs["a2"], msg_regs["a3"]),
                    ra="0x%08x" % msg_regs["ra"],
                    pan="",
                    vol="",
                    throttle="",
                    clock="",
                    state=dbg.state_name(),
                )
            )
            return None, None
        bank = msg_regs["a0"]
        index = struct.unpack("<i", struct.pack("<I", msg_regs["a1"]))[0]
        if bank not in banks:
            banks[bank] = bank_names(dbg, bank)
        bname, table = banks[bank]
        rows.append(
            dict(
                cycle=cycle,
                kind="cue",
                bank=bname,
                cue_index=index,
                cue=table.get(index, "?"),
                ra="0x%08x" % msg_regs["ra"],
                pan=struct.unpack("<i", struct.pack("<I", msg_regs["a2"]))[0],
                vol=struct.unpack("<i", struct.pack("<I", msg_regs["a3"]))[0],
                throttle="",
                clock="",
                state=dbg.state_name(),
            )
        )
        return bname, index

    # Phase 1: every cue start, until the anchor cue.
    first_ingame = None
    while True:
        try:
            dbg.wait_for_break(args.bp, timeout=90.0)
        except TimeoutError:
            print("anchor cue never started", file=sys.stderr)
            break
        regs = gpr(dbg)
        cycle = dbg.call("cpu.status")["ticks"]
        bname, index = record(regs, cycle)
        if (bname, index) == (args.anchor_bank, args.anchor_cue):
            break
        if "Description" not in rows[-1]["state"] and first_ingame is None:
            first_ingame = cycle
        if (
            args.cues_only
            and first_ingame is not None
            and cycle > first_ingame + args.cues_only * CYCLES_PER_FRAME
        ):
            break
        dbg.call("cpu.resume")

    if args.cues_only:
        dbg.remove_breakpoint(args.bp)
        dbg.resume()
        dbg.hold(cross=False)
        dbg.close()
        for row in rows:
            rel = (row["cycle"] - first_ingame) / CYCLES_PER_FRAME
            print(
                "%8.2f %-14s %-8s %s ra=%s %s"
                % (rel, row["bank"], row["cue_index"], row["cue"] if row["kind"] == "raw" else "",
                   row["ra"], row["state"])
            )
        return

    # Phase 2: one row per Ship_UpdateCraft, the tick axis itself.
    dbg.remove_breakpoint(START_SOUND)
    dbg.add_breakpoint(SHIP_UPDATE_CRAFT)
    release_cycle = None
    for hit in range(args.craft_ticks):
        dbg.call("cpu.resume")
        dbg.wait_for_break(SHIP_UPDATE_CRAFT, timeout=30.0)
        cycle = dbg.call("cpu.status")["ticks"]
        hit_craft = gpr(dbg)["a0"]
        throttle = dbg.read_f32(hit_craft + CRAFT_THROTTLE)
        clock = dbg.read_f32(hit_craft + CRAFT_LAP_CLOCK)
        if release_cycle is None and throttle > 0.0:
            release_cycle = cycle
        rows.append(
            dict(cycle=cycle, kind="craft", bank="", cue_index="", cue="", ra="0x%08x" % hit_craft,
                 pan="", vol="", throttle=round(throttle, 3), clock=round(clock, 5),
                 state=dbg.state_name())
        )
    dbg.remove_breakpoint(SHIP_UPDATE_CRAFT)

    # Phase 3: cue starts after the release.
    if args.after_release > 0:
        dbg.add_breakpoint(START_SOUND)
        stop = release_cycle + args.after_release * PSP_CLOCK_HZ if release_cycle else None
        while True:
            dbg.call("cpu.resume")
            try:
                dbg.wait_for_break(START_SOUND, timeout=args.after_release + 5.0)
            except TimeoutError:
                break
            regs = gpr(dbg)
            cycle = dbg.call("cpu.status")["ticks"]
            record(regs, cycle)
            if stop is not None and cycle > stop:
                break
        dbg.remove_breakpoint(START_SOUND)
    dbg.resume()
    dbg.hold(cross=False)
    dbg.close()

    if release_cycle is None:
        print("never saw the race clock leave 0 - no tick anchor", file=sys.stderr)
    for row in rows:
        row["tick"] = (
            "%.2f" % (272 + (row["cycle"] - release_cycle) / CYCLES_PER_FRAME)
            if release_cycle is not None
            else ""
        )
    craft_rows = [r for r in rows if r["kind"] == "craft"]
    print("craft hits recorded: %d" % len(craft_rows), file=sys.stderr)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    if args.quiet_craft:
        rising = [r for r in craft_rows if r["throttle"] not in ("", 0.0)][:2]
        rows = sorted(
            [r for r in rows if r["kind"] == "cue"] + craft_rows[:1] + rising,
            key=lambda r: r["cycle"],
        )
    for row in rows:
        print(
            "%8s %-5s %-10s %3s %-16s ra=%s pan=%s vol=%s thr=%s clk=%s %s"
            % (row["tick"], row["kind"], row["bank"], row["cue_index"], row["cue"],
               row["ra"], row["pan"], row["vol"], row["throttle"], row["clock"],
               row["state"]),
            file=sys.stderr,
        )


if __name__ == "__main__":
    main()
