# 2048's Zone class name over `ZONE`: a second walker, HD's fourteen-record table

`eboot.elf` (WipEout 2048, Vita, `PCSF00007`, EU, patch v1.04), ARM Thumb-2, imported at `0x81000000`.
Static reading, so scores stop at 84 ([rubric](../../../reverse-engineering/confidence-rubric.md)); the frame
`data/reference/2048-hud/68-zone-5.png` (HD Campaign, Vineta K Zone, zone 5, `FLASH`) corroborates one rung.

**The question.** `Zone_HUD_HD.xml` authors a `SpeedClass` text above `ZONE`
([2048-hud.md](../../../formats/2048-hud.md)), and the frame reads a class name there. 2048 has two speed-class
tables for Zone and they are not the same: the seventeen-record `*_CLASS` ladder
([zone-environment-fallback.md](zone-environment-fallback.md), `Hud_UpdateZoneSpeedClassWidget`, `0x81197d6c`) grades
the circuit and drives `ZoneLight0`-`9`; **the text comes from a second walker over HD's table**.

## The table: `g_zone_speed_class_text_table` at `0x8151f434`

Fourteen 8-byte records `{ i32 zoneThreshold, char* nameId }`, descending 75 to 0, ids resolved against the language
plugin. **Identical to HD's `g_ZoneSpeedClassTable`** ([zone-speed-class-table.md](../ps3-hdfury-eu/zone-speed-class-table.md)):

| Threshold | Id | Reads | Threshold | Id | Reads |
| ---: | --- | --- | ---: | --- | --- |
| 75 | `IG_HUD_SUPSON` | SUPERSONIC | 16 | `MSC_SPHANTOM` | SUB-PHANTOM |
| 60 | `IG_HUD_MACH1` | MACH 1 | 12 | `Rapier` | RAPIER |
| 50 | `IG_HUD_SUBSON` | SUBSONIC | 7 | `MSC_SRAPIER` | SUB-RAPIER |
| 42 | `IG_HUD_SUPZEN` | SUPER ZEN | 5 | `Flash` | FLASH |
| 35 | `MSC_ZEN` | ZEN | 3 | `MSC_SFLASH` | SUB-FLASH |
| 27 | `MSC_SPPHANTOM` | SUPER-PHANTOM | 2 | `Venom` | VENOM |
| 20 | `Phantom` | PHANTOM | 0 | `MSC_SVENOM` | SUB-VENOM |

The v1.00 USA base (`PCSA00015`) carries the same table at vaddr `0x8150b114`. Reproduce (either file, little endian,
the table is found by the pointer to `IG_HUD_SUPSON`, the first record's name):

```sh
python3 -I - <<'PY'
import struct
b = open('data/extracted/vita/PCSF00007/patch-v104/eboot.elf', 'rb').read()
phoff = struct.unpack_from('<I', b, 0x1c)[0]; n = struct.unpack_from('<H', b, 0x2c)[0]
segs = [s for s in (struct.unpack_from('<8I', b, phoff + i * 32) for i in range(n)) if s[0] == 1]
def f2v(o): return next(va + o - off for t, off, va, pa, fs, ms, fl, al in segs if off <= o < off + fs)
def v2f(v): return next(off + v - va for t, off, va, pa, fs, ms, fl, al in segs if va <= v < va + fs)
name = f2v(b.find(b'\0IG_HUD_SUPSON\0') + 1)
base = b.find(struct.pack('<I', name)) - 4
for k in range(14):
    thr, ptr = struct.unpack_from('<II', b, base + k * 8)
    o = v2f(ptr); print(thr, b[o:b.index(b'\0', o)].decode())
PY
```

## The walker: `Hud_UpdateZoneSpeedClassText` at `0x811538c2` (confidence 80)

Called once, from `FUN_8115c2d8` (`0x8115c982`). Per frame, with `param_1` the HUD instance:

- Lights `hud+0x238 + 4i` (eleven widgets, `ZonePlus` rows) from the counter `*(*(hud + 0x3c) + 0x20)` and the float
  `0x2b228` off `FUN_811704aa()`; blinks them on `hud+0x6b0 % 10 >= 6` while that counter is positive.
- Prints `"%d"` of the counter into the `ZonePlus` rows (`hud+0x20c + 4i`) and its digits into `hud+0x264` (`Zone Txt`
  family), centred by `FUN_8128fdea` width when `hud+0xa9` is clear.
- **The class name:** if `hud+0x280` (the `SpeedClass` widget) is bound, walks `i = 0..13` over the table
  (`piVar5 = &DAT_8151f434; piVar5 += 2`), takes the first record with `threshold <= counter`, and writes its
  string into `SpeedClass` with `FUN_810e34ce(widget, nameId, 0, 1)`; hides `hud+0x564`/`+0x568`, and, past record 0,
  places the next-class row beside the `ZonePlus` row `record[i-1].threshold - counter` below the current one
  (`+32.0` x, `-2.0` y, next record's name into `hud+0x568`). It stores `14 - i` into `hud+0x634` and returns.
- **No zone gate.** Counter 0 matches the last record (`MSC_SVENOM`), so the original writes SUB-VENOM at zone 0 as
  far as this reading goes; ours stays blank at zone 0 (the shared gate `Zone`/`Score` use). **Open**: no frame at
  zone 0 or 1 is on disk (`67-zone-grid.png` is named by two docs and is not in `data/reference/2048-hud/`).

Not the grade ladder: `hud+0x634` here is a field of the HUD instance, a different object from the per-craft `+0x634`
`Zone_UpdateStage` clamps ([zone-environment-fallback.md](zone-environment-fallback.md), fourth pass).

## What is wired and what is not

`oag_2048::hud::ZONE_SPEED_CLASSES` (fourteen ids, `rung_zones` the thresholds ascending, no caption blank);
`HudArt::zone_speed_classes` for 2048. Checked on the HD Campaign's Zone event: zone 3 reads `SUB-FLASH`, 5 `FLASH` (the
frame), 35 `ZEN`, 75 `SUPERSONIC`, each resolved by 2048's own plugin and none a raw key. Test:
`crates/raceplay/tests/vita_2048_hud_event_ground_truth.rs::the_hd_campaigns_zone_reads_its_hd_layout` (disc) and
`crates/hud/src/zone_tests.rs::the_2048_zone_hud_names_the_class_off_the_zone_on_hds_thresholds`.

## The score, and the next-class row (read 2026-11-17, `hud-2048`)

**The score value is centred at runtime (confidence 80).** In the same function the `"%d"` of `*(*(hud + 0x3c) + 0x2c)`
goes into the widget at `hud+0x264` (`Score`), after which: the width of a string of as many `3` characters as the value
has digits is measured (`FUN_8128fdea`; a value of 0 or less measures one character), and while neither `hud+0xa8` nor
`hud+0xa9` is set the widget's x becomes `getter(hud+0x268)` (vtable `+0x60`, `ScoreTxt`'s x, 480) minus half that width.
`Zone_HUD_HD.xml` authors `Score` `align="left"` at `x="0"` in `ScoreFrame` (`OffsetX="480"`), under `ScoreTxt` `align="centre"`,
so the left anchor is a placeholder and the value hangs centred under `SCORE`. Ours: `oag_hud`'s `zone_score` module
draws `Score` centred on a title naming `ZoneScore::CentredOnValue` (HD and Omega measure the same template,
[zone-score.md](../ps3-hdfury-eu/zone-score.md)). **Chosen, not measured:** we centre on the value's own width, which
equals the original's `3`-string only while the font's digits share a width. Test:
`crates/hud/src/state_2048_tests.rs::the_2048_zone_score_value_is_centred_under_its_caption`.

**The next-class row is authored invisible in the HD Zone layout (confidence 75).** `Zone_HUD_HD.xml` puts
`ZonePlus1`-`ZonePlus10`, `NextSpeedClassBG` and `NextSpeedClass` in a block commented "stuff needed for the old hud code to
pick up", each with `color="0"` (alpha 0) and no position, so at `(0, 0)`. `FUN_811538c2` positions the pair beside
`ZonePlus[record[i-1].threshold - counter]` (`+32.0`, `-2.0`; the text a further `+20.0` right) and sets their visible bit, but
writes no colour; `search_instructions` for `ldr.w ... #0x564]`/`#0x568]` finds this function the only HUD reader of the two
widgets. With the rows themselves at alpha 0 and nothing writing a colour, the row is placed and not seen. That agrees with
`68-zone-5.png`, which the data-axis notes describe as `FLASH` / `ZONE` / `5` only. **Not wired, on purpose**: drawing it
would be inventing a colour the layout nulls. Headless Vineta K Zone at zone 5, before and after: `FLASH`, `ZONE`, `5`, no ladder.

**Omega: checked, differs.** Omega's `Data/xml/zone_hud.xml` (the file its `SPZoneRaceManager` loads) is HD's layout, not 2048's
`_HD` one: `NextSpeedClassBG` (385x35 art) and `NextSpeedClass` (`"next speed class"`) are authored visible, `ZonePlus1`-`3`
sit on an arc at their own x and y (`25,76`, `0,135`, `-15,198`), `SpeedClass` is authored at `(130, 35)`, `Score` is a left
`Score` with no `ScoreTxt` caption widget. So Omega does draw the next-class widget, but placed by its own walker
(`FUN_0126b030`, unread for the row) on that arc, not by 2048's `+32/-2` rule, and `zone_speed_classes` is still `None`. **Wired 2026-11-19 (`omega-zone-ladder`)**: Omega's executable carries the same
fourteen records (`0x0197b890`, [zone-speed-class-ladder.md](../ps4-omega-eu/zone-speed-class-ladder.md)) and
`oag_omega::hud::ZONE_SPEED_CLASSES` draws them; where `FUN_0126b030` places the next-class row is still unread
(the row sits on `ZonePlus<k>` where the next threshold falls, as HD's does, and matches the original's frame).

