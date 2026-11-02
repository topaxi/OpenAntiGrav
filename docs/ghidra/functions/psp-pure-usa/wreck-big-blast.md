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
