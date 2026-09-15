#!/usr/bin/env python3
"""Two live measurements on HD/Fury under RPCS3's GDB stub, one boot each.

`flare` - **does `EngineFlare_RenderTick` skip the viewing player's own craft?**
`0x002a0bb4` (`beq cr7 -> epilogue`) is the last gate before the sprite's
fade math and quad (engine-trail.md, "Ninth session"), and `r10` at
`0x002a0bac` is the value it tests: `1` for a craft whose owner index
(`craft+0x7a60`) is `-1`, else `owner != view` (`*0x008c1430`), overridden
by a camera object (`*0x009878c0`) targeting the craft. The static reading
is "the sprite is not drawn for the craft the current view belongs to",
held at 75 because neither input had been seen live. So: one breakpoint at
`0x002a0bac`, and on each hit `r31` (the flare), `r10` (the gate), the
craft pointer `*(r31+0x134)`, its owner `*(craft+0x7a60)`, the view index
and the camera object's target/mode. Then, as the positive control, the
same per-craft readout at `0x002a1104` - the `bl 0x002c4ad0` that submits
the four vertices, reached only through the fade path - and at
`0x002a1428`, where `r28` is the occlusion query's answer.

`zone` - **does `zoneOrigin` move every frame, and with what?**
`Scene_PrepareFrame` stores it at `0x003ad8dc` (`stvx v0,0,r9`) from
`lvx v0,r3,r0` with `r0 = 0x30`, i.e. the float4 at `+0xb0` of the object
`r3 - 0x80` (zone-effectsettings-loader.md, thirtieth pass). The `g` dump
carries no VMX registers, so the value about to be stored is read as the
16 bytes at `r3 + 0x30` instead, alongside the 16 bytes still sitting at
`0x00c81550` (last frame's), the whole `+0x80..+0xc0` block, the entity
`array[r27]` the object hangs off, the stage table's radius and weight
(`0x008c2cb8+0x08`/`+0x18`) and the byte at `0x009384dd` that gates the
radius advance. Consecutive hits are consecutive frames. A short pass on
the flare gate first collects the player's craft pointer (owner `0`) so the
entity can be compared against it, and at the end `start` is pressed with
the target running and `0x009384dd` re-read, to see whether it is the pause
flag.

Both boot with `Session(interpreter=True)`: the muted `--config` copy with
`PPU Decoder: Interpreter (static)`, the only decoder `Z0` fires under.
One breakpoint armed at a time, `hd-fury-backdrop-break.py`'s `stop_at`
shape; between two hits on the *same* address the thread is stepped off it
with the breakpoint removed, since a thread resumed onto an armed address
does not step over it.

    uv run --with evdev python3 scripts/hd-flare-owner-break.py flare [out_dir]
    uv run --with evdev python3 scripts/hd-flare-owner-break.py flare-gate [out_dir]   # the gate alone, 240 hits
    uv run --with evdev python3 scripts/hd-flare-owner-break.py zone [out_dir]
"""

import importlib.util
import json
import struct
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
from rpcs3_debugger import Debugger, REG_PC  # noqa: E402

IMAGE = ROOT / "data/images/hdfury-ps3-eu-dec.iso"

# EngineFlare_RenderTick (0x002a08a8)
FLARE_GATE = 0x002A0BAC      # rlwinm r0,r10,..; beq at +8 skips fade and quad
FLARE_SUBMIT = 0x002A1104    # bl 0x002c4ad0 - the four-vertex submit
FLARE_QUERY = 0x002A1428     # rlwinm r0,r28 - the occlusion query's answer
VIEW_INDEX = 0x008C1430      # *(r25) in RenderTick, r2+0x5a8c -> 0x008b2f64
CAMERA_OBJECT_PTR = 0x009878C0  # r2+0x59dc -> 0x008b2eb4 -> this; *this is the object

# Scene_PrepareFrame (0x003aa888)
ZONE_STORE = 0x003AD8DC      # stvx v0,0,r9 : zoneOrigin = *(r3 + 0x30)
ZONE_ORIGIN = 0x00C81550
ENTITY_ARRAY = 0x0098D7C0    # r2-0x617c; array[r27] -> +0x6adc -> the object
STAGE_TABLE = 0x008C2CB8     # H, 0x38-byte entries; +0x08 radius, +0x18 weight
GATE_BYTE = 0x009384DD


def gpr(regs, index):
    return int.from_bytes(regs[index * 8:index * 8 + 8], "big")


def u32(blob, at=0):
    return int.from_bytes(blob[at:at + 4], "big")


def s32(blob, at=0):
    return int.from_bytes(blob[at:at + 4], "big", signed=True)


def floats(blob):
    return list(struct.unpack(">%df" % (len(blob) // 4), blob))


def find_at(dbg, address, prefer=None):
    """`(tid, regs)` of a thread parked at `address`, checking `prefer` first."""
    if prefer is not None and dbg.select(prefer):
        regs = dbg.registers()
        if regs is not None and int.from_bytes(regs[REG_PC:REG_PC + 8], "big") == address:
            return prefer, regs
    for tid in dbg.threads():
        if tid == prefer:
            continue
        regs = dbg.registers(tid)
        if regs is None:
            continue
        if int.from_bytes(regs[REG_PC:REG_PC + 8], "big") == address:
            return tid, regs
    return None, None


class Breaker:
    """One armed address at a time, with the step-off between repeat hits."""

    def __init__(self, dbg):
        self.dbg = dbg
        self.prefer = None
        self.parked = None  # address a thread was last left parked on
        self.stepped = 0
        self.ran_off = 0

    def step_off(self, address):
        """Move the parked thread past `address` with nothing armed there.

        Done by *hopping*: arm `address + 4`, resume, and the parked thread
        executes exactly one instruction before it parks again - the shape
        `hd-fury-backdrop-break.py` alternates its two addresses in, so a
        thread is never resumed onto an armed address. With that, the very
        next call of the same function - the next flare in the same frame -
        is what the re-armed breakpoint catches. `vCont;s` was tried first
        and never moved the thread (0 of 213 on 2026-09-15), and a 30 ms free
        run instead let the whole frame go by and sampled one craft 172 times
        out of 214; the free run stays as the fallback.
        """
        dbg = self.dbg
        hop = address + 4
        if self.prefer is not None:
            dbg.add_breakpoint(hop)
            try:
                dbg.resume()
                if dbg.wait_for_stop(timeout=1.0) is None:
                    dbg.pause()
                dbg.drain()
                tid, _ = find_at(dbg, hop, self.prefer)
            finally:
                dbg.remove_breakpoint(hop)
            if tid is not None:
                self.parked = hop
                self.stepped += 1
                return True
        for _ in range(6):
            dbg.run_for(0.03)
            dbg.drain()
            tid, _ = find_at(dbg, address, self.prefer)
            if tid is None:
                self.parked = None
                self.ran_off += 1
                return True
        return False

    def stop_at(self, address, tries=20, slice_seconds=0.3):
        dbg = self.dbg
        if self.parked == address:
            self.step_off(address)
        dbg.add_breakpoint(address)
        try:
            for _ in range(tries):
                dbg.resume()
                if dbg.wait_for_stop(timeout=slice_seconds) is None:
                    dbg.pause()
                dbg.drain()
                tid, regs = find_at(dbg, address, self.prefer)
                if tid is not None:
                    self.prefer = tid
                    self.parked = address
                    return tid, regs
        finally:
            dbg.remove_breakpoint(address)
        return None, None


def flare_readout(dbg, regs, where):
    """Everything the gate at 0x002a0bb4 looks at, for one call."""
    flare = gpr(regs, 31)
    craft = u32(dbg.read(flare + 0x134, 4))
    entry = {
        "at": "%#x" % where,
        "flare": "%#x" % flare,
        "craft": "%#x" % craft,
        "r10": gpr(regs, 10) & 0xFF,
        "r28": gpr(regs, 28) & 0xFFFFFFFF,
        "r29": gpr(regs, 29) & 0xFF,
    }
    if craft:
        entry["owner"] = s32(dbg.read(craft + 0x7A60, 4))
        entry["count_field"] = u32(dbg.read(craft + 0x5FA4, 4))
    entry["view"] = s32(dbg.read(VIEW_INDEX, 4))
    # Frame stamps: zoneOrigin is rewritten once per frame by
    # Scene_PrepareFrame, and the flare's own occlusion ring index (+0x270)
    # and alpha-noise countdown (+0x190) advance once per call that reaches
    # them - so two hits on the same flare can be told apart as "same frame,
    # called again" versus "next frame".
    entry["frame_stamp"] = dbg.read(ZONE_ORIGIN, 16).hex()
    ring = dbg.read(flare + 0x18C, 4)
    entry["fade_alpha"] = floats(ring)[0]
    entry["noise_countdown"] = u32(dbg.read(flare + 0x190, 4))
    entry["ring_index"] = u32(dbg.read(flare + 0x270, 4))
    camera = u32(dbg.read(CAMERA_OBJECT_PTR, 4))
    entry["camera"] = "%#x" % camera
    if camera:
        head = dbg.read(camera + 0x34, 0x10)
        entry["camera_flags"] = "%#x" % u32(head, 0)
        entry["camera_mode"] = u32(head, 0xC)
        entry["camera_target"] = "%#x" % u32(dbg.read(camera + 0x1EC, 4))
        entry["camera_targets_craft"] = entry["camera_target"] == entry["craft"]
    return entry


def summarise_flare(hits):
    table = {}
    for h in hits:
        key = (h["craft"], h.get("owner"))
        row = table.setdefault(key, Counter())
        row["hits"] += 1
        row["r10=%d" % h["r10"]] += 1
        row["view=%d" % h["view"]] += 1
        if h.get("camera_targets_craft"):
            row["camera_targets"] += 1
    return {"%s owner=%s" % k: dict(v) for k, v in sorted(table.items())}


def boot_to_main_menu(session):
    print("rpcs3 pid %d" % session.proc.pid, flush=True)
    if not session.wait_for_screen_pressing("Main Menu", 420.0):
        print("never reached the Main Menu (last screen: %s)"
              % drive.current_screen(), file=sys.stderr)
        return False
    print("Main Menu; config: %s" % drive.config_report(), flush=True)
    time.sleep(12.0)
    return True


def wait_for_load(session, out, seconds=600.0):
    """Sit through the track load and say what TTY.log called the race.

    Keyed on `Loading Screen Finished` rather than a fixed pause: under the
    interpreter on a loaded machine the load ran past 90 s on 2026-09-15,
    and attaching the debugger mid-load pauses the load itself.
    """
    # The front end prints `Loading Screen Finished` on its own boot too, so
    # the marker is a *new* one after `InGame`, plus the track line.
    before = drive.tty_text().count("Loading Screen Finished")
    deadline = time.time() + seconds
    while time.time() < deadline:
        text = drive.tty_text()
        if (text.count("Loading Screen Finished") > before
                and "Loading track model" in text):
            break
        time.sleep(2.0)
    else:
        raise SystemExit("the track never finished loading in %g s" % seconds)
    text = drive.tty_text()
    race_type = [l for l in text.splitlines() if "RACE TYPE" in l]
    print("track: %s; %s" % (drive.track_name(), race_type[-1:] or "no RACE TYPE line"),
          flush=True)
    time.sleep(10.0)
    drive.screenshot(out / "race-loaded.png", trim=True)
    return race_type[-1] if race_type else None


def run_flare(out, gate_only=False):
    result = {"gate": [], "submit": [], "query": []}
    with drive.Session(str(IMAGE), str(out / "logs"), interpreter=True) as session:
        if not boot_to_main_menu(session):
            return 1
        if session.walk_to_race() not in drive.RACE_ARRIVED:
            print("did not reach a race (%s)" % drive.current_screen(), file=sys.stderr)
            return 1
        result["race_type"] = wait_for_load(session, out)
        # Thrust so the field spreads and the camera moves; the pad state
        # persists while the target is stopped.
        session.pad.set("cross", True)
        time.sleep(6.0)
        dbg = Debugger()
        breaker = Breaker(dbg)
        try:
            dbg.pause()
            dbg.drain()
            phases = (("gate", FLARE_GATE, 160, 600.0),
                      ("submit", FLARE_SUBMIT, 40, 240.0),
                      ("query", FLARE_QUERY, 40, 240.0))
            if gate_only:
                phases = (("gate", FLARE_GATE, 240, 900.0),)
            for name, address, want, budget in phases:
                print("== %s at %#x" % (name, address), flush=True)
                deadline = time.time() + budget
                misses = 0
                while len(result[name]) < want and time.time() < deadline:
                    tid, regs = breaker.stop_at(address, tries=4)
                    if tid is None:
                        misses += 1
                        print("  no thread reached %#x (%d)" % (address, misses), flush=True)
                        if misses >= 3:
                            break
                        continue
                    entry = flare_readout(dbg, regs, address)
                    entry["tid"] = tid
                    result[name].append(entry)
                    print("  %s" % json.dumps(entry, sort_keys=True), flush=True)
                    (out / "flare.json").write_text(json.dumps(result, indent=1))
                if name == "gate":
                    drive.screenshot(out / "race-gate.png", trim=True)
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
        session.pad.set("cross", False)
    result["summary"] = {name: summarise_flare(result[name]) for name in ("gate", "submit", "query")}
    result["step_off"] = {"stepped": breaker.stepped, "ran_off": breaker.ran_off}
    (out / "flare.json").write_text(json.dumps(result, indent=1))
    print(json.dumps(result["summary"], indent=1))
    return 0


def walk_to_zone(session, out):
    """Main Menu -> RACEBOX tab -> Single Player -> Mode: Zone -> a race.

    The recipe rpcs3-debugger.md records as working: the Mode carousel's
    first tap after arrival is eaten while the entrance animation runs, so
    sleep 3 s first; four `right` taps at 1.5 s land on `Zone`; screenshot
    before confirming, since a dropped tap leaves no trace in TTY.log.
    """
    session.tap("right", settle=2.0)
    drive.screenshot(out / "menu-racebox-tab.png", trim=True)
    was, now = session.press_once("cross")
    print("  press cross  %-22s -> %s" % (was, now), flush=True)
    if now != "Single Player":
        print("expected Single Player, got %r" % now, file=sys.stderr)
        return False
    time.sleep(3.0)
    for n in range(4):
        session.tap("right", settle=1.5)
        drive.screenshot(out / ("mode-%d.png" % (n + 1)), trim=True)
    print("  four rights sent; confirm mode-4.png reads ZONE", flush=True)
    return session.walk_to_race() in drive.RACE_ARRIVED


def zone_readout(dbg, regs, prev):
    r3 = gpr(regs, 3) & 0xFFFFFFFF
    r27 = gpr(regs, 27) & 0xFFFFFFFF
    entity = u32(dbg.read(ENTITY_ARRAY + r27 * 4, 4))
    block = dbg.read(r3, 0x40)
    entry = {
        "r3": "%#x" % r3,
        "object": "%#x" % (r3 - 0x80),
        "r27": r27,
        "entity": "%#x" % entity,
        "r9": "%#x" % (gpr(regs, 9) & 0xFFFFFFFF),
        "rows": [floats(block[i:i + 16]) for i in range(0, 0x40, 16)],
        "new_origin": floats(block[0x30:0x40]),
        "zone_origin_before": floats(dbg.read(ZONE_ORIGIN, 16)),
        "gate_byte": dbg.read(GATE_BYTE, 1)[0],
    }
    if entity:
        entry["entity_object"] = "%#x" % u32(dbg.read(entity + 0x6ADC, 4))
    stage = dbg.read(STAGE_TABLE, 0x38)
    entry["stage"] = {
        "current": u32(stage, 0), "requested": u32(stage, 4),
        "radius": floats(stage[8:12])[0], "speed": floats(stage[0x10:0x14])[0],
        "accel": floats(stage[0x14:0x18])[0], "weight": floats(stage[0x18:0x1C])[0],
    }
    if prev is not None:
        a, b = prev["new_origin"], entry["new_origin"]
        entry["delta"] = [round(y - x, 4) for x, y in zip(a, b)]
    return entry


def run_zone(out):
    result = {"flare_gate": [], "bursts": [], "pause": {}}
    with drive.Session(str(IMAGE), str(out / "logs"), interpreter=True) as session:
        if not boot_to_main_menu(session):
            return 1
        if not walk_to_zone(session, out):
            print("did not reach a race (%s)" % drive.current_screen(), file=sys.stderr)
            return 1
        result["race_type"] = wait_for_load(session, out)
        if not result["race_type"] or "ZONE" not in result["race_type"]:
            print("not a Zone race - stopping here", file=sys.stderr)
            (out / "zone.json").write_text(json.dumps(result, indent=1))
            return 1
        time.sleep(15.0)
        drive.screenshot(out / "race-running.png", trim=True)
        dbg = Debugger()
        breaker = Breaker(dbg)
        try:
            dbg.pause()
            dbg.drain()
            print("== flare gate, for the player's craft pointer", flush=True)
            for _ in range(8):
                tid, regs = breaker.stop_at(FLARE_GATE, tries=4)
                if tid is None:
                    break
                entry = flare_readout(dbg, regs, FLARE_GATE)
                result["flare_gate"].append(entry)
                print("  %s" % json.dumps(entry, sort_keys=True), flush=True)
            for burst in range(3):
                if burst:
                    dbg.run_for(5.0)
                print("== zoneOrigin burst %d" % burst, flush=True)
                hits = []
                prev = None
                misses = 0
                while len(hits) < 70:
                    tid, regs = breaker.stop_at(ZONE_STORE, tries=4)
                    if tid is None:
                        misses += 1
                        print("  no thread reached the store (%d)" % misses, flush=True)
                        if misses >= 3:
                            break
                        continue
                    entry = zone_readout(dbg, regs, prev)
                    entry["tid"] = tid
                    hits.append(entry)
                    prev = entry
                    print("  %s new=%s before=%s d=%s r=%.2f w=%.2f gate=%d ent=%s"
                          % (entry["object"], ["%.2f" % v for v in entry["new_origin"]],
                             ["%.2f" % v for v in entry["zone_origin_before"]],
                             entry.get("delta"), entry["stage"]["radius"],
                             entry["stage"]["weight"], entry["gate_byte"],
                             entry["entity"]), flush=True)
                result["bursts"].append(hits)
                (out / "zone.json").write_text(json.dumps(result, indent=1))
                drive.screenshot(out / ("burst-%d.png" % burst), trim=True)
            # The pause test: press start with the target running, stop, read.
            print("== pause test", flush=True)
            result["pause"]["before"] = dbg.read(GATE_BYTE, 1)[0]
            dbg.resume()
            time.sleep(1.0)
            session.pad.press("start", 0.2)
            time.sleep(3.0)
            dbg.pause()
            dbg.drain()
            result["pause"]["after_start"] = dbg.read(GATE_BYTE, 1)[0]
            drive.screenshot(out / "paused.png", trim=True)
            dbg.resume()
            time.sleep(1.0)
            session.pad.press("start", 0.2)
            time.sleep(3.0)
            dbg.pause()
            dbg.drain()
            result["pause"]["after_second_start"] = dbg.read(GATE_BYTE, 1)[0]
            drive.screenshot(out / "unpaused.png", trim=True)
            print("  %s" % result["pause"], flush=True)
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
    (out / "zone.json").write_text(json.dumps(result, indent=1))
    return 0


def main():
    if len(sys.argv) < 2 or sys.argv[1] not in ("flare", "flare-gate", "zone"):
        print(__doc__, file=sys.stderr)
        return 2
    mode = sys.argv[1]
    out = Path(sys.argv[2] if len(sys.argv) > 2
               else ROOT / "data/reference/hd-capture/flare-owner" / mode)
    out.mkdir(parents=True, exist_ok=True)
    if mode == "zone":
        return run_zone(out)
    return run_flare(out, gate_only=(mode == "flare-gate"))


if __name__ == "__main__":
    sys.exit(main())
