# Wipeout Pure's campaign: the medal law and the unlock rows

**Binary:** `pure-psp-usa` `BOOT.BIN` (`UCUS98612`), image base `0x08804000`. Read
2026-10-12 by the `pure-campaign` lane, to build Pure's campaign list
([`docs/formats/pure-campaign.md`](../../../formats/pure-campaign.md)). The data
the law reads is on the disc; this page is the code that turns a finished race into
a medal and a medal into an open gate.

**The import trap applies** (see the first paragraphs of [ai-controls.md](ai-controls.md)):
strings and tables the decompiler prints as `0x2410f0` or `DAT_002886a0` are
addends, the loaded address being `0x08804000 +` that (segment 1). Every address
below is the loaded one.

## `Medal_Evaluate` (`0x088081d8`)

Confidence **88**: decompiled and read in full as disassembly, including the jump
table it dispatches through. Signature as read: `(profile, value, place, count)`.

It returns the medal this result earns - `0` gold, `1` silver, `2` bronze, `0xff`
none - and does three things with it:

1. **Dispatches on the game mode** (`mode - 3`, the same ordinals
   `oag_tables::race_campaign::Mode` carries) through the nine-entry table at
   `0x08a444b0`. Entries are code addends: modes 3 and 9 go to `0x088082bc`,
   4 and 10 to `0x08808480`, 5 and 11 to `0x08808314`, 6 to `0x08808424`, 7 and 8
   to the common tail `0x0880852c` with no medal.
2. **Keeps the better stored medal** per `(track, class)` or per track, writing
   it into the profile record only when the new one is better.
3. **Raises a profile counter** at the tail (`0x0880852c`): gold `+0x7c`, silver
   `+0x78`, bronze `+0x74`. See [what is not determined](#what-is-not-determined).

| Mode | Handler | What is compared | Stored at |
| --- | --- | --- | --- |
| Race (3), Head2Head (9) | `0x088082bc` | the finishing place, one-based: `place - 1 < 3` is the medal, so 1st gold, 2nd silver, 3rd bronze, 4th and below none | track record `+0x71a`, `+0x21` a class |
| Tournament (4), SpeedLap (10) | `0x08808480` | **only on the last race** (`race index == race count - 1`), the final standings rank, the same `< 3` rule | the tournament's own record, `+0x21` a class |
| TimeTrial (5), CustomGrid (11) | `0x08808314` | the time in **centiseconds** against three limits: `RaceTimes[class] * 100`, `(RaceTimes[class] + Silver) * 100` and `(RaceTimes[class] + Bronze) * 100`, taking the first limit the time does not exceed | track record `+0x654`, `+0x21` a class |
| Zone (6) | `0x08808424` | the zone count against `ZoneMedals` gold, silver, bronze (`+0xc4`, `+0xc8`, `+0xcc` of the stats object), **highest first**: a count at or above the figure earns it | track record `+0x6f9` |

The three limits and the `ZoneMedals` ints are the track `stats.xml`'s own, parsed
into the stats object by `TrackStats_ParseElement` (`0x088d9390`,
[ai-controls.md](ai-controls.md)): `RaceTimes` lands at `+0xa8` (five class floats),
its `Silver` and `Bronze` attributes at `+0xbc` and `+0xc0`, and `ZoneMedals` at
`+0xc4`. So the silver and bronze figures are **offsets in seconds from the class's
gold time**, not absolute times.

**Checked against the running engine, 2026-10-12.** The lone autopilot flew Vineta K
at Venom in 111.78 s. The disc's figures there are gold 105, silver +3, bronze +6:
111.78 s sits just past the 111 s bronze limit, which only holds if the unit is a
whole race of the title's default three laps in seconds. Had it been one lap, or
hundredths misread, the number would be off by a factor, not by seconds
(`pure_chain_walk_ground_truth`).

## Tournament points

`FUN_0880f4a4` returns a leg's points for a craft: a table lookup at
`DAT_002886a0` (`0x08a8c6a0`), indexed by finishing place. The table reads
`[0, 8, 6, 5, 4, 3, 2, 1, 0]`: 1st 8 points down to 8th 0. That is
[`oag_race::tournament::POINTS_BY_POSITION`](../../../../crates/race/src/tournament.rs)
unchanged, which Pulse measured; Pure shares it. Confidence **85** (a static read of the
table and of the single caller's index).

## `Unlock_ParseElement` (`0x088b4568`)

Confidence **80**. Allocates a `0x3c`-byte record per `<Unlock>` and walks its
attributes through a thirteen-way chain of name compares. The strings are at
`0x08a52340`..`0x08a523c8` (`MedalCount` is `0x08a523b4`). The fields it fills:

| Offset | Attribute | Offset | Attribute |
| --- | --- | --- | --- |
| `+0x00` | `Exclusive` (a bool) | `+0x24` | `Team` |
| `+0x04` | next pointer | `+0x28` | `Track` |
| `+0x08`..`+0x1c` | six context filters (`ClassIs`..`GridIs` in Pulse) | `+0x2c` | `Tournament` |
| `+0x20` | `Class` | `+0x30` | `Medal` |
| | | `+0x34` | `MedalCount` |
| | | `+0x38` | `TimePlayed` |

Pulse's same record has `GridIs` where Pure has `LeagueIs`, a `Grid` field Pure lacks
and a `Loyalty` field Pure lacks, so every field from `Medal` on sits one slot earlier
(Pulse: `Medal` `+0x34`, `MedalCount` `+0x38`;
[the Pulse table](../psp-pulse-usa/race-campaign.md)). The accessor for `Class`
(`0x088b4f88`) resolves the literal `asSelected` (`0x08a52400`) to the class being
played: a row with `class="asSelected"` is **per class**.

## Confirmed shape, and the one thing the executable does not say

The shipped file authors 17 rows and no `Exclusive` attribute, so they are ANDed
within a row and across rows (Pulse's `Definition_IsUnlocked` combine, which Pure's
parser shares by layout). Seventeen rows, three conditions, all modelled by
`oag_tables::pure_campaign::Unlock::met`.

## What is not determined

- **How the profile counts medals for `MedalCount`.** The tail raises the three
  counters on every award, repeat or not; whether `MedalCount` reads their sum, and
  whether a replay raises them again, was not traced (the evaluator of the
  `MedalCount` field was not found). The list counts each event's best medal once
  (**chosen, not measured**), so a replay cannot farm a gate.
- **Which class opens when.** A fresh profile's `Class Selection` lists only Vector
  and Venom (PPSSPP capture, 2026-09-10); the gate is in code, not in
  `Definition.xml`. Read 2026-10-12 (`pure-campaign-authored`): the strings
  `Unlock Track`, `Unlock Liveries`, `Unlock Flash Class`, `Unlock Rapier Class` and
  `Unlock Phantom Class` (`0x08a7c800`..`0x08a7c85c`) are entries 6 to 10 of an
  eleven-entry reward-name pointer table at `0x08ab4a0c` (the pointers read `0x2787a0`..`0x278848`
  as addends), whose one code reference is `FUN_08951538` (`0x08951538`), a per-frame routine
  that shows the entry its index names and advances the index when one button (index 5) is
  pressed: the reward reveal. So the classes open as **rewards** (confidence 60). The
  menu attribute that hides an entry, `DisableEntriesBitField`, is parsed in the menu
  reader `FUN_088c76a0`; **what raises a reward, and who sets the class menu's bitfield,
  was not traced** (confidence under 50, so no renames). This build offers Vector and
  Venom from `Campaign::classes_at_start` and opens a later class on any tournament medal
  in the class before it (**chosen, not measured**).
- **The time trial's lap count.** `RaceTimes` is a whole-race figure; the list
  races a time trial at the mode's own default laps, which the 111.78 s check above
  agrees with, not a measured Pure figure.
- **The profile counters' own persistence** and the `Medal_%d` and `%s&Medal=%d`
  save strings (`0x08a7c41c`, `0x08a7db78`): the original saves through PSP
  save data; this project does not (modern saves).
