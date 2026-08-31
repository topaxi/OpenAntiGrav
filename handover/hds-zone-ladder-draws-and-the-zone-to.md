# HD's Zone ladder draws, and looking for its speed-class table found the stage writer nobody could find

2026-08-31, branch `worktree-hd-zone-hud`. Full write-up in
[hd-hud.md](../docs/formats/hd-hud.md#zones-ladder-draws-and-the-widget-it-is-missing-is-missing-for-a-reason).
**The one to know**: the gap was three gaps, and each one alone leaves the
picture empty - the sprites were off `oag_hd::hud::ALWAYS_ON` (the set was read
off a *speed lap* frame, which authors none of them), the eleven `ZonePlus<N>`
labels carry no `idstring` and so fell off `draw_list`'s text allow-list, and
`RotationTheta` was neither parsed nor correctly applied. Checked against a Zone
frame of the running original the maintainer supplied
(`data/shots/hd_zone_hud_original.png`, gitignored) and reproduced at
`data/shots/hd_zone_hud_ours.png`.

## A renderer bug fell out of it, and it had been latent since the reticle landed

`ui.wgsl` rotated the **unit square** and scaled by the rectangle afterwards -
`scale . rotate`, which shears every quad that is not square. `ZoneBG` is a
676x153 bar at a quarter turn and came out lying on its side at the wrong size.
Every previous caller of `Draw::RotatedSprite` was the lock-on reticle at 8
pixels square, where the two orders agree exactly, so nothing had ever shown it.
Now in pixels. **Worth knowing for any future rotated HUD widget**: a square
test case cannot catch this.

## What is established

- **The fifteen rungs' names**, `oag_hd::hud::ZONE_SPEED_CLASSES`, confidence 84.
  Three independent readings agree one for one: `zonemode.effectsettings`'
  palette keys (`0 Start` .. `14 Supersonic`), the language plugin's own HUD
  strings (`MSC_SVENOM` = `SUB-VENOM` .. `IG_HUD_SUPSON`, with `IG_HUD_MACH1`
  filling `13 Mach 1`), and the reference frame reading `SUB-VENOM` **with the
  string table's hyphen** rather than the palette key's space.
- **`ZonePlus<N>` is `zone + N`**, confidence 85: the widget name, the authored
  placeholders (`1`..`10`, with `ZonePlus0`'s empty - the same list at zone 0),
  and the frame reading `1` to `11` at zone 1.
- **The two retro skins author `CurrentZonePanel` (and, on `2097_hud`, `ZoneBG`)
  as bare grouping elements with no `<Values>`**, so their ladders have no
  column and no highlighted row. 13 / 12 / 11 sprites, asserted.

## The table was found, and it closed a second question on the way

The maintainer asked for it to be recovered rather than fitted, and it is:
`g_ZoneSpeedClassTable` at `0x00860d44`, fourteen 8-byte records of
`{ u32 zoneThreshold, u32 stringIdPointer }` descending to zero, walked by
`Hud_UpdateZoneSpeedClass` (`0x00049718`). Full evidence, addresses, three
accessors and a reproduce script in
[zone-speed-class-table.md](../docs/ghidra/functions/ps3-hdfury-eu/zone-speed-class-table.md);
it is `oag_hd::race::ZONE_STAGES`. Bands `0`-`1`, `2`, `3`-`4`, `5`-`6`,
`7`-`11`, widening to fifteen at the top.

**How it was found is worth writing down, because the obvious searches had all
failed.** Not from the code side at all: the *string ids* the HUD shows
(`MSC_SVENOM`, `IG_HUD_MACH1`, ...) were already known from the language plugin,
and `grep`ping the ELF for one of them landed in a contiguous fourteen-string
blob; the fourteen pointers to it were the table. Three earlier passes had
searched for the consumer by offset and by dataflow and come back empty. **On a
binary where an offset search is defeated by folded index bias, a known string
is a better handle than a known field.**

**And the walker's last instruction is `stw r3, 0x640(r29)` with `r3 = 14 - i`** -
the writer of the per-craft Zone stage index that
[hd-zone-stage-textures-are-grounded.md](hd-zone-stage-textures-are-grounded.md)
and [2048-and-hd-ship-an-unread-effectsettings-table.md](2048-and-hd-ship-an-unread-effectsettings-table.md)
both had open as "no writer found". So **HD's Zone colour grade escalates now**,
`oag_title::RaceDefaults::zone_stages` is filled in for this title, and the HUD's
class name and the circuit's palette are the same index by construction. It is
2048's architecture exactly: the HUD widget drives the grade.

`14 - i` lands on `zonemode.effectsettings`' fifteen rungs one for one, which is
fourteen independent agreements between a table in `.data` and a table in an
asset file.

## Open

- **What sets the float `Hud_UpdateZoneSpeedClass` compares.** It arrives as
  `f1` from its single caller, which forwards its own argument; nothing traced
  it back to the zone counter's storage. The units rest on the two observations
  from the running game instead (the frame's `SUB-VENOM` at zone 1, and the
  maintainer's zone 2 = Venom), which is a stronger footing than a dataflow
  trace on this binary usually gets.
- **On this reading, a Zone race with its HUD hidden would freeze the colour
  grade**, the store to `+0x640` being inside the HUD-text update. That is what
  the code says and it has not been checked against the running game.
- **The rotation's sign is passed through, not verified.** `ZoneBG`'s quarter
  turn covers the same pixels either way and the ticks are too small to read off
  the frame.
- `ZonePlusLight*` author `color="FEGlobals->HD_Blue"`, which no HUD file
  declares, so they draw white. `FEGlobals` is a live binding rather than a
  load-time constant (`docs/formats/fexml.md`); what the runtime binds is
  unread.
- **The cross-fade's own rate is still unrecovered**, on both titles - the
  sibling threads' remaining question, untouched by this.

## A zone-8 frame settled the placement and confirmed the table a third time

The maintainer supplied it (`data/shots/hd_zone_hud_original_zone8.png`,
gitignored; low contrast, the Zone palette at that rung being nearly white). It
reads `8  SUB-RAPIER` on the current row and `RAPIER` on row `12`, four down.

- **The table's widest band so far, confirmed**: zones `7`-`11` on Sub Rapier
  with the bump at `12`. A five-zone band is the case no one-rung-per-zone
  reading could ever produce, so this is a genuinely independent check rather
  than a restatement of the zone-1 frame.
- **The next class's name sits on its row's own line**, same font and size as the
  digit beside it, inset by the 66 units `SpeedClass` already has from
  `ZonePlus0`. The zone-1 frame could not separate that from a competing reading
  - they are six screen pixels apart at `n=1` - and a four-row gap can.
  `the_zone_eight_frame_is_reproduced_row_for_row` pins it.

## Next Steps

- **Check the grade actually escalates in a long race now**, which nothing has
  watched: `just play hd --race --mode zone --ticks 40000 --screenshot`, at 600
  ticks a zone, should walk the palette up the ladder.
- Look for the same `{threshold, stringId}` shape on **Detonator**, whose own
  ladder is recovered by a different mechanism (`RaceManager->+0x2e10`) and may
  or may not share this table's walker.
