# Omega 1.07: where the composite's `ColorScale` comes from

Evidence page for the 1.07 patch composite's new `ColorScale` (float3). Static reading of the **patch** `eboot.bin`
(`data/extracted/ps4/omega-eu-patch/uroot/eboot.bin`), which is not in the Ghidra project; no live readout (shadPS4 has
no memory readout). No `names.tsv` rows: every address here is in the patch, and `names.tsv` replays into the base
import.

**Address convention.** Text virtual address = file offset - `0x5430`; data virtual address `0x99c000` = file offset
`0x99d990`; Ghidra-style = va + `0x01000000` (same as [fe-style-colours.md](fe-style-colours.md)). Addresses below are
virtual addresses without the image base.

## Result

**Wiring path: confidence 70. Resting value `(1, 1, 1)`: confidence 65. The value under a fade or on the second branch
is not read.**

1. The patch's composite constructor looks the member up by name: `"ColorScale"` (string at va `0x890f2e`, lea at
   `0x643d20`, call `0x7e87a0`) and stores the handle at `0x9a8168` (`0x643d33`). The base build has no such lookup (its
   only `*ColorScale` name is `cAccum_ColorScale`, see below).
2. The handle has exactly one load: `0x66405f`, in the composite's per-frame constants. It writes **three floats**
   (`vmovss` + two `vextractps`, `0x6640a1`-`0x6640ae`) from a float4 spilled at `0x6637b6` that was loaded at `0x66379e`
   from a global at **`0x104a520`**. There is no constant default in the constructor: the patch's constructor sets the
   same four zero fields the base's does (`0x644325`-`0x64434b`) and nothing for `ColorScale`.
3. `0x104a520` has two writers: the init at `0x65c647`-`0x65c65f`, which copies the constant at `0x857a20`, read as
   **`(1.0, 1.0, 1.0, 1.0)`**, and the per-frame function at `0x649e10` (store at `0x64a091`). That function picks one of
   three sources:
   - the **default path**: reload the float4 held at `0x121b8b0` unchanged;
   - a constant: the scalar at `0x86344c`, read as **`1.0`**, broadcast to `(1, 1, 1)` with a `w` of the same constant;
   - a computed scalar from two float4s (squares, rsqrt, a reciprocal), broadcast.

   The branch is taken only when byte `0x103ec39` is set **and** bit 0 of `0x103ec38` is set **and** dword `0x103ed10`
   is above 1; otherwise the default path runs. Those three are unread (see Open).
4. The held float4 at `0x121b8b0` is written by a fade update at `0x5bdec0` (store `0x5bdf3f`): `v = max(0, hi_dword
   + dt * -5.0)` (constant at `0x863044`) broadcast to four lanes, and saved and restored through an object's `+0x1a0` /
   `+0x1b0` (`0x5be6a4`). Several other sites (`0x3098ab`, `0x312d6f`, `0x64a612`, `0x75032e`, `0x769505`) store the
   constant `(1, 1, 1, 1)` after a guard call, one each (constants read at `0x847cf0`, `0x847e60`, `0x857660`,
   `0x85ba10`, `0x85c250`, all `(1,1,1,1)`).

**Reading.** `ColorScale` is a global colour multiplier that rests at `(1, 1, 1)` and is driven toward a lower value
by a fade (rate `-5.0` per second on a scalar broadcast to all channels). So the composite
`out = ColorScale * vignette * (Frame + LowResAdditive + BloomScale * Bloom) + ScreenTint`
is the base's picture when no fade runs. **To wire**: pass `(1, 1, 1)`; the tint's placement outside the vignette is the
only visible change, and only once `oag_post::omega_tonemap` draws tint and vignette at all.

## Not what it is

`cAccum_ColorScale` (base `FUN_016110f0`, handle `DAT_01940594`, written in `FUN_01625750`; the patch's at `0x6640b8`)
is **not** this member. It is two floats `(a, 1 / (a + b))`, a motion-blur accumulation weight.

## Open

- What the gate `0x103ec39` / `0x103ec38` bit 0 / `0x103ed10 > 1` is, and so which branch a normal single-player
  race takes (a viewport count is a guess; the executable also carries `sceHmd*` strings, so VR is a candidate, not
  read). Next address: the writers of `0x103ed10`.
- What triggers the `0x5bdec0` fade (fade-in at race start is a guess, not read), and whether any value other than 1
  reaches the composite in a race. A live shadPS4 frame at race start would show a darkening ramp if it does.
- Whether the base's `0x121b8b0` equivalent exists: the AVX block `0x649eee`-`0x649f36` was not byte-searched in the
  base program.

## 2048 and HD cross-check

2048: `grep -c ColorScale` on `patch-v104/eboot.elf` is 0; its composite is a USSE `.gxp`, whose member names are not
read (`gxp.md`). **Not checkable by name; no equivalent found.** HD/Fury: not checked (the base Omega composite has
no `ColorScale` either, so HD's, which Omega's base follows, does not carry it in this form).
