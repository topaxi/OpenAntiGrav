#!/usr/bin/env python3
"""Where a Zone race's target stage comes from, measured live on HD/Fury.

`Environment_UpdateStageBlend` (`0x003da540`) picks the pending stage
`H[n].+0x04` from one of four sources by `g_GameState.mode`
(zone-effectsettings-loader.md, tenth pass): mode `0xe` at `0x003dd5dc`
reads `RaceManager->+0x2e10`, `0xd` at `0x003dd364` reads
`*(RaceManager + 0x2dfc + n*4)`, `0x15` at `0x003dd3a8` reads
`*(RaceManager + 0x351c + n*4)`, and the fall-through at `0x003da650`
reads `craftArray[n]->+0x640` with `craftArray = *(0x008b7c00)`. The
static reading says Zone takes the fall-through, and
zone-speed-class-table.md names the writer of that field statically -
`Hud_UpdateZoneSpeedClass`'s `stw r3, 0x640(r29)` at `0x0004a014`, `r3 =
14 - i` for the speed-class record `i` the zone counter sits in - while a
`Z2` watch on the same field caught nothing in Campaign and Eliminator
races, and the loader page's thirty-third pass re-filed the writer as
"not on a craft". One Zone race, one boot, five breakpoints in turn:

1. `dispatch`  - `0x003da638`, the `cmpwi` right after `lwz r0, 0xe0(r9)`:
   `r0` is `g_GameState.mode`, `r31` is `n`. Which branch, from the value
   the branch is taken on.
2. `targets`   - each of the four branch targets armed in turn; which
   parks a thread and which never does. `0x003da650` is also reached by
   the `bne` at `0x003da62c` off the byte at `0x009384e1`, so the byte and
   the mode are read at every hit there to tell the two routes apart.
3. `source`    - `0x003da674`, the `stw r0, 0x4(r9)` of the fall-through:
   `r0` is the value just loaded, `r11` is `craftArray[n]`, so `T = r11 +
   0x640`; read alongside `H[n]` (`+0x00` current, `+0x04` pending before
   the store, radius, speed, weight). Bursts of consecutive frames
   (hopping, see `hd-flare-owner-break.py`'s `Breaker`), with the commit
   breakpoint below used as the free run between them.
4. `commit`    - `0x003da74c`, `stwx r8, r4, r9`: fires only when a new
   stage is pending, so waiting on it *is* waiting for the ladder to step,
   and `r8` is the value that lands in `H[n].+0x00` - the sanity control
   that what `T` held is what got committed. At each commit the PPU thread
   list is dumped with PC and LR.
5. `writer`    - `0x0004a014`: `r29` is the object being written, `r3` the
   value, `r28` the record index `i`, `f29` the zone counter the table was
   walked against. Pointer identity between `r29` and the source's `r11`
   in the same race is the proof the two pages describe one field.

Boots with `Session(interpreter=True)`: the muted `--config` copy with
`PPU Decoder: Interpreter (static)`, the only decoder `Z0` fires under.
One breakpoint armed at a time, never a resume onto an armed address.

    uv run --with evdev python3 scripts/hd-zone-stage-source-break.py zone-source [out_dir]
    uv run --with evdev python3 scripts/hd-zone-stage-source-break.py writer-chain [out_dir]

    uv run --with evdev python3 scripts/hd-zone-stage-source-break.py hud-writer [out_dir]

`writer-chain` is the second boot: the HUD writer is not reached every
frame, so it is waited on like the commit - arm, free-run until it parks
a thread - and the commit is then waited on right behind it, so each link
of the chain is read in order on one stage step: `r3` at the writer, `T`
after it, `r8` at the commit.
"""

import importlib.util
import json
import struct
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))


def _load(name, filename):
    spec = importlib.util.spec_from_file_location(name, ROOT / "scripts" / filename)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


drive = _load("rpcs3_drive", "rpcs3-drive.py")
flare = _load("hd_flare_owner_break", "hd-flare-owner-break.py")
from rpcs3_debugger import Debugger, REG_FPR, REG_LR, REG_PC  # noqa: E402

IMAGE = ROOT / "data/images/hdfury-ps3-eu-dec.iso"

# Environment_UpdateStageBlend (0x003da540)
DISPATCH = 0x003DA638        # cmpwi cr7,r0,0xe : r0 = g_GameState.mode, r31 = n
TARGET_DETONATOR = 0x003DD5DC  # mode 0xe  : RaceManager->+0x2e10
TARGET_MODE_D = 0x003DD364     # mode 0xd  : *(RaceManager + 0x2dfc + n*4)
TARGET_MODE_15 = 0x003DD3A8    # mode 0x15 : *(RaceManager + 0x351c + n*4)
TARGET_FALLBACK = 0x003DA650   # else      : craftArray[n]->+0x640
SOURCE_STORE = 0x003DA674      # stw r0,0x4(r9) : r0 = *(r11 + 0x640), r11 = craftArray[n]
COMMIT_STORE = 0x003DA74C      # stwx r8,r4,r9  : H[n].+0x00 = r8 (= H[n].+0x04)
TARGETS = (("detonator", TARGET_DETONATOR), ("mode_d", TARGET_MODE_D),
           ("mode_15", TARGET_MODE_15), ("fallback", TARGET_FALLBACK))

# Hud_UpdateZoneSpeedClass (0x00049718)
WRITER_STORE = 0x0004A014      # stw r3,0x640(r29) : r29 = craftArray[index], r3 = 14 - i

# FUN_0008ce10, the HUD's Zone ladder widget update (hud = r3 = r30): walks
# g_ZoneSpeedClassTable against the zone counter *(*(hud+0x40)+0x20) and
# stores 14 - i (r7, `subfic r7,r31,0xe`) or 14 outright into hud+0x640.
HUD_LADDER_ENTRY = 0x0008CE10
HUD_LADDER_STORES = (0x0008DD70, 0x0008D9F0, 0x0008DEF8)

G_GAMESTATE = 0x00936FE8       # *(0x008b7af4); .mode at +0xe0
MODE_ADDRESS = G_GAMESTATE + 0xE0
SKIP_BYTE = 0x009384E1         # *(0x008b7af8); set -> straight to the fallback
CRAFT_ARRAY = 0x0098767C       # *(0x008b7c00); two entries
STAGE_TABLE_SLOT = 0x008B7944  # -> 0x008c2cb8, H, 0x38-byte entries
STAGE_TABLE = flare.STAGE_TABLE
ENTRY_SIZE = 0x38

gpr, u32, s32, floats = flare.gpr, flare.u32, flare.s32, flare.floats


def fpr(regs, index):
    at = REG_FPR + index * 8
    return struct.unpack(">d", regs[at:at + 8])[0]


def stage_entry(dbg, n):
    blob = dbg.read(STAGE_TABLE + n * ENTRY_SIZE, ENTRY_SIZE)
    return {
        "current": u32(blob, 0), "pending": u32(blob, 4),
        "radius": floats(blob[8:12])[0], "speed": floats(blob[0x10:0x14])[0],
        "accel": floats(blob[0x14:0x18])[0], "weight": floats(blob[0x18:0x1C])[0],
        "gate_2c": blob[0x2C],
    }


def globals_readout(dbg):
    out = {
        "mode": u32(dbg.read(MODE_ADDRESS, 4)),
        "skip_byte": dbg.read(SKIP_BYTE, 1)[0],
        "stage_table": "%#x" % u32(dbg.read(STAGE_TABLE_SLOT, 4)),
        "craft_array": ["%#x" % u32(dbg.read(CRAFT_ARRAY + i * 4, 4)) for i in range(2)],
    }
    for i, craft in enumerate(out["craft_array"]):
        craft = int(craft, 16)
        if craft:
            out["craft%d_0x640" % i] = u32(dbg.read(craft + 0x640, 4))
    out["H"] = [stage_entry(dbg, n) for n in range(2)]
    return out


def dispatch_readout(dbg, regs):
    n = gpr(regs, 31) & 0xFFFFFFFF
    return {
        "mode_r0": gpr(regs, 0) & 0xFFFFFFFF,
        "n": n,
        "g_GameState_r9": "%#x" % (gpr(regs, 9) & 0xFFFFFFFF),
        "mode_mem": u32(dbg.read(MODE_ADDRESS, 4)),
        "skip_byte": dbg.read(SKIP_BYTE, 1)[0],
        "H": stage_entry(dbg, n) if n < 2 else None,
    }


def source_readout(dbg, regs):
    n = gpr(regs, 31) & 0xFFFFFFFF
    craft = gpr(regs, 11) & 0xFFFFFFFF
    entry_ptr = gpr(regs, 9) & 0xFFFFFFFF
    t = craft + 0x640
    return {
        "n": n,
        "craft_r11": "%#x" % craft,
        "T": "%#x" % t,
        "value_r0": gpr(regs, 0) & 0xFFFFFFFF,
        "T_mem": u32(dbg.read(t, 4)),
        "entry_r9": "%#x" % entry_ptr,
        "entry_expected": "%#x" % (STAGE_TABLE + n * ENTRY_SIZE),
        "H": stage_entry(dbg, n) if n < 2 else None,
        "mode_mem": u32(dbg.read(MODE_ADDRESS, 4)),
    }


def commit_readout(dbg, regs):
    base = gpr(regs, 4) & 0xFFFFFFFF
    offset = gpr(regs, 9) & 0xFFFFFFFF
    n = offset // ENTRY_SIZE
    craft = u32(dbg.read(CRAFT_ARRAY + n * 4, 4)) if n < 2 else 0
    out = {
        "n": n,
        "base_r4": "%#x" % base,
        "offset_r9": offset,
        "value_r8": gpr(regs, 8) & 0xFFFFFFFF,
        "H_before": stage_entry(dbg, n) if n < 2 else None,
        "craft": "%#x" % craft,
        "T_mem": u32(dbg.read(craft + 0x640, 4)) if craft else None,
    }
    return out


def writer_readout(dbg, regs):
    obj = gpr(regs, 29) & 0xFFFFFFFF
    widget = gpr(regs, 30) & 0xFFFFFFFF
    return {
        "object_r29": "%#x" % obj,
        "value_r3": gpr(regs, 3) & 0xFFFFFFFF,
        "record_i_r28": gpr(regs, 28) & 0xFFFFFFFF,
        "widget_r30": "%#x" % widget,
        "widget_index_0x154": u32(dbg.read(widget + 0x154, 4)) if widget else None,
        "zone_counter_f29": fpr(regs, 29),
        "old_0x640": u32(dbg.read(obj + 0x640, 4)) if obj else None,
        "craft_array": ["%#x" % u32(dbg.read(CRAFT_ARRAY + i * 4, 4)) for i in range(2)],
        "H0": stage_entry(dbg, 0),
    }


def thread_dump(dbg):
    rows = []
    for tid in dbg.threads():
        regs = dbg.registers(tid)
        if regs is None:
            rows.append({"tid": tid})
            continue
        rows.append({
            "tid": tid,
            "pc": "%#x" % int.from_bytes(regs[REG_PC:REG_PC + 8], "big"),
            "lr": "%#x" % int.from_bytes(regs[REG_LR:REG_LR + 8], "big"),
        })
    return rows


def collect(breaker, address, readout, want, budget, label, sink, save):
    """`want` consecutive hits at `address`, each passed through `readout`."""
    dbg = breaker.dbg
    deadline = time.time() + budget
    misses = 0
    while len(sink) < want and time.time() < deadline:
        tid, regs = breaker.stop_at(address, tries=4)
        if tid is None:
            misses += 1
            print("  %s: no thread reached %#x (%d)" % (label, address, misses), flush=True)
            if misses >= 3:
                break
            continue
        entry = readout(dbg, regs)
        entry["tid"] = tid
        entry["t"] = round(time.time(), 2)
        sink.append(entry)
        print("  %s %s" % (label, json.dumps(entry, sort_keys=True)), flush=True)
        save()
    return len(sink)


def probe_target(breaker, address, slices=8, slice_seconds=0.5):
    """Does any thread ever park at `address`? One arm, `slices` free runs."""
    dbg = breaker.dbg
    if breaker.parked == address:
        breaker.step_off(address)
    dbg.add_breakpoint(address)
    try:
        for index in range(slices):
            dbg.resume()
            if dbg.wait_for_stop(timeout=slice_seconds) is None:
                dbg.pause()
            dbg.drain()
            tid, regs = flare.find_at(dbg, address, breaker.prefer)
            if tid is not None:
                breaker.prefer = tid
                breaker.parked = address
                return index + 1, tid, regs
    finally:
        dbg.remove_breakpoint(address)
    return None, None, None


def wait_for_hit(breaker, address, budget, slice_seconds=1.0):
    """Arm `address` and free-run until a thread parks on it, or give up.

    For a site that runs rarely - the commit store fires once per stage
    step, the HUD class writer once per class change - the free run *is*
    the wait, and the slices are only there to look between them.
    """
    dbg = breaker.dbg
    if breaker.parked == address:
        breaker.step_off(address)
    dbg.add_breakpoint(address)
    started = time.time()
    try:
        while time.time() - started < budget:
            dbg.resume()
            if dbg.wait_for_stop(timeout=slice_seconds) is None:
                dbg.pause()
            dbg.drain()
            tid, regs = flare.find_at(dbg, address, breaker.prefer)
            if tid is not None:
                breaker.prefer = tid
                breaker.parked = address
                return time.time() - started, tid, regs
    finally:
        dbg.remove_breakpoint(address)
    return time.time() - started, None, None


def wait_for_commit(breaker, budget, slice_seconds=1.0):
    return wait_for_hit(breaker, COMMIT_STORE, budget, slice_seconds)


def boot_into_zone(session, out, result, save):
    """Boot, walk to a Racebox Zone race, START RACE, and let it settle."""
    if not flare.boot_to_main_menu(session):
        return False
    if not flare.walk_to_zone(session, out):
        print("did not reach a race (%s)" % drive.current_screen(), file=sys.stderr)
        return False
    result["race_type"] = flare.wait_for_load(session, out)
    if not result["race_type"] or "zone" not in result["race_type"].lower():
        print("not a Zone race - stopping here", file=sys.stderr)
        save()
        return False
    result["config"] = drive.config_report()
    # Zone waits on a `START RACE` prompt; then the countdown.
    session.pad.press("cross", 0.2)
    time.sleep(25.0)
    drive.screenshot(out / "race-running.png", trim=True)
    return True


def run_writer_chain(out, steps=4):
    """The chain, in order: HUD class writer -> `T` -> the blend's commit.

    The first run showed `0x0004a014` is not reached every frame (three
    misses of four 0.3 s slices each), so the writer is waited on the way
    the commit is: arm it, free-run until it parks a thread, read `r29`,
    `r3`, `f29` and `T` before and after the store, then arm the commit
    and wait for it to fire on the value just written.
    """
    result = {"globals": {}, "chain": []}

    def save():
        (out / "writer-chain.json").write_text(json.dumps(result, indent=1))

    with drive.Session(str(IMAGE), str(out / "logs"), interpreter=True) as session:
        if not boot_into_zone(session, out, result, save):
            return 1
        dbg = Debugger()
        breaker = flare.Breaker(dbg)
        try:
            dbg.pause()
            dbg.drain()
            result["globals"]["at_attach"] = globals_readout(dbg)
            print("globals: %s" % json.dumps(result["globals"]["at_attach"]), flush=True)
            save()
            for step in range(steps):
                link = {"step": step}
                print("== waiting on the writer at %#x (step %d)" % (WRITER_STORE, step),
                      flush=True)
                waited, tid, regs = wait_for_hit(breaker, WRITER_STORE, budget=300.0)
                link["writer_waited_s"] = round(waited, 1)
                link["writer_tid"] = tid
                if regs is None:
                    print("  no writer hit in %.0f s" % waited, flush=True)
                    result["chain"].append(link)
                    save()
                    continue
                link["writer"] = writer_readout(dbg, regs)
                link["writer"]["threads"] = thread_dump(dbg)
                breaker.step_off(WRITER_STORE)
                obj = int(link["writer"]["object_r29"], 16)
                link["writer"]["new_0x640"] = u32(dbg.read(obj + 0x640, 4))
                print("  writer %s" % json.dumps(
                    {k: v for k, v in link["writer"].items() if k != "threads"}), flush=True)
                drive.screenshot(out / ("writer-%d.png" % step), trim=True)
                print("== waiting on the commit at %#x" % COMMIT_STORE, flush=True)
                waited, tid, regs = wait_for_commit(breaker, budget=60.0)
                link["commit_waited_s"] = round(waited, 1)
                link["commit_tid"] = tid
                if regs is not None:
                    link["commit"] = commit_readout(dbg, regs)
                    breaker.step_off(COMMIT_STORE)
                    n = link["commit"]["n"]
                    link["commit"]["H_after"] = stage_entry(dbg, n) if n < 2 else None
                    print("  commit %s" % json.dumps(link["commit"]), flush=True)
                else:
                    print("  no commit in %.0f s" % waited, flush=True)
                link["source"] = []
                collect(breaker, SOURCE_STORE, source_readout, 6, 60.0, "source",
                        link["source"], save)
                result["chain"].append(link)
                save()
            result["globals"]["at_end"] = globals_readout(dbg)
            result["step_off"] = {"stepped": breaker.stepped, "ran_off": breaker.ran_off}
            save()
            dbg.resume()
        except (TimeoutError, OSError) as exc:
            print("stub desync: %r" % exc, file=sys.stderr, flush=True)
            result["desync"] = repr(exc)
        finally:
            try:
                dbg.resume()
            except Exception:
                pass
            dbg.close()
    save()
    return 0


def hud_entry_readout(dbg, regs):
    hud = gpr(regs, 3) & 0xFFFFFFFF
    state = u32(dbg.read(hud + 0x40, 4))
    return {
        "hud_r3": "%#x" % hud,
        "state_0x40": "%#x" % state,
        "zone": s32(dbg.read(state + 0x20, 4)) if state else None,
        "hud_0x640": u32(dbg.read(hud + 0x640, 4)),
        "hud_0x28c": "%#x" % u32(dbg.read(hud + 0x28C, 4)),
        "hud_0x6a8": s32(dbg.read(hud + 0x6A8, 4)),
        "craft_array": ["%#x" % u32(dbg.read(CRAFT_ARRAY + i * 4, 4)) for i in range(2)],
        "H0": stage_entry(dbg, 0),
    }


def hud_store_readout(dbg, regs, where):
    hud = gpr(regs, 30) & 0xFFFFFFFF
    state = u32(dbg.read(hud + 0x40, 4))
    return {
        "at": "%#x" % where,
        "hud_r30": "%#x" % hud,
        "value_r7": gpr(regs, 7) & 0xFFFFFFFF,
        "record_r31": gpr(regs, 31) & 0xFFFFFFFF,
        "zone": s32(dbg.read(state + 0x20, 4)) if state else None,
        "old_0x640": u32(dbg.read(hud + 0x640, 4)),
        "H0": stage_entry(dbg, 0),
    }


def race_over():
    return "EndRace Results" in drive.tty_text()


def run_hud_writer(out, budget=480.0):
    """`FUN_0008ce10`'s three `stw r7, 0x640(r30)`, live, across a whole race.

    The second boot showed `Hud_UpdateZoneSpeedClass`'s store never runs in
    a Racebox Zone race while the field steps 1 -> 4 regardless. The HUD's
    other ladder walker over the same table is `FUN_0008ce10`; this arms its
    entry for a few consecutive frames, then samples its stores until the
    race ends (an unsteered Zone craft lasts about 69 s of race time), and
    each time a store carries a value different from what the field held,
    waits for the blend's commit right behind it.
    """
    result = {"globals": {}, "entry": [], "stores": [], "commits": []}

    def save():
        (out / "hud-writer.json").write_text(json.dumps(result, indent=1))

    with drive.Session(str(IMAGE), str(out / "logs"), interpreter=True) as session:
        if not boot_into_zone(session, out, result, save):
            return 1
        dbg = Debugger()
        breaker = flare.Breaker(dbg)
        try:
            dbg.pause()
            dbg.drain()
            result["globals"]["at_attach"] = globals_readout(dbg)
            print("globals: %s" % json.dumps(result["globals"]["at_attach"]), flush=True)
            save()
            print("== entry at %#x" % HUD_LADDER_ENTRY, flush=True)
            collect(breaker, HUD_LADDER_ENTRY, hud_entry_readout, 8, 60.0, "entry",
                    result["entry"], save)
            last = None
            started = time.time()
            misses = 0
            while time.time() - started < budget and not race_over():
                hit = None
                for address in HUD_LADDER_STORES[:2]:
                    waited, tid, regs = wait_for_hit(breaker, address, budget=6.0)
                    if regs is not None:
                        hit = hud_store_readout(dbg, regs, address)
                        hit["waited_s"] = round(waited, 2)
                        hit["tid"] = tid
                        hit["t"] = round(time.time() - started, 1)
                        break
                if hit is None:
                    misses += 1
                    print("  no store hit (%d)" % misses, flush=True)
                    if misses >= 4:
                        break
                    continue
                breaker.step_off(int(hit["at"], 16))
                hud = int(hit["hud_r30"], 16)
                hit["new_0x640"] = u32(dbg.read(hud + 0x640, 4))
                result["stores"].append(hit)
                changed = hit["value_r7"] != hit["old_0x640"]
                print("  store %s%s" % (json.dumps(hit, sort_keys=True),
                                        "   <- CHANGE" if changed else ""), flush=True)
                save()
                if changed or (last is not None and last != hit["new_0x640"]):
                    print("== waiting on the commit at %#x" % COMMIT_STORE, flush=True)
                    waited, tid, regs = wait_for_commit(breaker, budget=30.0)
                    row = {"waited_s": round(waited, 1), "tid": tid,
                           "after_store": hit["t"]}
                    if regs is not None:
                        row.update(commit_readout(dbg, regs))
                        breaker.step_off(COMMIT_STORE)
                        n = row["n"]
                        row["H_after"] = stage_entry(dbg, n) if n < 2 else None
                        drive.screenshot(out / ("commit-%d.png" % len(result["commits"])),
                                         trim=True)
                    result["commits"].append(row)
                    print("  commit %s" % json.dumps(row), flush=True)
                    save()
                last = hit["new_0x640"]
            result["race_over"] = race_over()
            result["globals"]["at_end"] = globals_readout(dbg)
            result["step_off"] = {"stepped": breaker.stepped, "ran_off": breaker.ran_off}
            save()
            dbg.resume()
        except (TimeoutError, OSError) as exc:
            print("stub desync: %r" % exc, file=sys.stderr, flush=True)
            result["desync"] = repr(exc)
        finally:
            try:
                dbg.resume()
            except Exception:
                pass
            dbg.close()
    save()
    return 0


def run(out):
    result = {"globals": {}, "dispatch": [], "targets": {}, "source": [],
              "commits": [], "writer": [], "flare_gate": []}

    def save():
        (out / "zone-source.json").write_text(json.dumps(result, indent=1))

    with drive.Session(str(IMAGE), str(out / "logs"), interpreter=True) as session:
        if not boot_into_zone(session, out, result, save):
            return 1
        dbg = Debugger()
        breaker = flare.Breaker(dbg)
        try:
            dbg.pause()
            dbg.drain()
            result["globals"]["at_attach"] = globals_readout(dbg)
            print("globals: %s" % json.dumps(result["globals"]["at_attach"]), flush=True)
            save()

            print("== flare gate, for the player's craft pointer", flush=True)
            for _ in range(4):
                tid, regs = breaker.stop_at(flare.FLARE_GATE, tries=4)
                if tid is None:
                    break
                entry = flare.flare_readout(dbg, regs, flare.FLARE_GATE)
                result["flare_gate"].append(entry)
                print("  %s" % json.dumps(entry, sort_keys=True), flush=True)
            save()

            print("== 1. dispatch at %#x" % DISPATCH, flush=True)
            collect(breaker, DISPATCH, dispatch_readout, 24, 240.0, "dispatch",
                    result["dispatch"], save)

            print("== 2. the four targets, each armed alone", flush=True)
            for name, address in TARGETS:
                slices, tid, regs = probe_target(breaker, address)
                row = {"address": "%#x" % address, "hit_on_slice": slices, "tid": tid}
                if regs is not None:
                    row["mode_mem"] = u32(dbg.read(MODE_ADDRESS, 4))
                    row["skip_byte"] = dbg.read(SKIP_BYTE, 1)[0]
                    row["r0"] = gpr(regs, 0) & 0xFFFFFFFF
                    row["n_r31"] = gpr(regs, 31) & 0xFFFFFFFF
                result["targets"][name] = row
                print("  %s: %s" % (name, json.dumps(row)), flush=True)
                save()
            # A second sweep, so a "never" is two arms and sixteen slices.
            for name, address in TARGETS:
                slices, tid, regs = probe_target(breaker, address)
                result["targets"][name]["second_sweep_hit_on_slice"] = slices
                print("  %s again: slice %s" % (name, slices), flush=True)
            save()

            print("== 3. source at %#x, first burst" % SOURCE_STORE, flush=True)
            collect(breaker, SOURCE_STORE, source_readout, 60, 300.0, "source",
                    result["source"], save)
            drive.screenshot(out / "source-burst-0.png", trim=True)

            print("== 5. writer at %#x, first burst" % WRITER_STORE, flush=True)
            collect(breaker, WRITER_STORE, writer_readout, 12, 120.0, "writer",
                    result["writer"], save)

            for step in range(3):
                print("== 4. waiting on the commit at %#x (step %d)" % (COMMIT_STORE, step),
                      flush=True)
                waited, tid, regs = wait_for_commit(breaker, budget=240.0)
                row = {"waited_s": round(waited, 1), "tid": tid}
                if regs is not None:
                    row.update(commit_readout(dbg, regs))
                    row["threads"] = thread_dump(dbg)
                    # The committed value, read back after the store lands.
                    breaker.step_off(COMMIT_STORE)
                    row["H_after"] = stage_entry(dbg, row["n"]) if row["n"] < 2 else None
                result["commits"].append(row)
                print("  commit %s" % json.dumps({k: v for k, v in row.items() if k != "threads"}),
                      flush=True)
                save()
                if regs is None:
                    print("  no commit in %.0f s" % waited, flush=True)
                    continue
                drive.screenshot(out / ("commit-%d.png" % step), trim=True)
                collect(breaker, SOURCE_STORE, source_readout, len(result["source"]) + 40,
                        240.0, "source", result["source"], save)
                collect(breaker, WRITER_STORE, writer_readout, len(result["writer"]) + 8,
                        120.0, "writer", result["writer"], save)
            result["globals"]["at_end"] = globals_readout(dbg)
            result["step_off"] = {"stepped": breaker.stepped, "ran_off": breaker.ran_off}
            save()
            dbg.resume()
        except (TimeoutError, OSError) as exc:
            print("stub desync: %r" % exc, file=sys.stderr, flush=True)
            result["desync"] = repr(exc)
        finally:
            try:
                dbg.resume()
            except Exception:
                pass
            dbg.close()
    result["summary"] = summarise(result)
    save()
    print(json.dumps(result["summary"], indent=1))
    return 0


def summarise(result):
    modes = {}
    for hit in result["dispatch"]:
        modes[hit["mode_r0"]] = modes.get(hit["mode_r0"], 0) + 1
    fired = {name: row.get("hit_on_slice") for name, row in result["targets"].items()}
    values = []
    for hit in result["source"]:
        if not values or values[-1][1] != hit["value_r0"]:
            values.append((hit["t"], hit["value_r0"], hit["T"]))
    writer_objects = sorted({hit["object_r29"] for hit in result["writer"]})
    source_objects = sorted({hit["craft_r11"] for hit in result["source"]})
    return {
        "dispatch_modes": modes,
        "targets_hit_on_slice": fired,
        "source_hits": len(result["source"]),
        "source_value_runs": values,
        "source_objects": source_objects,
        "writer_objects": writer_objects,
        "writer_matches_source": writer_objects == source_objects,
        "commits": [{k: v for k, v in c.items() if k != "threads"} for c in result["commits"]],
    }


def main():
    modes = ("zone-source", "writer-chain", "hud-writer")
    if len(sys.argv) < 2 or sys.argv[1] not in modes:
        print(__doc__, file=sys.stderr)
        return 2
    mode = sys.argv[1]
    out = Path(sys.argv[2] if len(sys.argv) > 2
               else ROOT / "data/reference/hd-capture/zone-stage-source" / mode)
    out.mkdir(parents=True, exist_ok=True)
    if mode == "writer-chain":
        return run_writer_chain(out)
    if mode == "hud-writer":
        return run_hud_writer(out)
    return run(out)


if __name__ == "__main__":
    sys.exit(main())
