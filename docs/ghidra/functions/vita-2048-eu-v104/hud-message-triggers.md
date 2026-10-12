# What raises 2048's HUD message lines

**Binary:** `vita-2048-eu-v104` `eboot.elf`, image base `0x81000000`, Ghidra
`/2048/eboot-vita-2048-eu-v104.elf`. Read 2026-10-12 (`hud-messages-port` lane).
Companion to HD's [hud-message-triggers.md](../ps3-hdfury-eu/hud-message-triggers.md),
which this title's routine reads as; the Omega page is
[../ps4-omega-eu/hud-message-triggers.md](../ps4-omega-eu/hud-message-triggers.md).

**A 2048 finding is checked against Omega and the reverse.** Result: **ported**
(2048 and Omega now raise the lap lines on their own layouts, see the end).

## The drain

| Address | Name | Confidence | What it is |
| --- | --- | ---: | --- |
| `0x81150730` | `Hud_UpdateMessages` | 85 | Called once a tick from `FUN_8115c2d8` (the HUD update). Drains `*(*(hud+0x3c)+0xf4)` into four slots at `hud+0x118` (sixteen bytes each). |

Decompiled in full. It is **HD's `Hud_UpdateMessages` (`0x00091d20`) with the
offsets moved**, not Pulse's:

- the same fifteen bits in HD's numbering (`0x1` `IG_HUD_PLAP`, `0x2`
  `IG_HUD_NEWLAP_REC`, `0x4` `IG_HUD_FIN_LAP`, `0x8`/`0x10` the contender-eliminated
  pair, `0x20`/`0x40`/`0x80` `ER_GMA`/`ER_SMA`/`ER_BMA`, `0x100`-`0x800` the zone
  and score lines, `0x1000` `IG_HUD_INVALID_LAP`, `0x2000` the queue, `0x4000`
  `IG_HUD_TARGET_ATTAINABLE`);
- **plus `0x8000`**: a formatted `MSC_RACE_ENDS` line (the race ends in N seconds,
  a multiplayer notice), 2048's own;
- `CONT_ELIM`'s two bits push `IG_HUD_CONT_ELIM` and play `cont_elim` when one
  global test (`FUN_81000930(&DAT_8153fc40,-1)`, unread; it also picks
  `PERFECT_LAP` over `RACE_Perfect_lap`) is nonzero, and **`HUD_Message_Ship_Destroyed`**
  when it is zero: 2048's own phrase;
- the same slot law: gate `0x3f4ccccd`, the `100.0` sentinel, life `4.0` (or `8.0`
  by `FUN_810006fc`), alpha `sin(pi * (1 - (t/life)^6))`, green `0x30ff30`, red
  `0xff3030`, the retro skin flag (`hud+0x6a8`) halving the alpha;
- the same `FrameMiddle%d`/`FrameLeft%d`/`FrameRight%d` backing, faded with the
  line, and a `MESSAGE` cue as a slot starts;
- `IG_HUD_NEWLAP_REC` skipped when the game mode `DAT_8153fd24` is `5`, **with no
  profile-byte test** (HD has one);
- the announcer cues `RACE_Perfect_lap`/`PERFECT_LAP`, `RACE_New_lap_record`/
  `NEW_LAP_REC` and `cont_elim`, chosen by the same global; a voice-only branch on
  `hud+0x40 & 0x20`.

## What 2048's layouts author

A census with `oag_raceplay::load` on `data/extracted/vita/PCSF00007` (the composed
layouts, 2026-10-12): `Arcade_HUD.xml`, `TimeTrial_HUD.xml` and `Zone_HUD.xml` all
author `InfoTextParent`, `Info`, `Info2nd`, **`Info1`-`Info4`**, `WrongWay` and
`TimeDiffText` (no `TimeDiffIcon` sprite), and its table resolves `IG_HUD_PLAP`
("PERFECT LAP!"), `IG_HUD_NEWLAP_REC`, `IG_HUD_FIN_LAP`, `ER_GMA` and
`IG_HUD_INVALID_LAP`. **The earlier statement that 2048 "has no `Info1`-`Info4`"
was wrong**; those widgets were never counted in the composed layout.

## The gap: bound, not read

`FUN_81158d88` (the widget bind) stores `TimeDiffText` at `hud+0x204` and
`TimeDiffIcon` at `+0x208`; the constructor `FUN_8114f498` stores the literals
`"- %.1f"` and `"+ %.1f"` (`0x8147b18c`, `0x8147b194`) at `hud+0xc4`/`+0xc8` -
HD's shape (`+0xcc`/`+0xd0` there), the wrong-way phrase at `+0xc0`. The routine
that reads them was **not found** (the one `ldr.w rN,[rM,#0x204]` hit,
`FUN_8116ea40`, is an unrelated 5 KB function). So 2048's gap is wired with HD's
rule as **chosen, not measured**.

## Wired

`oag_2048::hud::ART`: `message_slots: true`, and `lap_hud` with the three ids,
`record_line_in_time_trial: false` (read), `MessageTint::GoodBad` (read), and the
gap rule from HD with **one split a lap** (HD's thirds are not verified here;
the lap function and split table are unread) and a 100-crossing table (chosen).
Disc-backed: `crates/game/tests/lap_messages_ground_truth.rs`
(`the_2048_lap_lines_run_on_its_own_layout_and_table`), checked on the default
circuit: "PERFECT LAP!" and "FINAL LAP!" green, a held "+ 3.9" red, the count-up.

## Open

- 2048's gap routine and its split table (thirds or not).
- The lap function's writers of `0x1`/`0x2`/`0x4` (assumed HD's conditions).
- `MSC_RACE_ENDS` (`0x8000`) and the voice cues.
