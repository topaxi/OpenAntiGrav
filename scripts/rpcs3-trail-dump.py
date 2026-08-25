#!/usr/bin/env python3
"""Dump Wipeout HD's live engine-trail state out of a running race.

The evidence reproducer for
`docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`: boots the HD disc in
RPCS3 on the virtual display, walks the front end into the first Fury race,
drives, then attaches the GDB stub and dumps - for all eight craft - the
`TrailEffectManager` per-trail block (the 54-sample history ring and its
per-frame context), both double-buffered SPU-output vertex buffers, and the
`EngineFlare` timer fields. Two snapshots, so accumulators and the double
buffer show their movement.

    uv run --with evdev python3 scripts/rpcs3-trail-dump.py [out_dir]
    uv run --with evdev python3 scripts/rpcs3-trail-dump.py [out_dir] --early
    uv run --with evdev python3 scripts/rpcs3-trail-dump.py [out_dir] --params

Needs the decrypted image (`data/images/hdfury-ps3-eu-dec.iso`), the `oag`
input profile and Xvfb :77 - `scripts/rpcs3-drive.py preflight` checks all
three. One debugger session per emulator launch (rpcs3_debugger.py, trap 2),
which is why this is one script rather than an interactive session.

`--params` is the scroll-phase mode. `Trail_InitManagerBuffers`
(`0x006b6870`) binds the material parameter `TrailSpeed` - `~crc32` hash
`0x07431a35` - to the **address** of each trail's phase accumulator, so the
question "what is `TrailSpeed` at draw time" is a pointer-identity check, not
a float hunt: for trails 0..3 it follows `+0x1208`'s instance array, finds
that hash in each instance's parameter table, and reports the value pointer
the binder wrote. A hit is four pointers exactly `0x1230` apart, each equal to
its own trail's `block + 0x1210`, unchanged across two pauses while the float
they point at moves. It also reads the head vertices, which is what says the
SPU's `u` carries no phase of its own.

`--early` is the race-start mode (the 2026-08-24 e0..e4/r0/r1 evidence on
engine-trail.md): it attaches while the race is still loading, polls for the
manager, and snapshots the player's trail the moment it exists and again at
1/2/4/8 s, then twice more once thrust is held - which is how "the ring is
already full, bunched at the grid slot, every vertex alpha 0" was read. The
default mode is the original at-speed dump of all eight craft.

Both modes also read the frame singleton `**0x00936FD4` (the object
`FUN_00018848`'s present loop hands to the render calls): its `+0xc0`/`+0xc4`
carry a monotonic clock in seconds, which is what refuted the "the splatted
draw-state constant is the TrailSpeed phase" hypothesis - one global value
cannot equal eight diverging per-trail accumulators.

Addresses, all from the Ghidra corpus (EBOOT.elf, BCES-00664):

- `0x00AED460 + 0x6c`: the manager global - `Trail_ConstructManager` stores
  `this` there; `this + 0x40` is the 0x11b00 allocation.
- alloc `+ 0x84a0 + slot * 0x1230`: one craft's trail block. Bytes 0..0x10e0
  are the ring (54 x 0x50: three basis rows, position, colour); the tail is
  the per-frame context and header (craft pointer at +0x1204, RSX vertex
  buffers at +0x1214/+0x1218, brightness at +0x11d8, u-head at +0x11ec,
  scroll phase at +0x1210).
- The RSX buffers read back through the stub at their `0xC...` addresses:
  0x2d90 bytes = 324 vertices of 36 (position f32x3, normal f32x3, Uv1
  f32x2, VertexColour1 u8x4 - the manager's own attribute table at
  alloc+0x11a00).
- `craft + 0x5f70`: the `EngineFlare`; `+0x12c` its boost timer, `+0x144`
  and `+0x150` its two boost blends. `craft + 0x7d2c`: the Fury-skin flag.
"""

import importlib.util
import json
import struct
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
spec = importlib.util.spec_from_file_location(
    "rpcs3_drive", ROOT / "scripts" / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)
from rpcs3_debugger import Debugger  # noqa: E402

IMAGE = ROOT / "data/images/hdfury-ps3-eu-dec.iso"
MANAGER_SLOT = 0x00AED460 + 0x6C
FLARE_VTABLE = 0x00869D30
FLARE_OFFSET = 0x5F70
# The pointer-to-pointer the present loop's TOC slot 0x008b44a0 resolves to;
# Trail_BuildDrawState splats *(*(here) + 0xc4) into a draw-state constant.
SINGLETON_PP = 0x00936FD4
# `~crc32("TrailSpeed")`, the form the material tables key parameters by -
# `ps3-sho.py hash TrailSpeed` prints it and `Trail_InitManagerBuffers`
# computes it with `Crc32_HashString` on the string at 0x007a2da8.
TRAIL_SPEED_HASH = 0x07431A35
# A trail's material-instance array (`+0x1208`) and its length (`+0x120c`),
# and inside one instance the parameter table `Material_SetInstanceParamPointer`
# walks: count at `+0x30`, entries at `+0x34`, 0x20 bytes each, `+0x00` the
# name hash, `+0x04` the kind (bit 15 marks a sampler), `+0x18` the value
# pointer the binder overwrites, `+0x1c` the vec4 count.
PARAM_ENTRY = 0x20
# Main memory, wide enough for both allocators involved: the instance *array*
# lands around `0x3058....` and the instances it points at around `0x40cb....`.
# A first run of this stopped at 0x40000000 and rejected every instance as "not
# a heap pointer" - the same shape of miss as reading a pointer for a float,
# and the reason `0x40...` is called out as stub-readable in the first place.
HEAP_LO, HEAP_HI = 0x10000000, 0x50000000
# A guard, not a measurement: an instance array longer than this is a sign the
# block was misread, and scanning it would burn the pause.
MAX_INSTANCES = 64


def u32(b, off=0):
    return struct.unpack_from(">I", b, off)[0]


def f32(b, off=0):
    return struct.unpack_from(">f", b, off)[0]


def read_singleton(dbg):
    """The frame singleton's +0xc0..0xcc, a monotonic seconds clock."""
    try:
        ptr = u32(dbg.read(SINGLETON_PP, 4))
        if not ptr:
            return {"ptr": 0}
        blk = dbg.read(ptr + 0xC0, 0x10)
        return {"ptr": "%08x" % ptr, "c0": f32(blk, 0), "c4": f32(blk, 4)}
    except Exception as e:  # a mid-load read can race object construction
        return {"error": str(e)[:80]}


def dump(dbg, out, addr, size, path=None):
    data = b""
    while len(data) < size:
        n = min(0x200, size - len(data))
        data += dbg.read(addr + len(data), n)
    if path:
        (out / path).write_bytes(data)
    return data


def early_snapshot(dbg, out, alloc, tag):
    """The player's trail block and both vertex buffers, nothing else -
    small enough (~2 s of stub reads) to catch a loading race's state."""
    base = alloc + 0x84A0
    blk = dump(dbg, out, base, 0x1230, "%s_trail0.bin" % tag)
    for name, off in (("A", 0x1214), ("B", 0x1218)):
        addr = u32(blk, off)
        if 0xC0000000 <= addr < 0xD0000000:
            dump(dbg, out, addr, 0x2D90, "%s_vtx0%s.bin" % (tag, name))
    return {
        "wall": time.time(),
        "singleton": read_singleton(dbg),
        "phase_1210": f32(blk, 0x1210),
        "bright_11d8": f32(blk, 0x11D8),
        "p_11e4": "%08x" % u32(blk, 0x11E4),
    }


def run_early(session, out):
    print("in race; attaching during the load", flush=True)
    time.sleep(18.0)
    dbg = Debugger()
    metas = {}
    try:
        alloc = 0
        deadline = time.time() + 90.0
        while time.time() < deadline:
            dbg.pause()
            try:
                manager = u32(dbg.read(MANAGER_SLOT, 4))
                alloc = u32(dbg.read(manager + 0x40, 4)) if manager else 0
            except Exception:
                alloc = 0
            if alloc:
                break
            dbg.resume()
            time.sleep(2.0)
        if not alloc:
            print("no manager after 90 s", file=sys.stderr)
            return 1
        print("manager alloc %08x" % alloc, flush=True)
        metas["alloc"] = "%08x" % alloc
        metas["e0"] = early_snapshot(dbg, out, alloc, "e0")
        dbg.resume()
        for tag, wait in (("e1", 1.0), ("e2", 2.0), ("e3", 4.0), ("e4", 8.0)):
            time.sleep(wait)
            dbg.pause()
            metas[tag] = early_snapshot(dbg, out, alloc, tag)
            dbg.resume()
        session.pad.set("cross", True)
        for tag, wait in (("r0", 3.0), ("r1", 8.0)):
            time.sleep(wait)
            dbg.pause()
            metas[tag] = early_snapshot(dbg, out, alloc, tag)
            dbg.resume()
    finally:
        try:
            dbg.resume()
        except Exception:
            pass
        dbg.close()
    session.pad.set("cross", False)
    (out / "meta.json").write_text(json.dumps(metas, indent=1))
    drive.screenshot(out / "race.png")
    print("done; artefacts in %s" % out, flush=True)
    return 0


def trail_speed_bindings(dbg, out, tag, slot, arr, count, expect):
    """Every `TrailSpeed` parameter entry in one trail's instance array.

    Returns the value pointer the binder wrote into each match, which is the
    whole point: the draw path reads the scroll phase *through* it.

    Everything it looked at is reported, including the misses. A silent
    `continue` past an out-of-range field is how the first run of this came
    back as an empty list that read like "the binding is not there" when it
    only meant "these offsets were guessed"; `raw` is the instance's own bytes
    so a wrong guess is fixable from the artefacts instead of another race.
    """
    report = {"array": "%08x" % arr, "count": count, "instances": []}
    if not arr or not count or count > MAX_INSTANCES:
        report["error"] = "array/count out of range"
        return report
    handles = dbg.read(arr, count * 4)
    for i in range(count):
        inst = u32(handles, i * 4)
        row = {"instance": "%08x" % inst, "hits": []}
        report["instances"].append(row)
        if not HEAP_LO <= inst < HEAP_HI:
            row["skipped"] = "not a heap pointer"
            continue
        raw = dump(dbg, out, inst, 0x200, "%s_inst%d_%d.bin" % (tag, slot, i))
        entries, table = u32(raw, 0x30), u32(raw, 0x34)
        row["count_30"] = entries
        row["table_34"] = "%08x" % table
        # The offsets the binder uses, tried first.
        if table and HEAP_LO <= table < HEAP_HI and 0 < entries <= 64:
            blob = dump(dbg, out, table, entries * PARAM_ENTRY,
                        "%s_params%d_%d.bin" % (tag, slot, i))
            for e in range(entries):
                off = e * PARAM_ENTRY
                row.setdefault("hashes", []).append("%08x" % u32(blob, off))
                if u32(blob, off) != TRAIL_SPEED_HASH:
                    continue
                if u32(blob, off + 4) & 0x8000:  # a sampler; the binder skips
                    continue
                row["hits"].append({
                    "entry": e,
                    "value_ptr": "%08x" % u32(blob, off + 0x18),
                    "vec4s": u32(blob, off + 0x1C),
                    "matches_phase_addr": "%08x" % u32(blob, off + 0x18) == expect,
                })
        else:
            row["skipped"] = "count/table at +0x30/+0x34 out of range"
        # Independent of the offsets above: where the hash and the expected
        # pointer actually sit in the instance's own bytes.
        row["hash_at"] = ["0x%x" % o for o in range(0, 0x200, 4)
                          if u32(raw, o) == TRAIL_SPEED_HASH]
        row["phase_ptr_at"] = ["0x%x" % o for o in range(0, 0x200, 4)
                               if "%08x" % u32(raw, o) == expect]
    return report


def params_snapshot(dbg, out, alloc, tag):
    """Trails 0..3: the block, the `TrailSpeed` bindings, the head vertices.

    One pause, one hop through `+0x1208`. The pointers are what settles where
    the draw-time `TrailSpeed` comes from; the head vertices settle whether
    the SPU already put the phase in `u`.
    """
    rows = []
    for i in range(4):
        base = alloc + 0x84A0 + i * 0x1230
        blk = dump(dbg, out, base, 0x1230, "%s_trail%d.bin" % (tag, i))
        bindings = trail_speed_bindings(
            dbg, out, tag, i, u32(blk, 0x1208), u32(blk, 0x120C),
            "%08x" % (base + 0x1210))
        row = {
            "slot": i,
            "block": "%08x" % base,
            "phase_1210": f32(blk, 0x1210),
            "phase_addr": "%08x" % (base + 0x1210),
            "uhead_11ec": f32(blk, 0x11EC),
            "bright_11d8": f32(blk, 0x11D8),
            "p_1208": "%08x" % u32(blk, 0x1208),
            "n_120c": u32(blk, 0x120C),
            "trail_speed": bindings,
        }
        live = u32(blk, 0x11D4)
        row["live_11d4"] = "%08x" % live
        if 0xC0000000 <= live < 0xD0000000:
            # 0x200 bytes is the first 14 vertices - rings 0..6 of fin 0,
            # which is all the `u` law needs.
            head = dump(dbg, out, live, 0x200, "%s_vtxhead%d.bin" % (tag, i))
            row["vertex_u"] = [f32(head, v * 0x24 + 0x18) for v in range(6)]
        rows.append(row)
    return {"wall": time.time(), "singleton": read_singleton(dbg),
            "alloc": "%08x" % alloc, "trails": rows}


def run_params(session, out):
    print("in race; waiting for the load", flush=True)
    time.sleep(50.0)
    session.pad.set("cross", True)
    time.sleep(16.0)
    dbg = Debugger()
    metas = {}
    try:
        dbg.pause()
        manager = u32(dbg.read(MANAGER_SLOT, 4))
        alloc = u32(dbg.read(manager + 0x40, 4))
        print("manager %08x alloc %08x" % (manager, alloc), flush=True)
        metas["p0"] = params_snapshot(dbg, out, alloc, "p0")
        dbg.resume()
        time.sleep(1.2)
        dbg.pause()
        metas["p1"] = params_snapshot(dbg, out, alloc, "p1")
        dbg.resume()
    finally:
        try:
            dbg.resume()
        except Exception:
            pass
        dbg.close()
    session.pad.set("cross", False)
    (out / "meta.json").write_text(json.dumps(metas, indent=1))
    drive.screenshot(out / "race.png")
    print("done; artefacts in %s" % out, flush=True)
    return 0


def snapshot(dbg, out, alloc, tag, manager=0):
    # `alloc` is recorded because without it a dumped pointer cannot be told
    # apart from any other heap address - which is exactly what made an
    # earlier session's `+0x1208` dumps uninterpretable after the fact.
    meta = {"wall": time.time(), "singleton": read_singleton(dbg),
            "manager": "%08x" % manager, "alloc": "%08x" % alloc,
            "trails": []}
    flares = []
    for i in range(8):
        base = alloc + 0x84A0 + i * 0x1230
        blk = dump(dbg, out, base, 0x1230, "%s_trail%d.bin" % (tag, i))
        craft = u32(blk, 0x1204)
        meta["trails"].append({
            "craft": "%08x" % craft,
            "vA": "%08x" % u32(blk, 0x1214),
            "vB": "%08x" % u32(blk, 0x1218),
            "phase_1210": f32(blk, 0x1210),
            "p_1208": "%08x" % u32(blk, 0x1208),
            "p_11e4": "%08x" % u32(blk, 0x11E4),
        })
        dump(dbg, out, u32(blk, 0x1214), 0x2D90, "%s_vtx%dA.bin" % (tag, i))
        dump(dbg, out, u32(blk, 0x1218), 0x2D90, "%s_vtx%dB.bin" % (tag, i))
        flare = u32(dbg.read(craft + FLARE_OFFSET, 4))
        row = {"flare": "%08x" % flare,
               "fury": dbg.read(craft + 0x7D2C, 1)[0]}
        if flare and u32(dbg.read(flare, 4)) == FLARE_VTABLE:
            fd = dbg.read(flare + 0x120, 0x40)
            row.update({
                "timer_12c": struct.unpack_from(">f", fd, 0x0C)[0],
                "blend_144": struct.unpack_from(">f", fd, 0x24)[0],
                "blend_150": struct.unpack_from(">f", fd, 0x30)[0],
            })
        flares.append(row)
    (out / ("%s_flares.json" % tag)).write_text(json.dumps(flares, indent=1))
    return meta


def main():
    flags = {"--early", "--params"}
    args = [a for a in sys.argv[1:] if a not in flags]
    early = "--early" in sys.argv[1:]
    params = "--params" in sys.argv[1:]
    out = Path(args[0] if args else "/tmp/hd-trail-dump")
    out.mkdir(parents=True, exist_ok=True)
    with drive.Session(str(IMAGE), str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        if not session.wait_for_screen("Main Menu", 180.0):
            print("never reached the Main Menu", file=sys.stderr)
            return 1
        time.sleep(12.0)
        if session.walk_to_race() not in drive.RACE_ARRIVED:
            print("did not reach a race", file=sys.stderr)
            return 1
        if early:
            return run_early(session, out)
        if params:
            return run_params(session, out)
        print("in race; waiting for the load", flush=True)
        time.sleep(50.0)
        session.pad.set("cross", True)
        time.sleep(16.0)

        dbg = Debugger()
        metas = []
        try:
            dbg.pause()
            manager = u32(dbg.read(MANAGER_SLOT, 4))
            alloc = u32(dbg.read(manager + 0x40, 4))
            print("manager %08x alloc %08x" % (manager, alloc), flush=True)
            metas.append(snapshot(dbg, out, alloc, "s0", manager))
            dbg.resume()
            time.sleep(1.2)
            dbg.pause()
            metas.append(snapshot(dbg, out, alloc, "s1", manager))
            dbg.resume()
        finally:
            try:
                dbg.resume()
            except Exception:
                pass
            dbg.close()
        (out / "meta.json").write_text(json.dumps(metas, indent=1))
        session.pad.set("cross", False)
        drive.screenshot(out / "race.png")
        print("done; artefacts in %s" % out, flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
