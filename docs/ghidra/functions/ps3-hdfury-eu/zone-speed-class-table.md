# HD/Fury's Zone speed-class ladder, and the writer of the stage index nothing had found

`EBOOT.elf` (Wipeout HD/Fury, PS3, `BCES-00664`), segment 0 maps file offset
`0` to vaddr `0x00010000`. Static reading throughout, so every score here is
capped at 84 per the
[confidence rubric](../../../reverse-engineering/confidence-rubric.md) - except
where a maintainer's play or a frame of the running original corroborates it,
which is stated per claim.

**The question this page answers**, and it is two questions that turn out to be
one. The HUD's Zone ladder draws the current speed class beside the current zone
(`docs/formats/hd-hud.md`), and needs to know which class a zone runs at.
Separately, [zone-effectsettings-loader.md](zone-effectsettings-loader.md) had
`Environment_UpdateStageBlend` reading the Zone colour stage from
`craftArray[n] + 0x640` with **no found writer** - the one thing between this
port and a Zone race that escalates. Both are answered by the same fourteen-record
table and the same function, which is the shape
[vita-2048-eu-v104](../vita-2048-eu-v104/zone-environment-fallback.md)'s
`Hud_UpdateZoneSpeedClassWidget` already has: **the HUD widget is what drives
the grade.**

## The table: `g_ZoneSpeedClassTable` at `0x00860d44`

Fourteen 8-byte records, `{ u32 zoneThreshold, u32 stringIdPointer }`, thresholds
strictly descending to zero:

| # | Threshold | String id | Reads |
| ---: | ---: | --- | --- |
| 0 | 75 | `IG_HUD_SUPSON` | `SUPERSONIC` |
| 1 | 60 | `IG_HUD_MACH1` | `MACH 1` |
| 2 | 50 | `IG_HUD_SUBSON` | `SUBSONIC` |
| 3 | 42 | `IG_HUD_SUPZEN` | `SUPER ZEN` |
| 4 | 35 | `MSC_ZEN` | `ZEN` |
| 5 | 27 | `MSC_SPPHANTOM` | `SUPER-PHANTOM` |
| 6 | 20 | `Phantom` | `PHANTOM` |
| 7 | 16 | `MSC_SPHANTOM` | `SUB-PHANTOM` |
| 8 | 12 | `Rapier` | `RAPIER` |
| 9 | 7 | `MSC_SRAPIER` | `SUB-RAPIER` |
| 10 | 5 | `Flash` | `FLASH` |
| 11 | 3 | `MSC_SFLASH` | `SUB-FLASH` |
| 12 | 2 | `Venom` | `VENOM` |
| 13 | 0 | `MSC_SVENOM` | `SUB-VENOM` |

The strings are a contiguous blob at `0x0077fdb0`-`0x0077fe62`, in the same
descending order, sitting between `Lockon Time = %0.0f` and the thirteen
`*Icon` weapon-widget names - the HUD code's own literals. The table's fourteen
pointers are the **only** references to any of them in the image, and the
address `0x00860d44` itself is referenced exactly once, from the TOC slot
`0x008a75cc`. Reproduce:

```sh
python3 - <<'PY'
import struct
b = open('data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.elf','rb').read()
s = lambda va: b[va-0x10000:b.index(b'\0', va-0x10000)].decode()
for i in range(14):
    thr, ptr = struct.unpack_from('>II', b, 0x860d44-0x10000 + i*8)
    print(f'{thr:>3}  {s(ptr)}')
PY
```

Confidence **86** on the table's layout and contents: fourteen records, no
misparse possible (the run terminates cleanly at `0x00860db4`, where the next
datum is a pointer to `"FEConst->"`), and the three accessors below read exactly
those two fields at exactly that stride.

## Three accessors, and one of them hands over the record count

| Address | Name | Body |
| --- | --- | --- |
| `0x00083490` | `ZoneSpeedClassTable_Count` | `li r3, 14; blr` |
| `0x00083498` | `ZoneSpeedClassTable_ThresholdAt` | `slwi r4,r4,3; lwz r9, toc(table); lwzx r3, r9, r4` |
| `0x000834b0` | `ZoneSpeedClassTable_NameAt` | `slwi r4,r4,3; lwz r0, toc(table); add r4,r4,r0; lwz r3, 4(r4)` |

Each takes `r3` = a table object it never reads and `r4` = the index, so the
table is static behind what the source spelled as a member function. **The count
is a literal `14` in the instruction stream**, which is what makes the record
count a read rather than a guess about where the run ends. Confidence 88.

## The walker: `Hud_UpdateZoneSpeedClass` at `0x00049718`

The only caller of all three, and itself called from exactly one site
(`0x0004d6ac`). Its loop:

```text
00049c8c  li    r28, 0                 ; i = 0
00049c90  b     0x49cd0                ; check the count first
00049c98  lwz   r0, 0x154(r30)         ; the craft index
00049c9c  slwi  r0, r0, 2
00049ca4  lwzx  r3, r31, r0            ; craftArray[index]
00049ca8  bl    0x83498                ; ZoneSpeedClassTable_ThresholdAt(i)
00049cbc  fcfid f13, f13               ; the threshold, as a float
00049cc0  frsp  f0, f13
00049cc4  fcmpu cr7, f0, f29           ; threshold vs the zone counter
00049cc8  bf    29, 0x49fbc            ; not greater -> this record is the one
00049ccc  addi  r28, r28, 1            ; i++
00049ce4  bl    0x83490                ; ZoneSpeedClassTable_Count
00049cf0  cmpw  cr7, r28, r3
00049cf4  bt    28, 0x49c98            ; while i < 14
```

So it walks **down** from the highest threshold and stops at the first record
the counter has reached - `first record with threshold <= zone`, which with a
descending table is the ordinary "which band is this in" search. Then:

```text
00049fd4  bl    0x834b0                ; ZoneSpeedClassTable_NameAt(i)
00049fdc  li    r5, 0
00049fe4  li    r6, 1
00049fec  bl    0x202f58               ; set the widget's text from the string id
00049ff4  lwz   r0, 0x154(r30)
0004a000  lwzx  r29, r31, r0           ; craftArray[index] again
0004a008  bl    0x83490                ; the count, 14
0004a010  sub   r3, r3, r28            ; 14 - i
0004a014  stw   r3, 0x640(r29)         ; <- the stage index
```

**`stw r3, 0x640(r29)` is the writer `zone-effectsettings-loader.md` records as
unfound.** That page traced `Environment_UpdateStageBlend` (`0x003da540`)
falling through to `craftArray[n]->+0x640` for every mode but Detonator and two
split-screen ones, and said "whose writer was not found". It is this
instruction, and the value is `14 - i`: the record's index counted from the
bottom of the table, which is the same `0x11 - i` arithmetic
`Hud_UpdateZoneSpeedClassWidget` uses on 2048. Confidence **85** - the store's
target, base register and value are all read directly; what is inferred is that
`r31` is the craft array, which follows from the other page's independent
reading of the same expression on the consuming side.

### `14 - i` lands on the `.effectSettings` rungs exactly

`/data/environments/zonemode.effectsettings` keys its palettes `0 Start`,
`1 Sub Venom`, `2 Venom` .. `14 Supersonic` - fifteen rungs. `14 - i` maps
record 13 (`MSC_SVENOM`) to `1`, record 12 (`Venom`) to `2`, and record 0
(`IG_HUD_SUPSON`) to `14`. Every one of the fourteen lands on the palette whose
key carries the same class name, and rung `0 Start` is the one the loader leaves
behind and no zone reaches. That is fourteen independent agreements, and it is
what raises the "these are zone numbers" reading above the level a static trace
alone could reach.

## The units are corroborated twice from the running game

The compared value is a float, so nothing in the arithmetic says what it counts.
Two observations of the original pin it, and they pin different rows:

- **A Zone frame supplied by the maintainer** (not kept in the repository)
  reads `SUB-VENOM` beside zone `1`. Record 13's band is `[0, 2)`. ✓
- **The maintainer's own play**: "not every single zone is a speedclass bump",
  and then "zone 2 should already be venom though (kindof the exception)".
  Record 12's threshold is `2` and its band is exactly the single zone `2`. ✓
- **A second frame, at zone 8** (not kept in the repository),
  reads `8  SUB-RAPIER` on the current row and `RAPIER` on row `12`. Record 9's
  band is `[7, 12)` and record 8's threshold is `12`. ✓

The second and third are the sharp ones. A one-zone band in the middle of a
table whose others run 2 to 15 wide is not something a wrong reading lands on,
and the third confirms a **five-zone** band and its exact upper boundary from a
frame the table was already written before anyone had. Confidence **90** that
the compared value is the zone counter.

## Bands

| Zones | Class | Rung |
| --- | --- | ---: |
| 0-1 | Sub Venom | 1 |
| 2 | Venom | 2 |
| 3-4 | Sub Flash | 3 |
| 5-6 | Flash | 4 |
| 7-11 | Sub Rapier | 5 |
| 12-15 | Rapier | 6 |
| 16-19 | Sub Phantom | 7 |
| 20-26 | Phantom | 8 |
| 27-34 | Super Phantom | 9 |
| 35-41 | Zen | 10 |
| 42-49 | Super Zen | 11 |
| 50-59 | Subsonic | 12 |
| 60-74 | Mach 1 | 13 |
| 75+ | Supersonic | 14 |

Two one-zone bands at the bottom and then a widening run, which is the same
character as 2048's `0`-`1`, `2`-`8`, `9`-`16` - and **not** the same numbers, so
neither table is a copy of the other.

## What is not established

- **What sets the float.** `Hud_UpdateZoneSpeedClass` takes it as `f1` from its
  single caller (`0x0004d6ac`), which forwards its own argument. Nothing here
  traced it back to the zone counter's own storage; the units rest on the two
  observations above rather than on a dataflow trace.
- **Whether the widget can be off while the grade still moves.** The store to
  `+0x640` is inside the function that updates the *HUD text*, so on this
  reading a Zone race with its HUD hidden would freeze the colour grade. That is
  what the code says and it has not been checked against the running game.
- **`0x00202f58`**, the text setter, is named by its argument shape (`node`,
  `stringId`, `0`, `1`) and is not disassembled here.

## See also

- [zone-effectsettings-loader](zone-effectsettings-loader.md) - the consumer of
  `+0x640`, and the page whose open question this closes
- [race-hud](race-hud.md) - `Hud_LoadDefinition`
- [hd-hud](../../../formats/hd-hud.md) - the Zone ladder widget this drives
- [zone-environment-fallback](../vita-2048-eu-v104/zone-environment-fallback.md) -
  2048's equivalent, found first and the reason this one was looked for
