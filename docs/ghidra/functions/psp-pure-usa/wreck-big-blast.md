# Pure's wreck: the node blast and the big blast

Pure has no `WO_SHIP_EXPLOSION`. Its executable names `WO_SHIP_FXNODE_BIGEXPLO`,
`WO_SHIP_EXPLO_SMOKE` and `WO_SHIP_EXPLOSION_DEBRIS` instead, and this page reads what throws them.

| | |
| --- | --- |
| **Binary** | `PSP_GAME/SYSDIR/BOOT.BIN` (Pure, PSP USA), image base `0x08804000` |
| **Related** | [`psp-pulse-usa/screen-flash-callers.md`](../psp-pulse-usa/screen-flash-callers.md) (Pulse's `Ship_SpawnExplosionSmall` and `Ship_SpawnExplosionBig`, the counterparts), [`rocket-and-collision-fx.md`](rocket-and-collision-fx.md) |

| Address | Name | Confidence |
| --- | --- | --- |
| `0x08923db4` | `Ship_SpawnExplosionSmall` | 78 |
| `0x089258b8` | `Ship_UpdateDestroyed` | 76 |

## Evidence

- Strings (`search_strings` over `/pure/BOOT-psp-pure-usa.BIN`): `WO_SHIP_FXNODE_EXPLO` `0x08a7a3e8`,
  `WO_SHIP_DEATH_SPARKS` `0x08a7a400`, `WO_SHIP_FXNODE_BIGEXPLO` `0x08a7a468`,
  `WO_SHIP_EXPLOSION_DEBRIS` `0x08a7a480`, `WO_SHIP_EXPLO_SMOKE` `0x08a7a49c`. `get_xrefs_to` resolves
  them (the first two from `0x08923ed4`/`0x08923ee0`, in `0x08923db4`; the other three from
  `0x08925a88`, `0x08925c70`, `0x08925d10`, in `0x089258b8`).
- `Ship_SetState` (`0x08928824`) dispatches through a table at `0x08a7a6a0`; case 5 (`0x08928cf0`) writes
  `craft+0x9a8 = 1.5` (`0x3fc00000`) and ends by calling `0x08923db4`: the same shape as Pulse's
  state 5 calling `Ship_SpawnExplosionSmall` and arming the 1.5 s timer.
- `0x08923db4` walks up to eight `craft+0xbf0` nodes and spawns `WO_SHIP_FXNODE_EXPLO` (tag `FXNE`) then
  `WO_SHIP_DEATH_SPARKS` (tag `DESP`) on each, as Pulse's loop does (eight nodes, Pulse's ten).
- `0x089258b8`, called from the per-state update (`0x08926ae4`, the call at `0x08926f28`), subtracts `dt`
  from `craft+0x9a8`; at zero or below it:
  1. plays a sound and the screen flash (the player's own shake and flash, which this port already
     throws for the `Destroyed` to `Eliminated` timeout);
  2. for each of up to **eight** nodes spawns `WO_SHIP_FXNODE_BIGEXPLO` (tag `FXBI`) parented to the
     node (`Psys_Spawn_q`-shaped call `0x08871e1c`, parent `*(node+0x30)`);
  3. builds a 0xa0-byte object from the craft's matrix row (`0x0885a7d8`) - not read, left unwired;
  4. scales the live model's matrix by `1e-4` (`0x38d1b717`), hiding the hull, and calls
     `Ship_SetState(craft, 6)`;
  5. spawns `WO_SHIP_EXPLOSION_DEBRIS` (tag `EXDE`) and `WO_SHIP_EXPLO_SMOKE` (tag `EXSM`) at the model
     node's matrix (`model+0x80`, 16 words), **with no drop along `-up`** (Pulse moves its blast 4.0 rows
     down).
- Confidence: the callers, timer and tags are read off the decompile and disassembly (76-78). Not read
  live on PPSSPP.

## What is wired

`oag_title::Trigger::WreckBigNode`, `WreckDebris`, `WreckSmoke` in `oag_pure::EFFECTS`, thrown by
`Race::throw_wreck_explosion` 1.5 s after the state 5 edge (`docs/rendering/hull-sparks.md`, "Other
titles"). Unread: the 0xa0 object at step 3 (Pulse's equivalent is the ship shockwave; Pure throws none),
the per-title sound, the effects' scale (the craft row scale `0.75` is carried from Pulse), and the
particle parent of step 5 (placed in the world, like Pulse's).

## Read live on PPSSPP (2026-11-03, `pure-wreck-blast-look`)

Method: Pure USA, PPSSPP 1.20.4 software renderer, a Vineta K Time Trial state, the player's entity wrecked with
`Ship_SetState(entity, 5)` from a stop at `0x08926ae4` (`scripts/psp-pure-wreck-blast.py`; `a0` of
`Ship_UpdateCraft` is the craft, not the entity, and injecting there crashed the emulator). Two wrecks, two emulator
boots, a GE dump (`gpu.record.dump`) one stop before the blast and 3 and 6 stops after it, read with
`psp-ge-dump.py census` and replayed edited with `psp-ge-replay.py`.

**Confirmed (confidence 75, two wrecks):**

- Pure's per-state update runs at **30 Hz**: the timer at `entity+0x9a8` is armed to `1.5` by state 5 and falls `0.0333` per
  stop, so the blast fires 45 stops after the wreck state and the entity reads state 6 with its timer at `2.0` at that stop
  (the state-6 timer is then `2.0`, falling the same way). Ours counts 90 ticks at 60 Hz: the same 1.5 s.
- The blast is a burst of new draws, none present one stop before it.

**The blast's draws (confidence 60: two wrecks, both ran the same state file, census identical in texture and count).** New
non-alpha-tested, depth-tested, depth-write-off draws, by the GE's own state:

| Draws | Blend (`BLENDMODE`) | Vertex type | Texture | World extent | Reads as |
| --- | --- | --- | --- | --- | --- |
| 6 (two wrecks: 6) | `0x32` source alpha, one minus source alpha (alpha-over) | `0x1fe` (xyz, normal, uv) 78 vertices | CLUT4, palette ramps orange to yellow with alpha | 6 to 9 units | flame blobs |
| 1, 1, 1 | `0x32` | `0x1fe`, 36 / 96 / 96 vertices | CLUT8 white with a 0 to 255 alpha ramp; CLUT4 grey smoke palettes (194 and 182 of 256 entries alpha 0) | 24, 25-28 units | smoke |
| 6 | `0xa2` source alpha, fixed (additive) | `0x19f` quad, 4 vertices | CLUT8, 256 opaque red-to-white entries | 20 to 24 units | the fireball sprites |
| 4 to 6 | `0xa2` | `0x1fe`, 24 vertices | CLUT8 orange ramp | 9 to 15 units | additive debris meshes |

Also new at the blast: one 103-vertex alpha-tested additive mesh of 21 units on a VRAM texture (`0x1c5480`), unidentified
(a candidate for the 0xa0 object at step 3, which this page left unread). The `.pob` blend classes agree: `WO_SHIP_EXPLOSION_DEBRIS`'
root and `Debris2` emitters and `WO_SHIP_EXPLO_SMOKE` are class 3 (alpha-over), `trails` and the fireball are class 2 (additive).

**Verdict per element against ours (`--force-wreck 40:7`, ticks 160 / 166 / 172, each effect alone):**

| Element | Verdict |
| --- | --- |
| Fireball (`WO_SHIP_FXNODE_BIGEXPLO`) | matches in kind: ours alone gives an orange fireball ring with no blocks; the original's fireball is additive sprites. Scale not compared (see below) |
| Smoke (`WO_SHIP_EXPLO_SMOKE`) | **matches** (see "The debris blocks"): not a bug, faint for its first 8 ticks |
| Debris (`WO_SHIP_EXPLOSION_DEBRIS`) | **fixed** (`pure-debris-blocks`): the blocks were ours' atlas read the wrong way round (see "The debris blocks") |

So the dark blocks are **ours alone** (confidence 65: one view of the original, from the destroy camera inside the blast, and one
of ours from a rival's wreck; the two are not the same camera). Not shown to be a blend bug: the original's class 3 blend is
the ordinary source alpha, one minus source alpha, and its palettes carry graded alpha. The cause is in how ours draws the debris
emitters' texture or mesh, not in this page's reading; it is open as a ready-to-wire item: bisect ours' debris emitters
(root, `Debris2`, `trails`) for the one that draws the blocks and compare its texture decode and shape with the census above.

**Not got:** the live scale at the spawn call (the breakpoint on `0x08871e1c` was never hit in the 60 s wait; the matrix is
read from the draws instead: sprite diagonals 20 to 24 world units, meshes 6 to 28), a clean original frame beside a
rival's wreck (a Time Trial has no rivals; the player's wreck brings the destroy camera, the end-of-race menu and a yellow
screen flash over the blast, so the replayed pictures are blast-through-flash, not a match for ours' framing), and the sound.

## The debris blocks (2026-11-03, `pure-debris-blocks`)

Cause found (confidence 90): ours read the emitter's atlas grid word with the columns in the wrong half.
`WO_SHIP_EXPLOSION_DEBRIS`'s root and `Debris2` author `4, 2` on a 64x128 sprite whose picture is two columns
of four 32x32 shards; ours cut 16x64 slivers (4 columns, 2 rows) and stretched each over a square quad, and
the colour table starts black at alpha 255, so a sliver drew as a hard dark rectangle. `trails` (additive, 32x32)
was never involved. The fix and its evidence are `docs/formats/pob.md`, "The atlas grid word".

Re-read of the original's dump (`run2`/`run3`, `dump+3`), which also corrects the table above:

| Original draw (texture dims, format, blend) | Emitter |
| --- | --- |
| 2 x 64x128 CLUT4 alpha-over, 96 vertices, UV cells 0.5 x 0.25, grey vertex colour 45-53, alpha 255 | the debris root and `Debris2` (the table above counted these two as smoke) |
| 1 x 64x64 CLUT8 alpha-over, 36 vertices, black vertex colour, alpha 31-88 at +3 stops, 119-174 at +6 | `WO_SHIP_EXPLO_SMOKE` |
| 4 x 32x32 CLUT8 additive, 12 vertices | `trails` |
| 6 x 128x64 CLUT4 alpha-over, UV cells 0.25 x 0.25 | the node effects' fire atlas (4x4) |

The debris is grey and dark in the original too (vertex colour 45-53), so "soft orange cloud shapes" in the
replay reading was the neighbouring fire draws, not the debris.

**Smoke: not a bug.** Ours' alpha channel is 1.7 at age 0, 90 at +6 ticks and 178 at +12 (the original's
vertex alphas 31-88 at +3 stops = 6 ticks and 119-174 at +6 stops = 12 ticks, spawn ages being jittered over the
8 emission ticks). Drawn alone, ours' smoke is faint from the blast tick, clearly visible by +8 ticks and a dense
black cloud by +25. The earlier "draws nothing at ticks 160-172" was the first 8 ticks of that ramp seen under
the fireball and debris.

After the fix (`--race --opponents --force-wreck 40:7`, ticks 160 / 166 / 172, Pure USA): the debris is a soft
dark shard cloud above the fireball, no rectangles; the smoke builds behind it. Not compared: scale at the spawn call.

Cross-title: 2048, Vita sprites: **ported** in the reader (24 non-square emitters checked, 22 square); Omega:
**checked, applies** (same reader and word; its `.gnf` sprites are not read in this census, so its per-emitter cells are unverified); HD: **checked, unchanged**.
