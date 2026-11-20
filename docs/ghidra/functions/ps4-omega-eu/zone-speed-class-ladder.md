# Omega's Zone speed-class ladder: HD's table, HD's `14 - i`, and a second 2048 ladder beside it

`eboot.bin` (WipEout: Omega Collection, PS4, EU). Static reading in the **1.07 patch** program
(`/omega/eboot-ps4-omega-eu-v107.bin`, image base `0x01000000`); the base package's file carries the
same table, located by byte pattern below. No names are applied to the patch program and the base
program was not open, so this page adds **no `names.tsv` row** (that file takes base addresses only,
[patch-v107-addresses.md](patch-v107-addresses.md)); the names here are proposals. Static reading caps
every score at 84 ([rubric](../../../reverse-engineering/confidence-rubric.md)).

**The question** (lane `omega-zone-ladder`): does Omega pick its Zone colour stage by a ladder of its
own, so that `omega/zone-vineta-k-z5`'s olive scenery (ours draws HD's Flash purple) is a ladder gap?
**No.** Omega's ladder is HD's, byte for byte, and zone 5 lands on the same stage (4, `Flash`) in both.
The hue gap is not the ladder; the lead it leaves is at the end.

## The table: 14 records, `0x0197b890` (patch), file offset `0x915080` (base)

Fourteen 16-byte records `{ u64 zoneThreshold, u64 stringIdPointer }` (HD's are 8 bytes: 32-bit
pointers), thresholds `75, 60, 50, 42, 35, 27, 20, 16, 12, 7, 5, 3, 2, 0` and ids
`IG_HUD_SUPSON, IG_HUD_MACH1, IG_HUD_SUBSON, IG_HUD_SUPZEN, MSC_ZEN, MSC_SPPHANTOM, Phantom,
MSC_SPHANTOM, Rapier, MSC_SRAPIER, Flash, MSC_SFLASH, Venom, MSC_SVENOM`: identical to
[HD's `g_ZoneSpeedClassTable`](../ps3-hdfury-eu/zone-speed-class-table.md) and
[2048's text table](../vita-2048-eu-v104/zone-speed-class-text.md). The strings are the contiguous
blob at `0x0187317d` (patch). Confidence **90**: no misparse is possible on 14 matching thresholds
and 14 resolving ids, in both packages' files. Reproduce (either `eboot.bin`; the pointers are file
vaddrs, so the ids resolve through the one string's file offset):

```sh
python3 -I - <<'PY'
import struct
thr = [75, 60, 50, 42, 35, 27, 20, 16, 12, 7, 5, 3, 2, 0]
for p in ['data/extracted/ps4/omega-eu/uroot/eboot.bin',
          'data/extracted/ps4/omega-eu-patch/uroot/eboot.bin']:
    b = open(p, 'rb').read()
    o = b.find(struct.pack('<Q', 75))
    while not all(struct.unpack_from('<Q', b, o + 16 * k)[0] == t for k, t in enumerate(thr)):
        o = b.find(struct.pack('<Q', 75), o + 1)
    first = struct.unpack_from('<Q', b, o + 8)[0]
    delta = b.find(b'IG_HUD_SUPSON\0') - first
    print(p, hex(o))
    for k in range(14):
        t, ptr = struct.unpack_from('<QQ', b, o + 16 * k)
        s = ptr + delta
        print(' ', t, b[s:b.index(b'\0', s)].decode())
PY
```

Base: table at file offset `0x915080`, first id pointer `0x8117f6`. Patch: `0x97f380`, `0x87317d`
(Ghidra `0x0197b890`).

## Three walkers, and one of them is the 2048 ladder

The table's first and last strings have the xrefs below; each walker sits beside a descriptor in the
same data region, and none has a direct caller (vtable data only, as `race-hud-selection.md` records
for the 2048-lineage managers).

| Patch address | Proposed name | Reads | Stores to `+0xcf8` | Confidence |
| --- | --- | --- | ---: | ---: |
| `0x012992b0` | `ZoneHud_UpdateSpeedClass` | the 14 thresholds, **unrolled as float compares** (`< 75.0`, `< 60.0`, ... `< 0.0`) over a float zone counter read from a per-slot array (`DAT_01a78e98 + 0x30b70 + slot * 0xc`); strings from the table at `0x0197b898` | `0xe - i`, on `*(*(hud + 0x78) + 0xcf8)` | 80 |
| `0x0126b030` | `ZoneHud_Update` (the HUD, also the `Message` cue site) | the same table (`0x01273aac`/`0x01273ab8`) | one `mov [r13 + 0xcf8], eax` at `0x01273bf7` | 65 (the stored value was not read) |
| `0x012d8440` | `ZoneHud2048_UpdateSpeedClass` | **17** records, `9999, 90, 85 .. 40, 33, 17, 9, 2, 0` at `0x0197d360`, stride `0x18` (2048's `*_CLASS` ladder, `docs/ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md`) | `0x11 - i`, on `*(hud + 0x10) + 0xcf8`; also `+0xcfc` (the zone as an int), `+0xd00`, `+0xd04`, `+0xd08` | 80 |

So Omega carries **both** ladders, as 2048 does, and the
split by layout is the one the HUD roots already draw: the HD-lineage `Zone_HUD.xml` (HD's arc, the
original frame) goes with the 14-record walkers, the `2048_hud\Zone_HUD*.xml` managers with the 17.
Which manager each walker belongs to is **not tied** (vtable data only); the pairing is by table and
layout, confidence 60.

## Where the stage goes: one field, two readers

`+0xcf8` of the object at `DAT_01a735b0` (the player's per-craft record; that this is the object the
HUD walkers write is inferred from the matching offset, **not read**, confidence 65) is read by:

- `FUN_016787b0` (`0x016787b0`, proposed `Zone_UpdateStage`), the 2048-shaped stage blend: clamps the
  stage to `0 ..= 12` (`< 0xd`), cross-fades row `cur` into row `cur + 1` over the `Zone %s.*` table
  (`DAT_020e5440`/`DAT_020e5580`, rows of `0x340` bytes), fires `ZONE_Wave` on a stage change and
  `ZONE_Pulse` when `+0xcfc / 5` changes. Confidence 75.
- `FUN_015f2d30` (`0x015f2d30`; its decompile timed out twice, read from the disassembly at
  `0x015f4cde`): `mov rax, [0x01a735b0]; mov eax, [rax + 0xcf8]; mov [0x019a80c4], eax`, compared with
  the previous value at `0x019a80c0`: HD's `Environment_UpdateStageBlend` equivalent, **no clamp to 12**.
  It binds the HD key set (`%s.Scene.Base Colour` ... at `0x0188e674`, binder `FUN_0161fdb0`). Confidence 60.

Other `+0xcf8` writers: `FUN_01284a40` stores `1`; `FUN_01256370` reads a quadword at `+0xcf8` of another
struct; `FUN_01797b80` uses `+0xcf8` as a float of its own. Not followed.

## Which table a race loads: `FUN_0162a390`, the same fork as 2048's `Environment_Load`

Omega's environment loader (`0x0162a390`, proposed `Environment_Load`) is 2048's
(`zone-environment-fallback.md`, "`Environment_Load`") with the same fork. A circuit path containing
`source\environments\`, `published\environments\` or `environments2048` takes the **primary** branch
(`<circuit>\ZoneMode2048.effectSettings`, then `ZoneMode2048default.effectSettings`, loaded into the
2048 key table by `FUN_01672120`, which binds `Zone %s.Window Colour 1.Gradient 1` ... `Track Paint.*`).
Any other path (HD-lineage `Data\Environments\<n>_<name>`, no `published\`) takes the **HD fallback**:
`data/ZoneEnvironmentHDFury/ZoneModeHDFury.effectSettings` (`DLC3` when the path names `Zone_1` to
`Zone_4`; the Detonator pair when the mode is `0xe`) through `FUN_01623120` (`0x01623120`, proposed
`Environment_LoadHDFuryContent`), and the function falls through to the general scene setup without
the primary branch. Static reading, confidence 70: the path tests and the two `goto` targets are
exact; the entry conditions of the whole block (a `0x8181 >> id` mask over game modes and a
`DAT_02009384` flag) were not unwound. Both HDFury files and the `ZoneMode2048` ones ship in Omega's
`data00.psarc`/`data04.psarc` (`ZoneModeHDFury.effectSettings` is 21,160 bytes, HD's size).

## 2048 cross-check (CLAUDE.md, "A 2048 finding is checked against Omega")

**Checked, applies, nothing to wire beyond what is wired.** 2048 and Omega both carry the 14-record
table (identical bytes, ids and thresholds) and the 17-record `*_CLASS` ladder, and both have the
loader fork above. On 2048 the 17-record ladder grades and the 14-record walker is HUD text only
(`Hud_UpdateZoneSpeedClassText` writes the HUD instance's `+0x634`, not the craft's). On Omega the 14-record
walker **does** store `14 - i` to a `+0xcf8` the stage readers read, as HD's `Hud_UpdateZoneSpeedClass`
does to `+0x640`; so Omega's HD-lineage Zone grade is HD's stage law, **measured**, where 2048's HD-lineage
circuits (the DLC `zone_N`, `Vineta_K`) grade off the 17-record ladder's `0x11 - i` clamped to 12, as
`zone-environment-fallback.md` reads it (its stage writer for those circuits stays unfound there) -
a difference between the two titles, **not** a thing Omega's HD-lineage circuits inherit from 2048. 2048's wiring of that (the fallback's
`stages`) is `oag_2048::race::ZONE_FALLBACK`, unchanged here.

## What is wired

- `oag_omega::hud::ZONE_SPEED_CLASSES` (zone-keyed: the walker compares the zone, so the HUD reads the
  zone, not the stage) in `oag_omega::hud::ART`: an HD-lineage Omega Zone race draws `FLASH` beside zone 5
  and `SUB-RAPIER` on the next-class row, as the original's frame does
  (`data/reference/compare-page/omega/zone-vineta-k-z5`). `oag_hud`'s `NextSpeedClass` now reads the rung
  above the one the HUD names (`ZoneSpeedClasses::rung`) instead of `zone_stage + 1`; HD (stage-keyed) is
  unchanged. Tests: `oag_hud::zone_tests::omegas_zone_hud_names_the_class_off_the_zone_on_the_grades_thresholds`,
  `...::the_next_class_row_names_the_rung_above_the_one_the_hud_names`, and the disc test
  `omega_hud_ground_truth::an_hd_era_zone_race_names_the_speed_class_in_the_players_language`.
- `oag_omega::race::ZONE_FALLBACK.stages` (HD's records, already) is now **measured on Omega**, not
  inherited.
- Not changed: the grade. Zone 5 was stage 4 before and after.

## What this leaves open

- **The hue gap is not the ladder.** `omega/zone-vineta-k-z5`: ours draws every Track chunk with HD's Flash
  Track group (purple) and the Scene chunks pale; the original draws the whole frame olive with a dark olive
  road. A render with the shape-name Track rule off (every chunk on the Scene group) turns the walls olive
  (`wall_right_mid` mean RGB 114,139,39 against the original's 38,58,6, hue right, 3x too bright) and the
  road pale blue-white (`0.72 0.91 0.96`, the Scene `Texture Colour` as fog) against the original's dark
  olive: so neither group's HD law reproduces Omega's road. What is left is Omega's GCN surface programs
  (unread), not a stage or a table.
- Which manager owns which walker, and that `DAT_01a735b0` is the object the HUD walkers write.
- The `FUN_016787b0` clamp to 12 against `FUN_015f2d30`'s none: only the HD-key reader matters for
  HD-lineage circuits (assumed, not read: both could run).

## See also

- [HD's ladder](../ps3-hdfury-eu/zone-speed-class-table.md), [2048's text walker](../vita-2048-eu-v104/zone-speed-class-text.md),
  [2048's loader and stage blend](../vita-2048-eu-v104/zone-environment-fallback.md)
- [patch-v107-addresses.md](patch-v107-addresses.md)
- [hd-zone-recolour.md](../../../rendering/hd-zone-recolour.md), "On 2048's container"
