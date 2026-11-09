# Trail hits on 2048: the burst is spawned by nothing

2026-11-09, `hd-trail-hit-draw`. Static read of `eboot.elf` (patch v1.04),
made as the cross-title check of Omega's finding
([ps4-omega-eu/trail-hits.md](../ps4-omega-eu/trail-hits.md)).

## The spawn chain is HD's (confidence 85)

- **`TrailHit_Spawn` (`0x81299798`, 85).** It names `WO_TRAIL_HITSHIP`
  (`0x814d5f20`, its only reference) or `WO_TRAIL_HITSHIP_RED` by its third
  argument, with tags `0x494c5254`/`0x524c5254`. It scales the craft's
  velocity by `0x3c888889` (1/60), and stores `1 - f` into the instance. This
  is the same body as HD's spawn and Omega's `0x01724f50`.
- **`Trail_SpawnHitBursts` (`0x812a100e`, 80)**, its one caller, loops 16 trail
  slots (stride `0x720`). For a slot with **`+0x6d0 & 4`** it finds the nearest
  of up to 10 hull anchors (craft `+0x6fd8`), checks two craft-state bytes,
  spawns, and rewrites bit `0x4`.

## Nothing sets bit `0x4` (confidence 65)

Every instruction with operand `#0x6d0]` was swept (26 matches). The trail-slot
writers are:

| writer | bit `0x4` |
| --- | --- |
| `FUN_812a025c`, `FUN_812a0292`, `FUN_812a032a`, `FUN_812a0390` (resets) | `bfi ..., #2, #1` from a register the reset zeroes (`movs r1, #0` at `0x812a0292`) |
| `FUN_812a0aba` (`0x812a0cc6`) | writes bits 5-8 only |
| `FUN_812a1d76` (`0x812a1ef4`, `0x812a1f34`) | writes bits `0x8` and `0x10` only |
| `Trail_UpdateRing` (`0x812a119c`, 60), `0x812a13a2` | `bfi r5, r1, #2, #1` with `r1` derived from `r4`, which `movs r4, #0` at `0x812a1396` zeroes on both paths into it (the fall-through and the `bne` at `0x812a1290`; nothing else branches to `0x812a1398`/`0x812a139c`) |
| `Trail_SpawnHitBursts` (`0x812a109a`) | rewrites it after the spawn check |

So 2048's CPU port of HD's `Trails` SPU job writes the hit bit as 0 every frame,
and the burst never spawns. Why 65: the reset functions' zero source is read at
one of the four, and nothing was run on Vita3K.

**This project's wiring.** Like Omega, 2048 plays the burst only with
`graphics.trail_hits` on, from its own `WO_TRAIL_HITSHIP`
(`oag_title::Effects::trail_hits_native`).

## Names applied

| Address | Name | Confidence |
| --- | --- | --- |
| `0x81299798` | `TrailHit_Spawn` | 85 |
| `0x812a100e` | `Trail_SpawnHitBursts` | 80 |
| `0x812a119c` | `Trail_UpdateRing` | 60 |
