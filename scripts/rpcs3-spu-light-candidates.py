#!/usr/bin/env python3
"""Reads the `Enable_spu_vertex_light` 128-slot **candidate** list and both
halves of the **compacted visible** double buffer, live, in the same paused
instant - the direct test of `docs/ghidra/functions/ps3-hdfury-eu/
renderer.md`'s 2026-09-20 static conclusion ("Thirteen is exhaustive, and the
numbers rule out every one of them as the source of the captured buffer").

This is a copy of `scripts/rpcs3-spu-light-dump.py`, extended rather than
edited: same boot, same Amphiseum walk, same five-snapshots-two-seconds-apart
shape. What is new is *what* gets read per snapshot and *when* it gets
compared:

**One paused stop, four structures, byte-identical timing.** `+0x2098`
(candidate count) and the up-to-128 candidate records at `+0x20a0` are read
in the same pause as `+0x2080` (current index), `+0x2084[0]`/`+0x2084[1]`
(both slots' own frustum-survivor counts - not just the current index's) and
both double-buffer slots' own up-to-128 records at `+0x80`/`+0x1080`. Nothing
here is stitched together from two different pauses.

**The match test is not "does every visible record match some candidate".**
`SpuLight_CompactVisibleCandidates` runs once per frame from
`Scene_PrepareFrame` and something must reset `+0x2098` every frame (it is a
128-slot cap fed by up to thirteen call sites; left unreset it overflows in
seconds), so a pause landing mid-frame generally sees a *partially refilled*
candidate list sitting alongside the *previous* frame's complete compacted
survivors - a non-match there is expected noise, not evidence against a
shared pipeline. The question that actually discriminates: **does any
visible record match any candidate at all**, at three tiers - exact 32-byte
(all eight fields), `(r, g, b, D)` only (position moves frame to frame,
colour and range do not), or none - plus whether the visible list is an
order-preserving subsequence of the candidate list, the structural signature
`SpuLight_CompactVisibleCandidates`'s own sphere-test-and-copy shape would
produce.

**The single number that actually settles the open question** is the
candidate list's own `(r, g, b, D)` values, independent of any comparison
to the visible buffer: if they read the `1 : 0.25 : 0.1` ratio at magnitudes
`40`-`440` with `D` in `0.6`-`2.0` (the visible buffer's own established
range, `data/traces/hd-spu-light-companion/`), the thirteen catalogued
producers' constants were misread statically (plausible - per renderer.md's
own table, colour goes via `vs34`/`vs35`, not a directly-disassembled
literal, for several sites) and the 2026-09-20 negative result is withdrawn.
If the candidates read `(100, 50, 50)` / `(7, 5, 1)` / `(500, 200, 50)` at
`D` in the `10`-`100` catalogued there, the static reading holds and the
buffer's real source is still unfound.

Records past the buffer's own count are still saved and decoded, but
labelled `stale` rather than silently mixed in - the mistake
`docs/ghidra/functions/ps3-hdfury-eu/renderer.md`'s "record 7's 'stationary'
position ... was a stale record past the count" cost a session to retract.

    uv run --with evdev python3 scripts/rpcs3-spu-light-candidates.py [out_dir]
    uv run --with evdev python3 scripts/rpcs3-spu-light-candidates.py analyze <out_dir>
    uv run --with evdev python3 scripts/rpcs3-spu-light-candidates.py attribute [out_dir] [max_hits]

The second form re-runs the comparison against already-saved raw `.bin`
files with no emulator involved - re-deriving the analysis is free, a boot
costs about 40s plus an 80s drive to Amphiseum, so any re-cut of the numbers
below should use this rather than re-capturing.

**`attribute` is the caller-identification pass, part 2 of this script's own
brief.** It boots with `PPU Decoder: Interpreter (static)` (`Z0` breakpoints
are otherwise silently dead - `docs/reverse-engineering/rpcs3-debugger.md`,
"`Z0` breakpoints fire, but only under the interpreter") and arms a
breakpoint directly at `SpuLight_AddCandidate`'s own entry (`0x0040d990`)
rather than the thunk (`0x006778c8`) - since the thunk tail-calls without
pushing its own frame, `LR` read at the callee's entry is already the real
caller's own return address, one hop closer than breaking at the thunk
itself would be. Per hit: `LR` (mapped, approximately, onto the nearest
*preceding* of the thirteen call-site addresses this page's own table
names - a heuristic, not a proof, since function boundaries between two
named sites are not known without Ghidra, which this lane does not use),
`f1`/`f2` (the two float args `SpuLight_AddCandidate(D, w)` per
renderer.md's own read of the call - position/colour arrive via `vs34`/
`vs35`, vector registers the GDB `g` dump does not carry at all), and then
the record this specific call just wrote: the entry breakpoint is swapped
for a one-shot breakpoint at `LR` itself, so by the time that fires the
call has returned and `+0x2098`'s new count together with the record at
`+0x20a0 + (count-1)*0x20` is unambiguously *this* call's own output, not
some other thread's or a later call's. This reads one call at a time end to
end (arm entry -> catch -> arm `LR` -> catch -> read -> re-arm entry), so
unlike `hd-flare-owner-break.py`'s `Breaker.step_off` hop it does **not**
guarantee every call within one frame is sampled - concurrent or
rapid-fire calls between the `LR` catch and the entry re-arm are missed.
Documented as a limitation, not fixed this session: a representative
multi-frame sample is what this pass is after, not frame-complete coverage.

`out_dir` defaults to `data/traces/hd-spu-light-candidates/` - gitignored
`data/`, never `/tmp`. Same three preconditions as the script this extends:
the decrypted image, the `oag` input profile, Xvfb `:77` -
`scripts/rpcs3-drive.py preflight` checks all three.
"""

import importlib.util
import json
import struct
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
spec = importlib.util.spec_from_file_location("rpcs3_drive", ROOT / "scripts" / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)
from rpcs3_debugger import Debugger, REG_FPR, REG_LR  # noqa: E402

IMAGE = ROOT / "data/images/hdfury-ps3-eu-dec.iso"

# Same static pointer `rpcs3-spu-light-dump.py` already established live:
# `0x00f4b300`, never relocated.
POINTER_SLOT = 0x008B83B0

# The compacted, frustum-culled double buffer (`SpuLight_CompactVisibleCandidates`,
# `0x0040d728`) - unchanged from `rpcs3-spu-light-dump.py`.
INDEX_OFFSET = 0x2080
COMPANION_OFFSET = 0x2084  # +0x2084[slot] u32: that slot's own survivor count
SLOT_BASE_OFFSET = 0x80
SLOT_STRIDE = 0x1000
RECORD_STRIDE = 0x20
SLOT_CAP = SLOT_STRIDE // RECORD_STRIDE  # 128

# The 128-slot raw candidate list `SpuLight_AddCandidate` (`0x0040d990`)
# fills, upstream of compaction - never read live before this script.
CANDIDATE_COUNT_OFFSET = 0x2098
CANDIDATE_BASE_OFFSET = 0x20A0
CANDIDATE_CAP = 128  # renderer.md's own "128-slot candidate list"
CANDIDATE_BYTES = CANDIDATE_CAP * RECORD_STRIDE  # 0x1000, same size as a slot

# `ptr+0x2080 .. +0x20a0`, one contiguous read: index, both companion counts,
# and whatever sits between `+0x208c` and `+0x2098` (unidentified, captured
# raw regardless).
HEADER_OFFSET = INDEX_OFFSET
HEADER_BYTES = CANDIDATE_BASE_OFFSET - INDEX_OFFSET  # 0x20


def u32(b, off=0):
    return struct.unpack_from(">I", b, off)[0]


def decode_records(data, count, cap=SLOT_CAP):
    """`(valid_records, stale_records)` - `count` bounds validity, `cap` bounds
    how much of `data` is a real record at all (past `cap * 0x20` is out of
    range and not decoded)."""
    records = []
    usable = min(cap, len(data) // RECORD_STRIDE)
    for i in range(usable):
        chunk = data[i * RECORD_STRIDE : (i + 1) * RECORD_STRIDE]
        records.append(struct.unpack(">8f", chunk))
    valid = records[: max(0, min(count, usable))]
    stale = records[max(0, min(count, usable)) :]
    return valid, stale


def compare(visible, candidates):
    """Per visible record: 'exact' (all 8 fields byte-identical to some
    candidate), 'colour_range' ((r,g,b,D) - indices 4..7 - match some
    candidate, position may have moved a frame), or 'none'. Also whether the
    full visible sequence is an order-preserving subsequence of the
    candidate sequence under the 'exact' relation - the structural signature
    a sphere-test-and-copy compaction would leave."""
    results = []
    for rec in visible:
        tier = "none"
        for cand in candidates:
            if rec == cand:
                tier = "exact"
                break
            if rec[4:8] == cand[4:8]:
                tier = "colour_range"
        results.append(tier)

    subsequence = True
    cursor = 0
    for rec in visible:
        found = False
        while cursor < len(candidates):
            if rec == candidates[cursor]:
                found = True
                cursor += 1
                break
            cursor += 1
        if not found:
            subsequence = False
            break
    if not visible:
        subsequence = None  # vacuous - nothing to say

    return results, subsequence


def snapshot(dbg, out, tag):
    ptr = u32(dbg.read(POINTER_SLOT, 4))
    result = {"pointer_slot": "%08x" % POINTER_SLOT, "ptr": "%08x" % ptr}
    if not (0x00010000 <= ptr < 0x50000000):
        result["note"] = "pointer outside plausible main-RAM range"
        return result

    header = dbg.read(ptr + HEADER_OFFSET, HEADER_BYTES)
    (out / f"{tag}_header.bin").write_bytes(header)
    index = u32(header, 0)
    companion = [u32(header, 4), u32(header, 8)]
    candidate_count = u32(header, CANDIDATE_COUNT_OFFSET - HEADER_OFFSET)
    result["index_raw"] = index
    result["companion"] = companion
    result["candidate_count_raw"] = candidate_count

    if index > 0xFFFF or candidate_count > 0xFFFF:
        result["note"] = "index or candidate count implausibly large, not reading further"
        return result

    candidates_raw = dbg.read(ptr + CANDIDATE_BASE_OFFSET, CANDIDATE_BYTES)
    (out / f"{tag}_candidates.bin").write_bytes(candidates_raw)
    cand_valid, cand_stale = decode_records(candidates_raw, candidate_count, CANDIDATE_CAP)
    result["candidates_valid"] = cand_valid
    result["candidates_stale_count"] = len(cand_stale)

    slots = []
    for slot_idx in (0, 1):
        slot_raw = dbg.read(ptr + SLOT_BASE_OFFSET + slot_idx * SLOT_STRIDE, SLOT_STRIDE)
        (out / f"{tag}_slot{slot_idx}.bin").write_bytes(slot_raw)
        valid, stale = decode_records(slot_raw, companion[slot_idx], SLOT_CAP)
        tiers, subseq = compare(valid, cand_valid)
        slots.append({
            "slot": slot_idx,
            "is_current_index": slot_idx == index,
            "companion_count": companion[slot_idx],
            "valid_records": valid,
            "stale_record_count": len(stale),
            "match_tiers": tiers,
            "exact_count": tiers.count("exact"),
            "colour_range_count": tiers.count("colour_range"),
            "none_count": tiers.count("none"),
            "order_preserving_subsequence": subseq,
        })
    result["slots"] = slots
    return result


# Same carousel plan as `rpcs3-spu-light-dump.py` - Racebox's Track Creation
# carousel, 8 `right`s from its default highlight, reaching Amphiseum.
AMPHISEUM_PLAN = {
    "Main Menu": ["right"],
    "Track Creation": ["right"] * 8,
}

# `SpuLight_AddCandidate` itself (`0x0040d990`), not the one-instruction
# tail-call thunk `FUN_006778c8` - breaking here means `LR` is already the
# real caller's own return address (the thunk never pushes a frame).
ADD_CANDIDATE_ENTRY = 0x0040D990

# The thirteen reachable call sites, `docs/ghidra/functions/ps3-hdfury-eu/
# renderer.md`'s "All thirteen reachable `SpuLight_AddCandidate` call sites"
# table, sorted by address. `nearest_site()` below buckets a live `LR` onto
# the closest *preceding* entry - a heuristic, since the real boundary
# between one named function and the next unnamed code is not known without
# Ghidra, which this lane does not use.
KNOWN_SITES = sorted([
    0x000CFB80,  # FUN_000cfb80 - ship glow, full chain traced
    0x000E41B0,  # FUN_000e41b0 - second, RaceManager-gated call for the same ship light
    0x00108C48,  # FUN_00108c48 - double-gated highlight, fixed global colour
    0x00115028,  # FUN_00115028 - track-interval marker producer
    0x00122098,  # FUN_00122098 - collision/skid, decal-ring-buffer, speed-gated
    0x00123DE8,  # FUN_00123de8 - two constants and one transform
    0x00127468,  # FUN_00127468 - WeaponExplosions_Draw-adjacent
    0x001310E8,  # FUN_001310e8 - sibling of FUN_00122098, fixed D/colour
    0x0013B300,  # FUN_0013b300 - two independent lights, twin-mount shape
    0x0014A7D8,  # FUN_0014a7d8 - generic "submit at this transform" utility
    0x0014F928,  # FUN_0014f928 - curve-driven, caller-supplied colour
    0x001547C8,  # FUN_001547c8 - linear-in-one-field, bright flash/pickup shape
    0x00155568,  # FUN_00155568 - sibling of FUN_001547c8, 16-slot racer table
])


def nearest_site(lr):
    """`(site_addr, offset)` for the closest preceding entry in `KNOWN_SITES`,
    or `(None, None)` if `lr` sits before all of them."""
    site = None
    for addr in KNOWN_SITES:
        if addr <= lr:
            site = addr
        else:
            break
    if site is None:
        return None, None
    return site, lr - site


def attribute_hit(dbg, ptr, tid, regs, timeout=1.5):
    """One `(LR, f1, f2, record)` reading. `lr_confirmed` is `False` when the
    `LR` breakpoint never fired within `timeout` (the call site's own code
    path is unusual, or the target drifted) - the count/record read still
    happens, but callers should treat an unconfirmed hit as weaker evidence
    rather than discard it outright, since `count`/`record` are still a
    real read of *some* state, just not provably this exact call's own."""
    lr = int.from_bytes(regs[REG_LR : REG_LR + 8], "big") & 0xFFFFFFFF
    f1 = struct.unpack(">d", regs[REG_FPR + 1 * 8 : REG_FPR + 1 * 8 + 8])[0]
    f2 = struct.unpack(">d", regs[REG_FPR + 2 * 8 : REG_FPR + 2 * 8 + 8])[0]

    dbg.remove_breakpoint(ADD_CANDIDATE_ENTRY)
    dbg.add_breakpoint(lr)
    confirmed = False
    try:
        dbg.resume()
        reply = dbg.wait_for_stop(timeout=timeout)
        if reply is None:
            dbg.pause()
        dbg.drain()
        for t in dbg.threads():
            r = dbg.registers(t)
            if r is not None and int.from_bytes(r[512:520], "big") & 0xFFFFFFFF == lr:
                confirmed = True
                break
    finally:
        dbg.remove_breakpoint(lr)
        dbg.add_breakpoint(ADD_CANDIDATE_ENTRY)

    header = dbg.read(ptr + HEADER_OFFSET, HEADER_BYTES)
    count = u32(header, CANDIDATE_COUNT_OFFSET - HEADER_OFFSET)
    record = None
    if 0 < count <= CANDIDATE_CAP:
        raw = dbg.read(ptr + CANDIDATE_BASE_OFFSET + (count - 1) * RECORD_STRIDE, RECORD_STRIDE)
        record = struct.unpack(">8f", raw)
    site, offset = nearest_site(lr)
    return {
        "lr": "%#x" % lr,
        "lr_confirmed": confirmed,
        "f1_D": f1,
        "f2_w": f2,
        "nearest_site": ("%#x" % site) if site is not None else None,
        "site_offset": offset,
        "count_after": count,
        "record": record,
    }


def run_attribution(dbg, ptr, max_hits, time_budget_seconds):
    dbg.add_breakpoint(ADD_CANDIDATE_ENTRY)
    hits = []
    misses = 0
    deadline = time.time() + time_budget_seconds
    try:
        while len(hits) < max_hits and time.time() < deadline:
            tid, regs = dbg.wait_at(ADD_CANDIDATE_ENTRY, tries=20, slice_seconds=0.3)
            if tid is None:
                break
            hit = attribute_hit(dbg, ptr, tid, regs)
            if not hit["lr_confirmed"]:
                misses += 1
            hits.append(hit)
            print("hit %3d  lr=%s (near %s+%s)  D=%.4f w=%.4f  record=%s"
                  % (len(hits), hit["lr"], hit["nearest_site"], hit["site_offset"],
                     hit["f1_D"], hit["f2_w"], hit["record"]), flush=True)
    finally:
        try:
            dbg.remove_breakpoint(ADD_CANDIDATE_ENTRY)
        except Exception:
            pass
    return hits, misses


def summarise_attribution(hits):
    by_site = {}
    for h in hits:
        key = h["nearest_site"] or "unmapped"
        by_site.setdefault(key, []).append(h)
    summary = {}
    for site, group in by_site.items():
        records = [h["record"] for h in group if h["record"] is not None]
        summary[site] = {
            "hits": len(group),
            "D_values": sorted({round(r[7], 3) for r in records}) if records else [],
            "colours": sorted({(round(r[4], 2), round(r[5], 2), round(r[6], 2)) for r in records}) if records else [],
        }
    return summary


def main_attribute(argv):
    out = Path(argv[0] if argv else ROOT / "data/traces/hd-spu-light-candidates")
    max_hits = int(argv[1]) if len(argv) > 1 else 40
    out.mkdir(parents=True, exist_ok=True)
    with drive.Session(str(IMAGE), str(out / "logs-attribute"), interpreter=True) as session:
        print("rpcs3 pid %d (interpreter)" % session.proc.pid, flush=True)
        if not session.wait_for_screen("Main Menu", 180.0):
            print("never reached the Main Menu", file=sys.stderr)
            return 1
        time.sleep(12.0)
        if session.walk_to_race(plan=AMPHISEUM_PLAN) not in drive.RACE_ARRIVED:
            print("did not reach a race", file=sys.stderr)
            return 1
        print("in race; waiting for the load", flush=True)
        time.sleep(50.0)
        session.pad.set("cross", True)
        time.sleep(16.0)

        dbg = Debugger()
        try:
            ptr = u32(dbg.read(POINTER_SLOT, 4))
            print("ptr=%#x" % ptr, flush=True)
            hits, misses = run_attribution(dbg, ptr, max_hits, time_budget_seconds=240.0)
        finally:
            try:
                dbg.resume()
            except Exception:
                pass
            dbg.close()
        (out / "attribution.json").write_text(json.dumps({"hits": hits, "misses": misses}, indent=1))
        session.pad.set("cross", False)
        drive.screenshot(out / "attribution.png")
        print("done; %d hits, %d misses; artefacts in %s" % (len(hits), misses, out), flush=True)
        print(json.dumps(summarise_attribution(hits), indent=1))
    return 0


def analyze_dir(out):
    """Re-run `compare()` against already-saved `.bin` files, no emulator."""
    meta_path = out / "meta.json"
    if not meta_path.exists():
        print("no meta.json in %s - nothing to re-analyze" % out, file=sys.stderr)
        return 1
    metas = json.loads(meta_path.read_text())
    for m in metas:
        tag = m.get("tag")
        if tag is None or "slots" not in m:
            continue
        header = (out / f"{tag}_header.bin").read_bytes()
        candidate_count = u32(header, CANDIDATE_COUNT_OFFSET - HEADER_OFFSET)
        candidates_raw = (out / f"{tag}_candidates.bin").read_bytes()
        cand_valid, _ = decode_records(candidates_raw, candidate_count, CANDIDATE_CAP)
        for slot in m["slots"]:
            slot_raw = (out / f"{tag}_slot{slot['slot']}.bin").read_bytes()
            valid, _ = decode_records(slot_raw, slot["companion_count"], SLOT_CAP)
            tiers, subseq = compare(valid, cand_valid)
            slot["match_tiers"] = tiers
            slot["exact_count"] = tiers.count("exact")
            slot["colour_range_count"] = tiers.count("colour_range")
            slot["none_count"] = tiers.count("none")
            slot["order_preserving_subsequence"] = subseq
        print(tag, json.dumps(m["slots"], indent=1))
    meta_path.write_text(json.dumps(metas, indent=1))
    return 0


def main():
    if len(sys.argv) > 1 and sys.argv[1] == "analyze":
        out = Path(sys.argv[2] if len(sys.argv) > 2 else ROOT / "data/traces/hd-spu-light-candidates")
        return analyze_dir(out)
    if len(sys.argv) > 1 and sys.argv[1] == "attribute":
        return main_attribute(sys.argv[2:])

    out = Path(sys.argv[1] if len(sys.argv) > 1 else ROOT / "data/traces/hd-spu-light-candidates")
    out.mkdir(parents=True, exist_ok=True)
    with drive.Session(str(IMAGE), str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        if not session.wait_for_screen("Main Menu", 180.0):
            print("never reached the Main Menu", file=sys.stderr)
            return 1
        time.sleep(12.0)
        if session.walk_to_race(plan=AMPHISEUM_PLAN) not in drive.RACE_ARRIVED:
            print("did not reach a race", file=sys.stderr)
            return 1
        print("in race; waiting for the load", flush=True)
        time.sleep(50.0)
        session.pad.set("cross", True)
        time.sleep(16.0)

        dbg = Debugger()
        metas = []
        try:
            for i, wait in enumerate((0.0, 2.0, 2.0, 2.0, 2.0)):
                if wait:
                    time.sleep(wait)
                tag = "s%d" % i
                dbg.pause()
                metas.append({"tag": tag, "wall": time.time(), **snapshot(dbg, out, tag)})
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
        for m in metas:
            slots_summary = [
                (s["slot"], s["companion_count"], s["exact_count"], s["colour_range_count"], s["none_count"])
                for s in m.get("slots", [])
            ]
            print(m["tag"], "candidate_count=%s" % m.get("candidate_count_raw"), slots_summary)
    return 0


if __name__ == "__main__":
    sys.exit(main())
