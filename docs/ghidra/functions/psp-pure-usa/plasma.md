# Pure's Plasma blast: three key tables, an eased scale and a 2.0 s life

**Binary:** `psp-pure-usa` `BOOT.BIN`, image base `0x08804000`, Ghidra
`/pure/BOOT-psp-pure-usa.BIN`. Read 2026-11-03 (lane `pure-weapon-visuals`), static
only (decompile plus disassembly through the bridge, `read_memory` for constants). Companion to
Pulse's [plasma.md](../psp-pulse-usa/plasma.md), whose GE-measured ramps this page reproduces
from Pure's own constants. Decompile-level readings: 85 is the ceiling; the key
mechanism sits at 85 because Pulse's identical code is confirmed by GE dumps.

`jal` operands and `_DAT_00xxxxxx` in the decompiler are image-base-relative: add `0x08804000`.

## Functions

| Address | Name | Confidence | What it is |
| --- | --- | ---: | --- |
| `0x0885b2a0` | `PlasmaBlast_Construct` | 85 | Builds the blast object (`0x170` bytes): loads three models, fills three key tables and three eased-scale triples. The addresses `0x0885b4f4`, `0x0885b5b0`, `0x0885b66c` that earlier notes called three loaders are call sites inside this one body (`0x0885b2a0`-`0x0885b9cb`). |
| `0x0885ae9c` | `PlasmaBlast_Update` | 85 | The object's per-tick update, vtable word `0x56e9c` (+ base) in the vtable at `0x08abc7c8`. Evaluates the key tables, eases the scale, scrubs the animation time, retires at 2.0 s. |
| `0x08a32e94` | `AnimKeyTrack_Construct` | 80 | Element constructor passed to the array constructor `FUN_088e9b78(obj+0x90, 3, 0x30, ...)`: writes key count `3` at `+0x24`, wrap period `0.0` at `+0x28`, `1.0` at `+0x2c`. |
| `0x08854b70` | `PlasmaBlast_Spawn` | 80 | Allocates `0x170` bytes and calls the constructor at the position `a1`. Three callers. |
| `0x088552a0` | `Plasmas_Update` | 75 | The bolt pool's per-tick update (the structural twin of Pulse's `Plasmas_Update`); spawns the blast at a bolt's end. |

## The three models and which key table drives which

`PlasmaBlast_Construct` loads, in this order, into the object's `+0x54`, `+0x58`, `+0x5c`:

| Index | Model (string) | Key table | Tint keys (255 is full) | Eased scale target, rate |
| --- | --- | --- | --- | --- |
| 0 | `Data\Weapons\plasma_halo.vex` (`0x08a48724`) | `+0x90` | `255, 255, 0` at `0, 0.2, 0.5` s | `15.0`, `0.1` |
| 1 | `Data\Weapons\plasma_hemisphere_noglow.vex` (`0x08a48744`) | `+0xc0` | `255, 255, 0` at `0, 0.3, 1.0` s | `8.0`, `0.07` |
| 2 | `Data\Weapons\plasma_hemisphere.vex` (`0x08a48770`) | `+0xf0` | `255, 0, 0` at `0, 1.0, 0` s | `7.8`, `0.07` |

Constants read from `.rodata` with `read_memory` (`0x08a8e5dc`-`0x08a8e620`): `0, 15.0, 0.5, 0.2, 0, 8.0, 1.0, 0.3, 0, 7.8, 1.0, 1.0, 1.0, 1.0, 0.1, 0.07, 0.07, -3.5`.
Key values are the immediates `0x437f0000` (255.0) and `0`. Table 2's third key has time `0` and value `0`
because the constructor never writes it (the array is zeroed by `AnimKeyTrack_Construct`'s `memset`s): an
evaluation past time `1.0` finds no key with `t <= time` and falls to the last value, `0`.

Confidence **85**: the tables, their count (`3`, from `AnimKeyTrack_Construct`) and the evaluator are read in full, and
the same evaluator and constructor shape in Pulse (below) reproduce Pulse's GE-measured ramps exactly.

## The key evaluator (in `PlasmaBlast_Update`)

For model `i` with `t = age` (`blast+0x50`, seconds since spawn):

```
if period (+0x28) > 0: t = frac(t / period) * (+0x2c)      // period is 0: skipped
find the first k in 1..count-1 with t <= time[k]
  v = value[k-1] + (value[k] - value[k-1]) * (t - time[k-1]) * (1 / (time[k] - time[k-1]))
if none: v = value[count-1]
tint = ((int)v & 0xff) * 0x10101 + 0xff000000          // opaque grey, stored per submesh group
```

`1 / (time[k] - time[k-1])` is precomputed by the constructor at `+0xa8 + 4k` (table 0), `+0xd8 + 4k`, `+0x108 + 4k`.
The tint is written by `FUN_08863898(model, colour)` (Pulse's `FUN_089122b4`, per-group `+0x6c`). The resulting
grey, as a fraction of white, per model and age:

- halo: `1` to age `0.2`, then linear to `0` at `0.5`, `0` after.
- noglow hemisphere: `1` to `0.3`, then linear to `0` at `1.0`.
- hemisphere: linear `1 - age` to `0` at `1.0`.

**Same law as Pulse, different key times.** Pulse's constructor (`0x0885fd90`) fills the same slots from
`DAT_08ab0f40/f3c`, `f50/f4c`, `f5c`, which read `0.2, 1.0`, `0.3, 1.5`, `1.0`; its element constructor `0x0885fd20`
also writes count `3`. Those give Pulse's halo `min(1, 1.25 (1 - age))`, `hemisphere2` `min(1, (1.5 - age) / 1.2)` and
`hemisphere1` `1 - age`, which is what the 2026-10-30 GE dumps measured. So the mechanism is live-confirmed on the
sibling binary; Pure's halo ends at 0.5 s (Pulse 1.0) and its noglow hemisphere at 1.0 s (Pulse 1.5), the hemisphere
is unchanged (1.0).

## The eased scale (`+0x6c..+0x8c`)

Each model has a triple `(target, current, rate)` at `+0x6c + 12 i`; Pure stores `(15, 0, 0.1)`, `(8, 0, 0.07)`,
`(7.8, 0, 0.07)`. Per update, `n = (int)(dt / 0.016666668)` iterations of
`current += (target - current) * rate`, then the model's world matrix is the base basis with rows 0-2 scaled by
`current` (`FUN_088599f8`, the same row scaler shape as `0x0885ba48`; the translation row is untouched).
Pulse stores `(current, target, rate)` as `(0, 8, 0.1)`, `(0, 3.5, 0.07)`, `(0, 3, 0.07)` (its update reads `+0x6c` as
current and `+0x70` as target), the same rates and the same per-tick integer-count law, so Pure's blast grows
to roughly twice Pulse's target scale (`15/8`, `8/3.5`, `7.8/3`) at the same speed. Confidence **80**
(structural read; the Pulse scale curve's tick-count dependence the Pulse page flags is the `(int)(dt/0.01667)` loop).

## Animation time, life and basis

- **Animation time:** `FUN_08863f40(age * m[i], model)` with `m = 1.0, 1.0, 1.0` (`0x08a8e608`-`0x08a8e610`, base `s6 = 0x0028a608`). Confidence 85.
  The `0.1 / 0.07 / 0.07` are the ease rates above, not animation-time rates: Pulse's update reads its multiplier from
  `DAT_08ab0f60` (also `1.0`; its decompile line `FUN_08912890(age * (&DAT_08ab0f60)[i], ...)`), so
  [Pulse's plasma.md](../psp-pulse-usa/plasma.md) ("the three `rate[i]` values ... `0.1`, `0.07`, `0.07`") attributes the ease rates to the animation time.
  Flagged for the Pulse lane; not edited here.
- **Life:** the object is retired when `age >= 2.0` (`lui a0, 0x4000` at `0x0885b1e0`, `c.lt.s` at `0x0885b1e8`, a flag write `|= 0xa`, `FUN_0008f940` release). **Pulse retires at 1.5 s.** Confidence 88.
- **Placement and basis (confidence 70):** the constructor stores the impact point at `+0x40` and
  `+0x160` (translation of the basis group at `+0x130`), copies the identity into the basis, then asks the track
  (`FUN_088441ac(500.0, ...)`, the `AiTrack_LocatePosition` shape, 500-unit reach). If it finds a surface it rebuilds the basis
  from the surface normal (`FUN_08859a64`) and moves the position by `-3.5 x row1` (`0x08a8e620`), i.e. 3.5 units along the
  surface normal. Models 1 and 2 (the hemispheres) use that world-fixed track frame every tick. Model 0 (the halo)
  uses a basis rebuilt each tick from the camera (`FUN_08a32e08(_DAT_0028a1cc)` returns the camera matrix's `z` column,
  the same call `FxLensflare_UpdateIntensity` makes), negated into row 2, at the raw point `+0x40`. The GE dumps of Pulse's blast
  put the halo and hemispheres 3.49 units apart and show the halo turning with the camera-independent spin, so the halo-basis reading is **55**,
  as in Pulse.
- **Effects at spawn:** `ScreenFlash_Start(_DAT_00288e44, 1, pos)` and `WO_PLASMA_FLASH` (`PLFL`, string `0x08a48794`) at the raw
  position with an identity basis, through `FUN_08871e1c`, as in Pulse.

## Triggers: who spawns a blast

`PlasmaBlast_Spawn` (`0x08854b70`) has three callers:

| Caller | Address | Reading |
| --- | --- | --- |
| `Plasmas_Update` | `0x088552a0` (call at `0x088554c0` to `0x08854e00`; spawn at its second loop) | A bolt whose timer `+0x58 > 10.0` or whose flag `+0x40 & 4` is set is torn down and spawns a blast at `+0xa0` of the bolt. Same `10.0` timeout and the same teardown spawn as Pulse. Confidence 80. |
| per-bolt flight step | `0x08854e00` | Spawns at `0x08855080` after the bolt's sweep, guarded by `FUN_0892410c` on the struck object. A hit spawn. Confidence 65. |
| bolt matcher | `0x088549fc` (called from `0x08855204`) | Finds a bolt by owner and serial (`+0x44`, `+0x48`) in the 16 slots, sets flag `4`, spawns, then runs `FUN_08854d10`/`FUN_08854c94`. Looks like the network or remote path; not named. |

Pulse spawns two blasts on a craft hit (the bolt and the craft, its plasma.md). Pure's 2 callers on the local path (`0x08854e00` and the pool teardown)
suggest the same, but the count per ending was not walked: **open**, confidence 50.

## Ready to wire

Pure's blast can reuse Pulse's wired blast pass with Pure's own data (confidence 80 that the data below is what the original plays):

1. models `plasma_halo`, `plasma_hemisphere_noglow`, `plasma_hemisphere` (index 0, 1, 2);
2. tint keys per the table (time-keyed linear, held at the last value);
3. eased scale targets `15, 8, 7.8` at rates `0.1, 0.07, 0.07` per 60 Hz tick;
4. life `2.0 s` (Pulse `1.5`);
5. hemispheres in the world-fixed track frame at impact + `3.5` along the surface normal; halo at the impact point, camera-derived basis (low confidence).

**Not measured live:** no Pure capture of the blast exists; a PPSSPP probe on `0x0885ae9c` (age at `+0x50`, tint via
`FUN_08863898`'s second argument) would turn the key times into a measured ramp.

## Wired (2026-11-03, lane `pure-weapons-wire`)

Pure's `plasma_blast_pulse` row (`crates/pure/src/lib.rs`) names the three models and carries a `PlasmaBlastLaw`
(`crates/title/src/weapons.rs`): the three `(hold, end)` tint pairs `(0.2, 0.5)`, `(0.3, 1.0)`, `(0, 1.0)`, the scale
targets, rates `0.1, 0.07, 0.07`, life `2.0` s, no halo hide, and the hemispheres dropped `3.5` along the track's up. Pulse's
mechanism (`oag_raceplay::blast_models`) now reads all of it from that law; Pulse's own row carries its constants (tints
`(0.2, 1.0)`, `(0.3, 1.5)`, `(0, 1.0)`, life 1.5), and its tint test is unchanged.

- **Unit.** Our decoder returns the `.vex` extent and the world size is `ease x 0.025 x extent`: the ease targets are stored
  as the read target divided by `0.025` (`15, 8, 7.8` become `600, 320, 312`). Pure's models are `plasma_halo` radius
  5.01 and the two hemispheres radius 4.0 (Pulse's 10.0, 8.65, 7.6), so Pure's blast is larger in its targets (`15/8`)
  but not twice as large on screen: the halo reaches `15 x 5.01 = 75` units against Pulse's `8 x 10 = 80`, the hemispheres
  `32` and `31` against `30` and `23`. Confidence 70 that the Pulse-fitted unit carries over (same loader, same
  decoder; not matched against a Pure frame).
- **Placement.** The `-3.5` drop is applied to Pure's hemispheres, against the frame's up: Pulse's GE dumps settle the
  sign for the shared code path (hemispheres 3.5 below the halo along minus row 1 = minus up, confidence 85;
  [Pulse's plasma.md](../psp-pulse-usa/plasma.md), "the hemisphere drop"). Pulse's row now applies it too. A Pure
  frame has not been captured; the sign is Pulse's, **inherited**.
- **Look.** The headless pair at Pure's default circuit shows a large violet dome around the craft at age 0.05 s and the
  halo's tint gone by 0.5 s; the dark discs and arches seen in this lane's shot are `WO_PLASMA_FLASH`'s particles drawn with the atlas grid
  bug (fixed since; not the shell's texture, which is opaque), see Pulse's page, "the dark discs".
- Open: a Pure capture of the blast (this is read, not matched), the craft-hit count (`plasma_craft_hit_blast_point` spawns
  Pulse's two on Pure too, unmeasured).

## Checked against HD and the other titles

HD and Omega run their own blast (ring, sphere and halo on eased scale ramps, `plasma.md` on those binaries): **checked, differs**
(the same eased `(cur, target, rate)` shape, different models). 2048: **not checkable** here (no PSP code).

## The head's arcs (checked against Pulse, 2026-11-03)

Pulse's charging head draws its root emitter's arcs through the craft's live matrix and at the
instance's live severity ([Pulse's plasma.md](../psp-pulse-usa/plasma.md), "the arcs' size follows the
charge"). **Checked, not wired for Pure.** Pure's `WO_PLASMA_HEAD` file carries the same flag `0x2`, but
`Plasma_Init`'s `Psys_Spawn` call (the parent argument that makes the matrix move) is unread, so the
evidence is below 70 and `Effect::rides_owner_frame` and `System::set_draws_live_severity` stay off for Pure.
Reading Pure's `Plasma_Init` is the step that would turn both on.

**Read 2026-11-11 (lane `pure-disruptor-finish`): Pure's Plasma charges and releases on the Ship Muzzle node, and its head is parented to it.**
`Plasma_Init` (`0x0885df98`, `param_4` the craft) loads `node = *(craft+0xe0)` (the `Ship Muzzle` node
`ShipMuzzle_Importer` builds, [weapon-effects.md](weapon-effects.md), "Disruptor"), copies the node's four matrix rows (`node+0x30`) onto the bolt
at `+0x70..+0xac`, and spawns the head `WO_PLASMA_HEAD` (`FUN_08871e1c(.., 0x45484c50 'PLHE', 0, flag 0x10, parent = *(craft+0xe0))`): the
same flag `0x10` and weapon-node parent Pulse's `Plasma_Init` uses. `Plasma_Launch` (`0x0885e1f4`) re-copies the node's matrix onto the bolt,
clears the charging byte `+0x50` and builds the velocity from the craft. So the Pulse reading (the node `(0, 0, 5.653)` through the craft's rows,
the head's arcs riding the craft) holds on Pure: confidence **85** for the node and the parent (decompile-level ceiling). **What this changes in
ours:** Pure's hulls now resolve the locator (`vex::classes::V4.ship_muzzle = 0x394`), so a Pure Plasma bolt is held and released on the node,
not on the fixed hold point. **Not wired:** `Effect::rides_owner_frame` and `set_draws_live_severity` for Pure's `WO_PLASMA_HEAD`; the evidence
the earlier paragraph asked for is now in, so they are ready to turn on (and to check against a Pure frame).
