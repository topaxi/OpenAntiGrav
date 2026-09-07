# Pulse's race box is read on both pressings, and the code side is blocked

2026-09-05. The Pulse half of the race-box investigation, PSP and PS2 together
because the PS2 is a delta rather than an independent measurement. The
permanent write-up is
[`docs/formats/race-setup.md`](../../docs/formats/race-setup.md).

## What is settled

The three-page flow, read straight out of the disc's own front-end XML:

```
Main Menu (FE_RACEBOX) -> Single Player -> Track Creation -> Team Selection -> Launch Game
```

`Single Player` authors five lists - Mode, Class, Weapons, Difficulty
(persisted as `SkillLevel`), Eliminations. `Track Creation` and `Team Selection`
author their chrome but bind their rows from code. Both previews are meshes.
See the docs page for the widget inventory, the two unlock axes and the
confidence scores.

**The PS2 pressing's race-setup page is identical** - same lists, same globals,
same idstrings, same defaults, same `GSDisableEntriesBitField="0xFA"`. Its
divergences are all around the page, and they are recorded on the docs page
too: the `Racebox` menu bypassed, split screen added, `Team Selection` widget
names `0`-suffixed, `Tournament C` widened from four track slots to twelve.

**Both previews are now captured live in PPSSPP, not just read out of
`.rodata`** - 2026-09-05, see
[the docs page's capture section](../../docs/formats/race-setup.md#captured-live-in-ppsspp-2026-09-05).
`Track Creation`'s preview is an animated first-person flythrough of the
circuit corridor in a hexagonal frame; `Team Selection`'s is a turntable-
rotating craft on black. The three-entry wrapping track list was walked all
the way round and confirmed to be exactly `16_Track`/`03_Track`/`18_Track`
with no locked entry reachable from this screen. `Black`/`White` in the
`TrackSelection` string block are confirmed to be the two circuit runs (the
screen's own Help text says so), not a preview toggle.

## Open

- **The two screen classes cannot be followed in Ghidra.** `TrackSelection`
  (`0x08a84a00`) and `TeamSelection` (`0x08a84688`) exist as strings in
  `/psp-pulse-usa/BOOT.BIN`, and every route to their code is blocked:
  `get_xrefs_to` returns "No references found", a byte scan for the literal
  pointer finds nothing, and `search_instructions` for the `addiu` immediate
  finds zero matches across 525,049 instructions. That is the defect in
  `handover/ghidra-applies-no-psp-relocation-the-patch-is.md` - **a tooling
  block, not an absence.** The patch is written and not yet installed.

  The way around it that already worked: the screen classes' string blocks sit
  contiguously in `.rodata` and terminate in the class name, so
  `inspect_memory_content` over the block recovers the asset templates and the
  widget names without any xref at all. That is how both previews were
  resolved. **It does not recover behaviour** - only names.

- **What populates the track list is unread.** `Track Creation` authors no
  `<List>` at all. The three-entry Custom Race list is consistent with the
  three `PI_Track` entries carrying no `<Unlock>`, but the code that reads
  `Definition.xml` and builds the rows has not been found.

- **`GSDisableEntriesBitField` is undecoded.** Read as "bit N disables entry
  N", confidence 72 - self-consistent on Pulse's `0xFA` over seven entries and
  on HD's `0xF0` over five, and in both cases every bit at or above the entry
  count is set. Nothing traced to the consuming code.

- **Six fields in the serialised race record have no authored row**:
  `Locked`, `damage`, `AICount`, `laps`, `ship`, `ShipChoice`. `ShipChoice="Yes"`
  might be the flag deciding whether `Team Selection` is shown at all, which
  would make the third page conditional - confidence 45, do not build on it.

- **Five of Pulse's 23 definition files do not resolve in `Data.wad`**
  (`Controls_`, `Credits_`, `Debug_Screens`, `MemoryStickBootScreens`,
  `MemoryStickScreens`). On Pure, `Controls_Definition` *does* resolve and
  carries no `localised` attribute, so Pure's own two-file failure has a
  different cause and does not explain Pulse's five. Both are still open.

- **`RB_YG` is a dead idstring** - referenced by a redirect in
  `Selection_Definition.xml`, on no menu anywhere.

- `hex_bg.pct` at `WADS2.WAD` entry 3410 looks like a texture resolution not
  yet listed in `docs/formats/ps2-texture.md`, falling straight out of that
  page's own `.mip` -> `.pct` rule. Unverified by eye.

- **`Top->Ship` still has no confirmed referent.** The 2026-09-05 capture
  found no ship model, silhouette or top-down framing on `Track Creation`
  under any of `square`/`triangle`/`select`/`up`/`down` - a bounded negative,
  confidence 65, not proof the path never renders anything. Unread in code.

- **The locked-circuit info panel is confirmed unreachable from `Track
  Creation` itself** (the wrapping list only ever holds the three ungated
  tracks), but still genuinely unanswered: reaching one needs `Tournament C`
  (a different screen, own binding) or campaign-grid progression, neither
  tried.

- **The `Info Track %d.%d` = (count, index) layout-selection hypothesis is
  untested rather than confirmed or refuted.** All three of Pulse's reachable
  circuits rendered the same single-row layout; nothing in this capture
  exercised the 2-row or 3-row templates the string block also lists.

## Next Steps

1. Once the PSP relocation patch lands, decompile the `TrackSelection` class
   and find the list-population site. That is the same missing piece HD's
   `docs/ghidra/functions/ps3-hdfury-eu/track-selection-screen.md#not-found`
   records, so solving it on either title informs the other.
2. Decode `WADS2.WAD` entry 3410 and confirm it is `hex_bg`, then add the row
   to `docs/formats/ps2-texture.md`. Five minutes.
3. Do not rename either screen class in Ghidra until its code is actually read.
   The strings are proof the classes exist, not evidence of what any function
   does, and the rubric's floor is 50 to rename at all.
