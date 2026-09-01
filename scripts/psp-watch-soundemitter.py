#!/usr/bin/env python3
"""Runtime-verify the positional-audio law at a `SoundEmitter_ComputeVolumeAndAngle`
breakpoint.

`docs/ghidra/functions/psp-pulse-usa/positional-audio.md` reads the whole pan
and attenuation law from static decompilation and says plainly: nothing on
that page is runtime-verified. This is the cheapest upgrade the page names -
one breakpoint, live craft, compare the closed form against what the game
actually computes.

`SoundEmitter_ComputeVolumeAndAngle` (`0x08939c00`) is
`(instance, emitter, &volume_out, &angle_out)`. Its two out-parameters are
stack slots in the caller (`SoundInstance_UpdateSpatial`), which is why this
script cannot just read them after resuming: it has to catch the CPU again
before that stack space is reused. **v1.20.4 only ever fires the
most-recently-armed execution breakpoint** (see
`ppsspp-debugger.md#each_hit_any`), so the two breakpoints this needs - entry
and return address - are armed one at a time, never together: read the inputs
at entry, swap the breakpoint to `ra`, resume, read the outputs, swap back.

`_DAT_002bde10` in the decompile is unrelocated the same way call targets are
(see positional-audio.md's own warning), so its address is
`0x08804000 + 0x002bde10 = 0x08ac1e10` - but the disassembly
(`lui s4,0x2c` / `lw a0,-0x21f0(s4)` / `addiu a0,a0,0x40`) shows that address
holds a *pointer to* the sound manager, not the manager itself, same shape as
`G_STATE_MACHINE` in `ppsspp_debugger.py`. The first read this script does is
a fingerprint of that relocation and dereference - the volume-curve gamma and
table - because if either is wrong everything downstream is nonsense read
from the wrong address, not a subtly wrong law.

    uv run --with websocket-client scripts/psp-watch-soundemitter.py \\
        --port 47820 --hits 400 --out /tmp/soundemitter.csv
"""

import argparse
import csv
import math
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger

ENTRY = 0x08939C00  # SoundEmitter_ComputeVolumeAndAngle
MGR_PTR = 0x08AC1E10  # _DAT_002bde10 + image base 0x08804000; holds *the* manager pointer


def read_gpr(dbg):
    regs = dbg.call("cpu.getAllRegs")
    gpr = next(c for c in regs["categories"] if c["name"] == "GPR")
    return dict(zip(gpr["registerNames"], gpr["uintValues"]))


def read_i32(dbg, address):
    (value,) = struct.unpack("<i", dbg.read(address, 4))
    return value


def read_mgr(dbg):
    """Dereference the manager pointer at `MGR_PTR`. Re-read every time this is
    called from the main loop, since it is a live global; only assumed fixed
    within the fingerprint check below."""
    mgr = dbg.read_u32(MGR_PTR)
    if not 0x08000000 <= mgr < 0x0A000000:
        raise SystemExit(
            f"*MGR_PTR (0x{MGR_PTR:08x}) = 0x{mgr:08x} is not a plausible RAM "
            f"pointer. Stop and recheck the _DAT_002bde10 math."
        )
    return mgr


def check_volume_curve(dbg, mgr):
    """Fingerprint the MGR relocation: gamma and the 256-entry curve.

    Confirms `_DAT_002bde10 + image base`, dereferenced, lands on the sound
    manager, and runtime-verifies `SoundManager_BuildVolumeCurve`'s law
    (`curve[i] = (i/255)**(1/gamma)`) in the same read.
    """
    gamma = dbg.read_f32(mgr + 0x168)
    curve = dbg.read_f32s(mgr + 0x16C, 256)
    if abs(gamma - 1.7) > 1e-4:
        raise SystemExit(
            f"MGR relocation looks wrong: gamma read as {gamma!r}, expected 1.7 "
            f"at 0x{mgr + 0x168:08x}. Stop and recheck the _DAT_002bde10 math "
            f"before trusting anything downstream."
        )
    worst = 0.0
    for i, v in enumerate(curve):
        expected = 0.0 if i == 0 else (i / 255.0) ** (1.0 / gamma)
        worst = max(worst, abs(v - expected))
    print(f"MGR relocation confirmed: mgr=0x{mgr:08x}, gamma={gamma}, curve max |actual-expected|={worst:.6g}")
    return curve


def expected_volume(distance, radius, cone_enabled, cone_angle, cone_half, request_volume, curve):
    if distance > radius:
        return 0
    atten = (radius - distance) / radius
    if cone_enabled:
        atten *= 1.0 - cone_angle / cone_half
    atten = min(atten * 1.25, 1.0)
    v = atten * request_volume
    v = max(0.0, min(1.0, v))
    index = int(v * 255.0)  # truncated, not rounded - matches SoundManager_VolumeCurve
    index = max(0, min(255, index))
    return int(curve[index] * 1024.0)


def expected_angle(direction, right):
    length = math.sqrt(sum(c * c for c in direction))
    if length == 0.0:
        normalized = (0.0, 0.0, 0.0)
    else:
        normalized = tuple(c / length for c in direction)
    c = sum(a * b for a, b in zip(normalized, right))
    c = max(-1.0, min(1.0, c))
    phi = math.degrees(math.acos(c)) - 90.0
    if phi < 0.0:
        phi += 360.0
    return int(phi)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, default=47800)
    parser.add_argument("--hits", type=int, default=200)
    parser.add_argument("--out", default="/tmp/soundemitter.csv")
    args = parser.parse_args()

    dbg = Debugger(args.port)
    dbg.brk()

    mgr0 = read_mgr(dbg)
    curve = check_volume_curve(dbg, mgr0)

    rows = []
    dbg.add_breakpoint(ENTRY)
    armed_entry = True
    for i in range(args.hits):
        dbg.call("cpu.resume")
        dbg.wait_for_break(ENTRY)
        regs = read_gpr(dbg)
        a0, a1, a2, a3, ra = regs["a0"], regs["a1"], regs["a2"], regs["a3"], regs["ra"]

        request_volume = dbg.read_f32(a0 + 0x08)
        doppler_scale = dbg.read_f32(a0 + 0x0C)

        distance = dbg.read_f32(a1 + 0x30)
        radius = dbg.read_f32(a1 + 0x38)
        cone_half = dbg.read_f32(a1 + 0x40)
        cone_angle = dbg.read_f32(a1 + 0x48)
        cone_enabled = dbg.read_u8(a1 + 0x4C) != 0
        direction = dbg.read_f32s(a1 + 0x10, 3)
        mgr = read_mgr(dbg)
        right = (dbg.read_f32(mgr + 0x40), dbg.read_f32(mgr + 0x50), dbg.read_f32(mgr + 0x60))

        dbg.remove_breakpoint(ENTRY)
        dbg.add_breakpoint(ra)
        armed_entry = False
        dbg.call("cpu.resume")
        dbg.wait_for_break(ra)

        volume_actual = read_i32(dbg, a2)
        angle_actual = read_i32(dbg, a3)

        dbg.remove_breakpoint(ra)
        dbg.add_breakpoint(ENTRY)
        armed_entry = True

        volume_wanted = expected_volume(
            distance, radius, cone_enabled, cone_angle, cone_half, request_volume, curve
        )
        angle_wanted = expected_angle(direction, right)

        row = dict(
            emitter=hex(a1),
            distance=distance,
            radius=radius,
            d_over_r=distance / radius if radius else float("nan"),
            cone_enabled=cone_enabled,
            request_volume=request_volume,
            doppler_scale=doppler_scale,
            volume_actual=volume_actual,
            volume_wanted=volume_wanted,
            volume_diff=volume_actual - volume_wanted,
            angle_actual=angle_actual,
            angle_wanted=angle_wanted,
            angle_diff=angle_actual - angle_wanted,
        )
        rows.append(row)
        print(
            f"{i:4d} e={row['emitter']:>10} d/r={row['d_over_r']:.3f} "
            f"vol {volume_actual:5d} vs {volume_wanted:5d} (diff {row['volume_diff']:+d})  "
            f"ang {angle_actual:4d} vs {angle_wanted:4d} (diff {row['angle_diff']:+d})"
        )

    if armed_entry:
        dbg.brk()
        dbg.remove_breakpoint(ENTRY)
    dbg.resume()
    dbg.close()

    with open(args.out, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        writer.writeheader()
        writer.writerows(rows)
    print(f"wrote {len(rows)} rows to {args.out}")

    vol_exact = sum(1 for r in rows if r["volume_diff"] == 0)
    ang_within_1 = sum(1 for r in rows if abs(r["angle_diff"]) <= 1)
    print(f"volume exact match: {vol_exact}/{len(rows)}")
    print(f"angle within +-1:   {ang_within_1}/{len(rows)}")

    regimes = {"clamped (d<0.2r)": 0, "ramp (0.2r<=d<r)": 0, "gated (d>=r)": 0}
    for r in rows:
        dr = r["d_over_r"]
        if dr >= 1.0:
            regimes["gated (d>=r)"] += 1
        elif dr < 0.2:
            regimes["clamped (d<0.2r)"] += 1
        else:
            regimes["ramp (0.2r<=d<r)"] += 1
    print("regime coverage:", regimes)


if __name__ == "__main__":
    main()
