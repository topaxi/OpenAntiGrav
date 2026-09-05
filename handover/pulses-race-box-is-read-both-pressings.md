# Pulse's race box is read on both pressings, and the code side is blocked

2026-09-05. The Pulse half of the race-box investigation, PSP and PS2 together
because the PS2 is a delta rather than an independent measurement. The
permanent write-up is
[`docs/formats/race-setup.md`](../docs/formats/race-setup.md).

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

## Next Steps

1. **Capture `Track Creation` and `Team Selection` in PPSSPP.** One capture
   answers what the previews look like framed, whether `Top->Ship` draws a ship
   on the track screen, and what the info panel shows for a locked circuit -
   three open items for one action. `docs/reverse-engineering/ppsspp-debugger.md`.
2. Once the PSP relocation patch lands, decompile the `TrackSelection` class
   and find the list-population site. That is the same missing piece HD's
   `docs/ghidra/functions/ps3-hdfury-eu/track-selection-screen.md#not-found`
   records, so solving it on either title informs the other.
3. Decode `WADS2.WAD` entry 3410 and confirm it is `hex_bg`, then add the row
   to `docs/formats/ps2-texture.md`. Five minutes.
4. Do not rename either screen class in Ghidra until its code is actually read.
   The strings are proof the classes exist, not evidence of what any function
   does, and the rubric's floor is 50 to rename at all.
