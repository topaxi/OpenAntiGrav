# Zone's sound banks: the bank-path gate is read, the announcer's own trigger still is not

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image
base `0x81000000`. **Nothing here is named** - every function below is cited
by address, confidence held below the 70 the project's naming threshold sets,
because none has the second, independent leg (a cross-build address check, a
runtime trace) the named functions in
[track-and-collision-loaders.md](track-and-collision-loaders.md) carry. Found
while closing `crates/2048/src/race.rs`'s `zone_announcer: None` the way
`oag_pulse`/`oag_pure`/`oag_hd`'s were closed - see
`docs/formats/psp-audio.md#speech_zonebnk-names-the-zone-announcer-one-ladder-per-title`.

**2026-08-28, second pass: the bank-path gate (item 1 below) is read, and it
picked an answer.** `crates/2048/src/race.rs::ZONE_ANNOUNCER` is wired to
`data/audio/sound/speech_zone_NGP.bnk`. The milestone table's own reader
(item 2) is still not found, despite a second, wider sweep - see its own
section below.

## The Zone speech bank is picked at track load, and the picker is real code

`FUN_8121bce2` (body `0x8121bce2`-`0x8121cb51`) is the function that loads a
circuit: its first write is `param_1[0xb] = "Backend/World/Track.cpp"`, a debug
tag on the object it constructs, the same convention
[`RcsModel_Load`](track-and-collision-loaders.md)'s own naming rests on for a
different class. Confidence **65** on the identification itself (`_q`-worthy
by [CLAUDE.md](../../../../CLAUDE.md)'s rule, held out of `names.tsv` rather
than written in with the suffix, since nothing here is a rename) - the tag
string is strong evidence for *what kind of object* this constructs, not proof
this is the whole of what a `Track_Construct` does, and it has not been
checked address-identical against `/2048/eboot-vita-2048-usa-v104.elf` the way
every named function in the sibling doc has been.

Deep in the function, gated on a mode/track-type variable (`DAT_8153fd24`)
and a pack-membership call (`FUN_812b5890`/`FUN_812b0880`, unread - see
below), it opens five banks per race type. The two that matter here, read
directly off the decompile:

```c
// DAT_8153fd24 == 0xd || DAT_8153fd24 == 0x15  (Zone Battle)
DAT_818c4df4 = FUN_81262b54(DAT_81521638, "data/audio/DLC1/speech_zbattle.bnk");

// the base-package fallback, reached when FUN_810018d4(&DAT_8153fc40) != 0
DAT_818c4df4 = FUN_81262b54(DAT_81521638, "data/audio/DLC1/speech_zone.bnk");

// the "has packs" branch's own fallback, reached when FUN_812b0880(iVar1, 1..4)
// all return false and FUN_812b0880(iVar1, 0) is true
DAT_818c4df4 = FUN_81262b54(DAT_81521638, "data/audio/sound/speech_zone_NGP.bnk");
```

So **this title ships (at least) two live Zone speech banks**,
`data/audio/DLC1/speech_zone.bnk` and `data/audio/sound/speech_zone_NGP.bnk`,
both reached through `DAT_818c4df4` - not a guess: this is a decompiled
function actually calling a bank-open API (`FUN_81262b54`) with these two
literal paths on two different branches of one dispatch. A third bank,
`data/audio/DLC1/speech_zbattle.bnk`, backs the separate "Zone Battle" mode -
see below.

## Which of the two paths a base-package circuit takes - read

`FUN_812b5890` is one line: `return DAT_81965b58;` - a plain global, read as
a pointer given how its result is used next. `FUN_812b0880(param_1,
param_2)` is also one line: `return *(int *)(param_1 + 400) == param_2;` - a
field compare, nothing more. So the gate the bank dispatch runs is "does the
current selection object exist, and does its `+400` field equal a given small
integer" - not a mystery function, just two one-liners the decompiler had
already fully resolved.

`DAT_81965b58` itself is written in exactly one place, `FUN_812b5af2(param_1)`:
`param_1 == 0` clears it; otherwise it stores `param_1` itself, copies
`param_1 + 0xc4` into a second global, and calls `FUN_810016ba(&DAT_8153fc40,
param_1 + 0xc4)` - copying the selected item's own data into the race context
that everything else in this function reads. That is the shape of "the player
picked a row in a list", not a per-frame or per-mode flag: cleared on `0`,
otherwise it *becomes* the selection and pushes its fields into the race
setup. A race reached by picking a circuit from the game's own menu - which is
every race a player actually starts - has this set by the time the track
loader runs; the `DAT_81965b58 == 0` branch (the one that reaches
`data/audio/DLC1/speech_zone.bnk`) is what is left over for whatever path
does *not* go through that selection, most plausibly a debug/test launcher
this project's own `oag-game --race` is itself the analogue of.

That makes `+400`'s five branches (`0`..`4`) the selected item's own **pack
id**, and the two branches with concrete evidence beyond "one of five cases"
corroborate it: `+400 == 2`'s branch checks the selection's name against the
literal `"2048 Speed Lap Artists"` - a real, specific pack name, not a guess -
and `+400 == 0` is reached only after `1`, `2`, `3` and `4` all fail, i.e. it
is the *catch-all* case a "no specific pack, therefore the base one" reading
predicts. [`DEFAULT_TRACK`](../../../../crates/2048/src/race.rs) is
`data/art/published/environments/altima/track.vex` - no DLC path segment
anywhere in it - so a race on it is exactly the circuit this reading says
takes the `+400 == 0` branch, `data/audio/sound/speech_zone_NGP.bnk`.

**What this still is not**: a runtime trace. The reading is two one-line
functions and one setter, all fully decompiled, plus one corroborating string
match (`"2048 Speed Lap Artists"`) - real evidence, not proof by construction.
Wired anyway, on the same standing `crates/2048/src/race.rs::SOUND_BANKS`'s
own doc comment already accepts for `shipHD.bnk`: "a choice made for a
reason, not a measurement."

## A milestone name table exists, in the same shape Pulse's does - but its reader was not found

`search_strings` for `zone_` turns up two contiguous arrays of `{name[N],
&name}` records - a name string immediately followed by a pointer to its own
first byte, the shape a compiler emits for a `const char*[]` initialiser whose
elements happen to get pooled next to the array slot referencing them:

- **`0x814875a4`**, fifteen entries: `zone_5`, `10`, `15`, `20`, `25`, `30`,
  `35`, `40`, `45`, `50`, `60`, `70`, `80`, `90`, `100` - **the same ladder
  Wipeout HD's `speech_zone.bnk` carries**, and immediately followed in memory
  by the debug string `"Backend/General/SPZone_RaceManager.cpp"` and then
  `"Data\XML\2048_hud\Zone_HUD_HD.xml"` - two more strings that read as "this
  table belongs to the ordinary Zone race manager, not Zone Battle."
- **`0x814840a0`**, twenty-three entries: `zone_1` through `zone_14`
  individually, then `zone_35`, `40`, `45`, `50`, `60`, `70`, `80`, `90`,
  `100` - a **different** ladder, immediately followed by eleven `MR_*`
  speed-class name strings (`MR_SVE`, `MR_VEN`, `MR_SFL`, `MR_FLA`, `MR_SRA`,
  ...) that reproduce Wipeout HD's own `speech_zone.bnk` speed-class set. This
  table's neighbours (`Zone_Battle_Pad.vex`, `speech_zbattle.bnk`,
  `321Go_HD_Zone_Battle.vex`) all read as the *other* mode, "Zone Battle" -
  which this engine does not implement (`oag_race::Mode` has no variant for
  it), so this ladder is recorded and not pursued further.

**The trap this pass walked into and is writing down so the next one does
not repeat it**: `get_xrefs_to` on the first table's base address
(`0x814875a4`) returns exactly one hit, a `DATA` reference from `0x8151f9b4`.
Reading the sixty-odd bytes around that address looked at first like the
milestone dispatch table itself - the same "walk a triple, compare the first
word, play the second and third" shape
[zone-mode.md](../psp-pulse-usa/zone-mode.md#the-ten-second-step) reads
`g_zone_milestones` as. It is not: `0x8151f9b4` sits inside a table of
`{name_ptr, description_ptr, count}` triples that also references the
already-named `BADGE_NAME_ZONE_PAD_HUNTER`/`BADGE_DESC_ZONE_PAD_HUNTER`
strings a few rows away - a **trophy/badge unlock table**, reusing the same
`"zone_5"` string as one badge's own criterion text, not the announcer's
trigger. One `zone_N` string can have more than one reader, and the reader an
xref search happens to find first is not guaranteed to be the announcer.

## Every direct reader of the Zone speech handle was checked, and none is the announcer

`get_xrefs_to` on `DAT_818c4df4` (the global both Zone bank branches assign)
returns nineteen distinct functions. All were decompiled this pass, not just
skimmed for the word "zone":

| Function | What it actually is |
| --- | --- |
| `FUN_8117e2ca` | Detonator's medal computation - plays `"bronze_med"`/`"silver_med"`/`"gold_med"` through a *different* handle (`DAT_818c4df8`), and a `"ZONEREDUCED"` event through this one, gated by a `{threshold, event_a, event_b}` table walked three words (12 bytes) at a time. **This is Zone Battle's decaying multiplier**, not the ordinary Zone milestone ladder - `DAT_8153fd30`, the value it decrements every 5 real seconds when a zone was not renewed, reads as the multiplier the fourteen-entry `zone_1`..`zone_14` run (the *other* table, see above) most plausibly names, not a plain elapsed-zone count. Out of scope on the same terms the rest of Zone Battle is. |
| `FUN_8120d732` | The weapon-pickup selector - plays `"NGP_PICKUP_Missile"`, `"NGP_PICKUP_Bomb"` etc. by rolling odds and reporting which weapon was drawn. Generic, not Zone-specific. |
| `FUN_811c711e` | The craft state machine - plays `"RESET"`, `"cont_elim"`, `"shipdestroy"` on state transitions. Generic race lifecycle. |
| `FUN_812bebcc` | Race completion - plays `"RACE_Race_complete"` once. |
| `FUN_81175d12` | The start-of-race countdown - plays `"Go_NGP"` (or a name built at `&DAT_814820b0` on the multiplayer branch) at the moment the lights change. |
| `FUN_811626a6` | Time trial / ghost-target logic - plays `"TARGETINRANGE"` when the player closes on a ghost's recorded split. |
| `FUN_81182bd4`, `FUN_8120bb70`, `FUN_811c6ca8`, `FUN_81182076`, `FUN_8111e31a`, `FUN_81181872`, `FUN_81181ba6`, `FUN_812b6716` and the remaining callers | Per-craft HUD/warning-light bookkeeping and autopilot handling, none of which builds a `"zone_"`-prefixed name or walks a table shaped like the milestone one. |

So `DAT_818c4df4` is confirmed as this race's **general** speech-bank handle -
pickups, state transitions, countdown and Detonator's multiplier all share
it, the same way `Data\Sound\speech.bnk` is shared across unrelated cues on
the PSP titles - and no reader of it is the Zone milestone dispatch. Two
readings this rules out rather than leaves open: the milestone table is not
walked through this handle at all (it uses a different bank or a different
play wrapper this sweep did not find), or it is walked from a function this
sweep's reader-of-`DAT_818c4df4` search cannot reach because the call is
indirect (a vtable/function-pointer slot, which does not show up as a direct
`jal`/`bl` xref the way every function above did).

## What would close this

1. **Find the milestone table's own reader.** Two directions neither pursued
   here: search for the *second* likely bank handle (`DAT_818c4df8`, seen
   above playing Detonator's medal cues - a title that ships more than one
   speech handle per race is not obviously done after two) and its own
   readers; or set a watchpoint on `0x814875a4` (the milestone table's base)
   during a live Zone race, the technique
   [`ship-parts.md`](../psp-pulse-usa/ship-parts.md#the-rotation-axis-recovered-from-a-live-read)
   used.
2. **Verify the bank-path reading against real audio.** `data.psarc` is
   extracted now (2026-08-28, `data/extracted/vita/PCSF00007/base/PSP2/data.psarc`,
   1.6 GB - see `data/README.md`), so the blocker on data availability this
   bullet used to name is gone. **The blocker moved to the container
   itself**: `oag_formats::sblk::Bank::parse` on the real
   `data/audio/sound/speech_zone_NGP.bnk` fails with
   `PartialAdpcmBlock { size: 1242097 }`, so this title's `.bnk` is not simply
   the same SBlk container the other three titles share, at least not for
   this file - `docs/formats/psp-audio.md`'s "every title in the lineage
   ships this container" claim has not actually been checked against 2048.
   Whether the header/name-table portion parses (`byte_order_of` did not
   immediately refuse the file) and only the codec/section-length math
   disagrees, or the container itself differs more fundamentally, is unread.

   **Update 2026-10-05 (`v2048-engine`).** The container is the same two-section
   SBlk at descriptor version 5, and 18 of the 29 banks under `data/audio/sound/`
   now parse (names are FNV-1 hashes: [2048-xfx.md](../../../formats/2048-xfx.md#sblk-version-5-banks-names-are-hashes-95)).
   `speech_zone_NGP.bnk`, `speech_fe_NGP.bnk` and `Speech_NGP.bnk` still fail the
   whole-block check, so their waveform section is not plain PS-ADPCM; this
   bullet's own question is still open for them.
