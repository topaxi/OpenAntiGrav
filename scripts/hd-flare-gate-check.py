#!/usr/bin/env python3
"""Which of `EngineFlare_RenderTick`'s gates is closing before its fade math.

The corrected breakpoint trace (`scripts/hd-flare-rendertick-break.py`,
"Fifth session" on `docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`)
confirmed the function's fade math is never entered for a sampled craft -
zero hits at any of three checkpoints inside it, across 45 seconds of
active, visibly-flared racing - but not *why*. Three gates sit between the
function's entry and the fade math:

- `0x002a0974` (`beq cr7,0x002a1110`) - taken if the byte flag at
  `struct_base+0x494` (the shared tuning/settings block) is `0`. Branches to
  `0x002a1110`, the function's shared epilogue - not diagnostic on its own
  (the mistake the previous script's first version made), but this specific
  instruction, read for its `cr7` bits rather than just "was it reached", is.
- `0x002a0960` (`ble cr7,0x002a1110`) - taken if the count-style field at
  `craft+0x5fa4` is `<= 2` after `-4`, unsigned - which (see engine-trail.md
  "Fourth session") skips only for that field in `{4,5,6}`, not the
  `count >= 7` reading an earlier session assumed.
- `0x002a0b44` (`beq cr7,0x002a1210`) - **found live-tracing the first two**
  (both measured open, 5/5 samples across two runs) and then re-reading the
  stretch between the byte-flag gate and the fade math to see what was
  actually closing it. `FUN_006765e8()`'s return value decides this one; if
  false, execution goes to `0x002a1210` - a *different* code path entirely,
  not the shared epilogue - and the fade math (and its own further,
  `r10`-gated check at `0x002a0bb4`) is never reached at all.

All three instructions execute unconditionally on every call that reaches
them - reaching them proves nothing, since every call reaches them. What
settles it is the `cr7` condition-register bits *at that point*, read
directly rather than inferred from a separate memory poll (which cannot
know if this specific call's values matched what was polled a moment before
or after). This script arms one at a time as a breakpoint (see below for
why not all three together) and at each hit decodes `cr7`'s `EQ` (for a
`beq`) or `GT` (for the `ble`) bit to say whether that specific gate would
fire on that specific call.

**Needs `PPU Decoder: Interpreter (static)` in `config.yml`** - shared,
session-scoped edit, switch back after
([rpcs3-debugger.md](../docs/reverse-engineering/rpcs3-debugger.md#z0-breakpoints-fire-but-only-under-the-interpreter)).

    uv run --with evdev python3 scripts/hd-flare-gate-check.py [out_dir] [byte|count|alt]

The optional third argument arms only one breakpoint - **do this by
default**, not the all-at-once form: simultaneous breakpoints on this
connection proved markedly less stable in practice (runs with two or three
armed got zero hits before an immediate stub desync; single-breakpoint runs
on the same connection type reliably got a dozen or more hits first).
"""

import importlib.util
import json
import sys
import time
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
spec = importlib.util.spec_from_file_location(
    "rpcs3_drive", ROOT / "scripts" / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)
from rpcs3_debugger import Debugger, REG_CR, REG_PC  # noqa: E402

IMAGE = ROOT / "data/images/hdfury-ps3-eu-dec.iso"

BYTE_FLAG_BEQ = 0x002A0974      # taken (skips to epilogue) if struct_base+0x494 == 0
COUNT_FIELD_BLE = 0x002A0960    # taken (skips to epilogue) if craft+0x5fa4-4 <= 2 unsigned
# A third gate, found live-tracing the first two (both measured open, 5/5
# samples) and then re-disassembling the stretch between the byte-flag gate
# and the fade math to see what was actually skipping it. `FUN_006765e8()`'s
# result (`0x002a0b34`-`0x002a0b44`) branches to `0x002a1210` - a wholly
# separate code path, not the shared `0x002a1110` epilogue at all - when
# false; only when true does execution reach the fade math's own further
# gate at `0x002a0bb4`. This is the direct branch on that call's result.
ALT_PATH_BEQ = 0x002A0B44       # taken (skips fade math for 0x002a1210 instead) if FUN_006765e8() == 0
# Two simultaneous breakpoints proved markedly less stable than one in
# practice (two runs got zero hits before an immediate stub desync, where
# single-breakpoint runs on this same connection type reliably got a dozen
# or more) - arm one at a time via ONLY_TARGET rather than both together.
ONLY_TARGET = None  # set to one of the three constants above to arm just it
ALL_TARGETS = {
    BYTE_FLAG_BEQ: "byte_flag_beq",
    COUNT_FIELD_BLE: "count_field_ble",
    ALT_PATH_BEQ: "alt_path_beq",
}
TARGETS = {ONLY_TARGET: ALL_TARGETS[ONLY_TARGET]} if ONLY_TARGET else ALL_TARGETS


def cr7(regs):
    word = int.from_bytes(regs[REG_CR:REG_CR + 4], "big")
    nibble = word & 0xF
    return {"LT": bool(nibble & 0x8), "GT": bool(nibble & 0x4),
            "EQ": bool(nibble & 0x2), "SO": bool(nibble & 0x1)}


def read_pc_hit(dbg, targets):
    for tid in dbg.threads():
        regs = dbg.registers(tid)
        if regs is None:
            continue
        pc = int.from_bytes(regs[REG_PC:REG_PC + 8], "big")
        if pc in targets:
            return tid, pc, regs
    return None, None, None


def wait_at_any(dbg, targets, tries, slice_seconds):
    for _ in range(tries):
        dbg.resume()
        time.sleep(slice_seconds)
        dbg.pause()
        dbg.drain()
        hit = read_pc_hit(dbg, targets)
        if hit[0] is not None:
            return hit
    return None, None, None


def step_off_breakpoint(dbg, hit_addr, targets):
    dbg.remove_breakpoint(hit_addr)
    dbg.resume()
    time.sleep(0.05)
    dbg.pause()
    dbg.drain()
    hit = read_pc_hit(dbg, {a: n for a, n in targets.items() if a != hit_addr})
    dbg.add_breakpoint(hit_addr)
    return hit


def main():
    global TARGETS
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "/tmp/hd-flare-gate-check")
    if len(sys.argv) > 2:
        only = {"byte": BYTE_FLAG_BEQ, "count": COUNT_FIELD_BLE,
                "alt": ALT_PATH_BEQ}[sys.argv[2]]
        TARGETS = {only: ALL_TARGETS[only]}
    out.mkdir(parents=True, exist_ok=True)
    with drive.Session(str(IMAGE), str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        if not session.wait_for_screen("Main Menu", 220.0):
            print("never reached the Main Menu", file=sys.stderr)
            return 1
        time.sleep(12.0)
        if session.walk_to_race() not in drive.RACE_ARRIVED:
            print("did not reach a race", file=sys.stderr)
            return 1
        print("in race; waiting for the load", flush=True)
        time.sleep(50.0)
        session.pad.set("cross", True)
        time.sleep(16.0)

        dbg = Debugger()
        hits = Counter()
        taken = Counter()
        details = []
        stub_desync = False
        try:
            dbg.pause()
            for addr in TARGETS:
                dbg.add_breakpoint(addr)
            print("breakpoints armed: %s" % [hex(a) for a in TARGETS], flush=True)

            def record(tid, addr, regs):
                name = TARGETS[addr]
                hits[name] += 1
                flags = cr7(regs)
                if name == "count_field_ble":
                    would_skip = not flags["GT"]
                else:
                    # byte_flag_beq and alt_path_beq are both `beq` - EQ set
                    # means the branch (to the epilogue or to 0x002a1210
                    # respectively) is taken, i.e. the fade math is skipped.
                    would_skip = flags["EQ"]
                if would_skip:
                    taken[name] += 1
                entry = {"tid": tid, "which": name, "cr7": flags, "would_skip": would_skip}
                details.append(entry)
                print(entry, flush=True)

            deadline = time.time() + 45.0
            pending = None
            while time.time() < deadline and len(details) < 30:
                try:
                    if pending is not None:
                        tid, addr, regs = pending
                        pending = None
                    else:
                        tid, addr, regs = wait_at_any(dbg, TARGETS, tries=1, slice_seconds=0.15)
                    if addr is None:
                        continue
                    record(tid, addr, regs)
                    pending_hit = step_off_breakpoint(dbg, addr, TARGETS)
                    if pending_hit[0] is not None:
                        pending = pending_hit
                except (TimeoutError, OSError) as exc:
                    print("stub desync after %d hits: %r" % (len(details), exc),
                          file=sys.stderr, flush=True)
                    stub_desync = True
                    break
            if not stub_desync:
                for addr in TARGETS:
                    dbg.remove_breakpoint(addr)
                dbg.resume()
        finally:
            try:
                dbg.resume()
            except Exception:
                pass
            dbg.close()
        result = {
            "hits": dict(hits),
            "would_skip_counts": dict(taken),
            "details": details,
            "stub_desync": stub_desync,
        }
        (out / "meta.json").write_text(json.dumps(result, indent=1))
        if not stub_desync:
            session.pad.set("cross", False)
            drive.screenshot(out / "race.png")
        print(json.dumps(result, indent=1))
        print("done; artefacts in %s" % out, flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
