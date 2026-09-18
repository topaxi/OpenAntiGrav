# `AmbientLight`, `DirectionalLight` and `PointLight`: the track's light rig

The three authored light classes small enough to decode without a batch
decoder. `docs/formats/vex.md` has only ever recorded that `0x12c`, `0x131`
and `0x132` *have* these names, from the class-ID table. No byte of any of
the three payloads was documented before this page.

A Ghidra pass and a live capture (2026-08-05, see
[Open](#open)) since answered most of "how the original draws it." Both
handlers are now recovered (`AmbientLight_RegisterClass`,
`DirectionalLight_RegisterClass`, confidence 90 each; see
[`lighting.md`](../ghidra/functions/psp-pulse-eu/lighting.md)); what they
lead to differs by class - `PointLight` has no registration site at all,
`AmbientLight`'s decoded colour is live-verified to never reach the render
path, and `DirectionalLight` is live-verified to be collected into a capped
list at track load from real authored data, with its final reader still
unfound. So, as with
[`Skycube`/`fogCube`](skycube.md), the confidence scores below are about
*what the data is*; the render-side confidence sits in the linked pages.

## Summary

| Claim | Confidence |
| --- | --- |
| `AmbientLight`/`DirectionalLight` payload is `{r, g, b, intensity}`, 16 bytes | **88** |
| `PointLight` payload is `{r, g, b, range}` then a `{1, 0, 0, 0}` trailer, 32 bytes | **86** |
| Placement is the node's transform chain, not the payload | **85** |
| `Dynamic Point Light` `0x3c2` is never authored anywhere on the PSP disc | **90** |
| `PointLight` has no `Vex_RegisterClass` call site on either binary - authored but never handled | **90** (exhaustive: all 46 sites read, both binaries) |
| `AmbientLight`'s decoded colour never reaches the one global its only two known consumers read - confirmed inert | **live-verified** |
| `DirectionalLight` is live-verified collected into a 4-entry list at track load, matching real authored data exactly - the reader is still unfound | **live-verified (collection); reader not recovered** |

Validated by `crates/vex/tests/lighting_ground_truth.rs` against every
`.vex` file in the PSP disc's `Data.wad` (1142 entries, `just test-data`).

## Why 88/86, not the 90-95 `Skycube`/`fogCube` reached

`Skycube` closes because a wrong material count or stride visibly fails to
land the geometry offset on its own alignment boundary - an arithmetic
coincidence a misreading cannot fake. `fogCube` closes because its matrix's
own last row has to read `w == 1.0`. **Neither light payload has anything
like that.** There is no second field whose value depends on getting the
first one right, and no self-check that a wrong stride would visibly fail.

What is measured instead, across every instance on the disc:

- The payload length is **exactly** 16 (`AmbientLight`, `DirectionalLight`) or
  32 (`PointLight`) bytes on every sample, with no variation - asserted as an
  exact length set, not "at least".
- Every field decodes to a finite, sane value: colour channels finite and
  non-negative, intensity/range finite and strictly positive.
- `PointLight`'s trailer reads `{1, 0, 0, 0}` on all 10 shipped samples, with
  zero variation - which is itself evidence the four `u32` are a real,
  authored field rather than uninitialised padding, even though what they
  mean is not decoded.

That is a real anchor - a wrong stride does not produce four fields that are
all finite and positive on every one of 106 samples by chance - but it is
honestly weaker than a closure argument, hence 88/86 rather than 90+.

## Layout

```text
AmbientLight / DirectionalLight, 16 bytes:
  +0x00  f32[3]  colour, linear RGB
  +0x0c  f32     intensity

PointLight, 32 bytes:
  +0x00  f32[3]  colour, linear RGB
  +0x0c  f32     range
  +0x10  u32[4]  trailer, {1, 0, 0, 0} on every shipped sample, undecoded
```

| Offset | Field | Type |
| --- | --- | --- |
| `+0x00` | `r` | `f32` |
| `+0x04` | `g` | `f32` |
| `+0x08` | `b` | `f32` |
| `+0x0c` | `intensity` (Ambient/Directional) or `range` (Point) | `f32` |

`PointLight` continues:

| Offset | Field | Type |
| --- | --- | --- |
| `+0x10` | trailer word 0, `1` on every sample | `u32` |
| `+0x14` | trailer word 1, `0` on every sample | `u32` |
| `+0x18` | trailer word 2, `0` on every sample | `u32` |
| `+0x1c` | trailer word 3, `0` on every sample | `u32` |

## Placement is the transform chain, not the payload

Unlike a locator class such as `Engine Flare` or `Start Position`, none of
these three payloads is itself a 4x4 matrix - both are far too short: 16 or
32 bytes against the 64 a matrix needs. So a light's world transform comes
from `vex::world_transforms`, the full per-node parent chain that contributes
the identity for any node that is not itself a `Transform`, the same source
[`pads::volumes`](pads.md) uses and for the same reason.

**Not `vex::class_world_transforms`.** That helper reads a matching node's
*own* payload as the matrix - correct for a locator, wrong here. Handed a
16- or 32-byte light payload, it sees a non-empty payload under 64 bytes,
decodes nothing, and the light is silently dropped from the result. The
failure mode is not a wrong matrix, it is every light on every track going
missing at once - `crates/vex/src/lighting.rs`'s module docs name this
explicitly, because it is the trap this format falls into if approached the
way `Skycube` was.

## What the sweeps found

Two sweeps run against `pulse-psp-usa.chd`, both in
`lighting_ground_truth.rs`:

- **Track-scoped** (the 40 known `Data\Environments\*\{track,zone_track}{,_reversed}.vex`
  files, name-hash resolved): **74 `AmbientLight`, 86 `DirectionalLight`,
  10 `PointLight`.**
- **Primary, full-disc** (all 1142 `Data.wad` entries, unfiltered by name):
  **106 `AmbientLight`, 114 `DirectionalLight`, 10 `PointLight`, 0
  `Dynamic Point Light`.**

The full sweep finds **32 more `AmbientLight` and 28 more `DirectionalLight`**
than the track-scoped one - `AmbientLight` and `DirectionalLight` are generic
scene classes, and they are genuinely authored off-track. Name-hash
resolution against the 28-entry block responsible for most of that excess
identifies it as **front-end ship display models** - `ship_FE.vex`, confirmed
for at least six of the eight teams (EGX, Feisar, Goteki, Piranha, Qirex,
Triakis) - each carrying exactly one `AmbientLight` and one `DirectionalLight`
to light the menu-screen ship carousel. The remaining four extra
`AmbientLight` instances (with no matching `DirectionalLight`) sit on two
further unresolved `.vex` entries near `01_Track`'s own block, not one of the
four track/zone file names this survey filters for. **`PointLight` and
`Dynamic Point Light` show no off-track instance at all** - the point lights
cluster on exactly the same two files in both sweeps.

**Point lights cluster on exactly 2 distinct source files, 5 each** - both
sweeps agree, `13_Track`/`13_Track_reversed` per the plan that opened this
work. Asserted structurally (a source-index to count map with exactly 2 keys,
each `5`) rather than by hardcoding which two files.

**Intensity is not clamped to `0..=1`.** The full sweep's own maximum colour
channel and maximum `AmbientLight`/`DirectionalLight` intensity both read
**5.7854**; `PointLight` range runs up to 400. See [Open](#open) for the
tension this raises.

**Direction-basis survey, track-scoped:** of the 86 `DirectionalLight`
instances, **0 sit under an identity-rotation parent** - every one carries a
genuine, non-trivial orientation, and every one's row 2 has length 1.0 (no
scale anywhere in the chain). That rules out "the direction is always the
identity, so the question is moot" and confirms that if the runtime does read
direction from the rotation basis, no normalisation step would be needed to
use it. It does not by itself settle rotation-row versus translation-row -
see [Open](#open).

**Per-file density** (files that author at least one, full sweep):
`AmbientLight` 1 to 7 per file (mean 1.56), `DirectionalLight` 1 to 3 per file
(mean 1.68), `PointLight` exactly 5 per file wherever it appears.

## Open

**Updated 2026-08-05 - the Ghidra pass this page called a "documented
follow-up" has now run, in `/pulse/BOOT-psp-pulse-eu.BIN` under the reversed
target-of-record policy (see `HANDOVER.md`).** Full account:
[`docs/ghidra/functions/psp-pulse-eu/lighting.md`](../ghidra/functions/psp-pulse-eu/lighting.md).
Summary, so this page doesn't go stale in place:

- **`AmbientLight` and `DirectionalLight` are registered** -
  `AmbientLight_RegisterClass` (EU `0x0892d960`, confidence 90) and
  `DirectionalLight_RegisterClass` (EU `0x08934fc8`, 90), both cross-verified
  against `psp-pulse-usa`. **`PointLight` is not** - exhaustively checked, all
  46 `Vex_RegisterClass` call sites read on both binaries, `0x132` appears on
  none of them. Joins `engine_fire`/`exitglow`/`gate` on the "authored but
  never registered" list.
- **The per-class `init`/bind body is unreachable through the stored method
  pointer.** Every class's method-table population goes through a
  compiler-emitted "return &self" stub, confirmed project-wide (not a
  lighting-specific quirk) on four samples across both binaries - so the
  question below about which part of the matrix `DirectionalLight` reads is
  **still open**, and could not be settled this pass despite the
  registration functions being found.
- **The bigger finding**: following `Gu_Ambient`'s own callers (rather than
  the unreachable init path) found the *real* ambient consumers -
  `Mesh_ApplyMaterialLighting` and `Mesh_ApplyShinemapReflection_q` - and
  both read ambient as **one flat global RGB triple**, and both **explicitly
  disable all four `GU_LIGHT0..3` hardware slots** even when lighting is on.
  Every sibling display-list-builder function was checked too; none enables
  them. This is evidence against, not merely absence of evidence for, a
  hardware-light-slot selection mechanism in the mesh draw path - it directly
  answers this project's own reason for opening the Ghidra pass (`13_Track`'s
  8 authored lights against 4 hardware slots) with "the mesh path doesn't use
  the slots at all," not "here is the selector."
- **Settled, and the answer is no.** A live PPSSPP capture (2026-08-05,
  headless/Xvfb) read the global at five points across two tracks with
  sharply different authored `AmbientLight` colours (Talon's Junction
  `(0.169, 0.193, 0.207)`, Moa Therma `(0.297, 0.258, 0.217)`/
  `(0.440, 0.375, 0.348)`) - it stayed at the compiled default `(0.2, 0.8,
  1.0)` every time, fresh boot through five in-race checkpoints. So this
  class's decoded colour is confirmed inert on the render side, the same
  standing `PointLight` already has from its missing registration. See
  [`lighting.md`](../ghidra/functions/psp-pulse-eu/lighting.md) for the full
  capture record.
- **The `PointLight` trailer's meaning.** Four `u32`, `{1, 0, 0, 0}` on every
  one of the 10 shipped samples. Constant across every sample is itself
  suspicious of a flag or an enabled-state word rather than a per-light
  parameter, but that is a guess with nothing in the payload to test it
  against - a wider sample would need a track that varies it, and none does.
- **Whether `DirectionalLight`'s direction comes from the rotation basis or
  the translation row of its world matrix.** Not settled by data alone. The
  direction-basis survey above establishes that every shipped instance has a
  genuine (non-identity, unscaled) rotation, which is a necessary condition
  for "direction = a rotation row" to be a meaningful reading at all - but it
  does not distinguish that reading from "direction = translation, normalised
  toward the origin" or some other rule entirely. **New evidence, not yet a
  full answer**: a live capture found a unit-length float vector at `+0x60`
  in the *runtime* object (see
  [`lighting.md`](../ghidra/functions/psp-pulse-eu/lighting.md#the-object-itself-and-three-passes-that-ruled-out-every-lead-but-one)),
  16 bytes past the on-disk 16-byte payload the object copies in at
  construction. Since the on-disk payload has no direction field, that
  vector's mere existence confirms something computes a direction once, from
  the node's world transform, at construction or load time - which row of
  the matrix it reads is still open, and `DirectionalLight_Init`
  (`0x08934cb0`) is now the concrete next place to look for that specific
  read, rather than an unnamed `init` handler.
- **The intensity range tension.** Real observed intensities and colour
  channels reach 5.7854, and `PointLight` range reaches 400. Neither is
  clamped in this decoder, on the reasoning that a clamp here would be
  guessing at a runtime behaviour never observed. But the PSP GE's own light
  colour registers are 8-bit-per-channel, so somewhere between an authored
  5.7854 and a byte the runtime tone-maps, clamps, or otherwise transforms
  the value - and that transform is exactly what is not recovered. Recorded
  as a tension, not resolved here.
- **The off-track excess.** The full sweep's 32 extra `AmbientLight` and 28
  extra `DirectionalLight` instances are mostly explained by front-end
  `ship_FE.vex` models (see above), plus 4 further `AmbientLight` instances on
  two unidentified `.vex` entries. Which entries those two are, and what kind
  of asset they belong to, is not resolved here.
- **`Dynamic Point Light` `0x3c2` is authored nowhere on the disc**, checked
  now across all 1142 `Data.wad` entries rather than just the 40 track files
  an earlier census covered. Kept out of render scope on that basis - there
  is no shipped data for the render side to point at, moving or otherwise.
- **`PointLight` `0x132` on Wipeout HD/Fury: authored (1,160 nodes, correctly
  classed) but converged-negative on the consumer side.** HD's own
  `g_VexClassTable` (confidence 95,
  [`vex-classes.md`](../ghidra/functions/ps3-hdfury-eu/vex-classes.md))
  confirms `0x132` is genuinely `PointLight` there too, so the node census
  above is real data - but HD ships no `PointLight_Importer.cpp` (Pulse-only
  in the by-name importer comparison,
  [`renderer.md`](../ghidra/functions/ps3-hdfury-eu/renderer.md)), and a
  disc-wide name-hash sweep of every `SHO` shader block on
  `hdfury-ps3-eu-dec.iso` - every `.rcsmaterial`'s compiled variants plus
  every block resident in `EBOOT.elf`, 97,861 parsed, none failed
  (confidence 85, same page, "A disc-wide name-hash sweep for `pointLight0*`
  finds no consumer anywhere",
  [`scripts/hd-pointlight-sweep.py`](../../scripts/hd-pointlight-sweep.py))
  - found zero references to `pointLight0PositionWorldSpace`/
  `pointLight0Colour`/`pointLight0Falloff` anywhere, despite all three
  being real, named slots in HD's own 81-entry engine parameter table
  (cross-checked against two known-positive controls, one per branch the
  method checks - `constantAmbientColour` and `positionScale` - both found
  broadly and plausibly used; see the page for a register-based first
  attempt that got this wrong and why). Pulse and HD
  now read the same way on this feature by two different methods on two
  different binaries: the authoring class exists, nothing consumes it. Not a
  render-side gap to fill; see `renderer.md` for what would overturn this
  (mainly: a runtime trace, not attempted here).
- **PS2 parity.** Not checked. Pulse's class IDs do not carry to Pure, and the
  PS2 build's numbering is unconfirmed - the same caveat
  [`skycube.md`](skycube.md) already carries for its own two classes.
