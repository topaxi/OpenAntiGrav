# Pure draws the weapon-absorb hull overlay with Pulse's law

**Binary:** `pure-psp` `BOOT.BIN` (USA, UCUS-98612), image base `0x08804000`.
**Status:** read off a running image's relocated code (a memory dump of the loaded game, `capstone` MIPS with the VFPU words left as
words) and the fade read live on PPSSPP v1.20.4. Nothing is renamed: the two functions stay `FUN_` addresses here, below the
rename bar for lack of a decompile of the matrix half. No `names.tsv` row.

Pulse's pair is [`HullOverlay_Submit`](../psp-pulse-usa/cannon-quake-leachbeam.md) (`0x0890e304`) and `HullOverlay_AbsorbFade`
(`0x0883e950`). Pure has the same pair with one texture instead of two (no LeachBeam), so the texture is read from a global inside the
submit function instead of being passed in.

## The texture

`Data\Tex\Weapons\absorb_surface.mip` (string `0x08a495f0`, a second copy in a name table at `0x08a50328` -> `0x08aac6cc`) is loaded at
`0x0886d618..0x0886d664`: `lui a1, 0x8a5; addiu a1, a1, -0x6a10` is the path, the call is `0x0887e108`, and the result is stored into
**`0x08ac9a00`** (`sw s2, -0x6600(a0)` at `0x0886d664`). That global has exactly one more reader: the submit function (below).
Confidence 90.

## The submit function, `FUN_0886e954` (`pulse` in `$f12`, mesh in `a0`, batch in `a1`, entry in `a2`)

Read at instruction level (`0x0886e954..0x0886ed9c`), constant for constant against Pulse's:

| Step | Pure | Pulse's twin |
| --- | --- | --- |
| alpha | `if pulse < 1.0 { pulse } else { 2.0 - pulse }` into all four channels (`0x0886e984..0x0886e9b8`, scaled by `255.0` = `0x437f0000` into a packed colour at `0x0886e9d4..0x0886eaf4`) | the same fold, `oag_fx::hull_overlay::alpha` |
| batch state | `jal 0x8866ecc` with `a1 = 0x282` (`0x0886eb08`) | `Gfx_BuildBatchStateList(0x282)` |
| texture bind | `lw s3, -0x6600(s3)` (the global above) then `jal 0x887e900` (`0x0886eb6c..0x0886eb88`) | the `a3` argument |
| repeat | the word at `0x08a8f510`, read live: **10.0** (`lui a0, 0x8a9; lw a0, -0xaf0(a0)` at `0x0886ec58`) | `DAT_08abf4a4` = 10.0 |
| scroll | `lui a0, 0xbfc0` (= **-1.5**) times `$f20` (the pulse) at `0x0886ec1c..0x0886ec24`, then the VFPU matrix (`0x48e4....`/`0xd000....` words) | `-1.5 * pulse` |
| quarter turn | `lui a1, 0x3fc9; ori a1, a1, 0xfdb` = `pi/2` into `0x08a2e7b0` (`0x0886eba..0x0886ebb4`) | the same constant |
| projection | texture-matrix upload `jal 0x88ee1a4` with `a0 = 3` (`0x0886ed54`), as Pulse's `Gu_SetMatrix(3)` | `Gu_SetMatrix(3)` |

So `u = 10 x`, `v = -10 z - 1.5 p` as on Pulse, confidence 85: the matrix itself is built in VFPU words capstone does not decode, so its
element order is read from the constants and the structure, not the instructions. The picture agrees (below).

Two callers: `0x0886e0f4` (a per-entity walk, one call per sub-mesh while `lhu (entry) & mask`) and `0x088832ec` (a per-batch walk), the
twins of Pulse's `HullOverlay_DrawAbsorb` and `HullOverlay_SubmitAbsorbBatched`. Both gate on the same fade.

## The fade, `FUN_08924998` (`craft` in `a0`)

```
e = *(craft + 0x960) - *(craft + 0x9ac)
return (0 <= e <= 1.0) ? e * 2.0 : 0.0
```

`+0x960` is the craft's clock, `+0x9ac` the absorb stamp. Both callers skip the draw at `fade <= 0.0`
(`c.le.s $f20, 0.0` at `0x0886e0c8`). It is Pulse's `HullOverlay_AbsorbFade` instruction for instruction.
**Read live** (PPSSPP, Vineta K, Time Trial, Vector, Feisar; the held id 1 written at weapon record `+0x1d8`, circle held, 2026-11-09):
the stamp reads `-10.0` until the update after the held id drops to 0, then the craft's own clock (`28.594`), and
`clock - stamp` reads `0.033, 0.067, 0.100, 0.134, 0.167` on the next five updates (a 30 Hz game: 0.0334 s per update). Confidence 90.

## In the port

`oag_pure::effects::LOOKS.hull_overlay` is on (`Platforms::Any`, `Origin::Measured`): the roster loader builds the overlay over Pure's
hulls through the shared `oag_fx::hull_overlay` and `Race::play_absorb_feedback(.., pickup)` stamps it, as for Pulse. Pure's
`absorb_surface.mip` is the same 2,064-byte 32x32 8-bit paletted file as Pulse's (`weapons.md`).
Ground truth: `crates/raceplay/tests/absorb_ground_truth.rs`, `pure_lights_the_hull_for_one_second_after_a_pickup_absorb`.
Pairs and scores: [`weapon-absorb-pairs.md`](../../../rendering/weapon-absorb-pairs.md), "Pure PSP".

## Pure's POB

`Data\Psys\WO_WEAPON_ABSORB.POB` on Pure (27,872 bytes) is not Pulse's (24,688). Its streak emitter is `shape 4`, `20` ticks, `3`
per tick, `-0.4998` speed, life `7`, class 6, colour mode **0** walking a purple `(109, 22, 167)` to magenta `(181, 14, 147)` table. It
also authors two template billboards, `ring1` and `glow`, which ours does not draw (the template draw is unread on Pure): see
[`particle-bars.md`](particle-bars.md). Pulse's file has the `rings` emitter (class 3, zero speed) and a cyan table.
