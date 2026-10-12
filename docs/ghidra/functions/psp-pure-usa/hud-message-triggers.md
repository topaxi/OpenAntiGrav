# What raises Pure's HUD message lines, and its lap-line time gap

**Binary:** `psp-pure-usa` `BOOT.BIN`, image base `0x08804000`, Ghidra
`/pure/BOOT-psp-pure-usa.BIN`. Read 2026-10-12 (`hud-messages-port` lane).
Companion to Pulse's [hud-message-triggers.md](../psp-pulse-usa/hud-message-triggers.md)
and HD's [hud-message-triggers.md](../ps3-hdfury-eu/hud-message-triggers.md).
Disassembly was taken with `llvm-objdump -d --triple=mipsel` over the file and the
address added to `0x08804000`; the decompiles are Ghidra's.

Pure is Pulse's older sibling and the same machine in a different dialect: the
same four slots and the same gate and curve, **its own phrases, drawn white, and a
gap worded as a sentence**. A previous page left Pure's lines off on the ground
that its table lacked Pulse's `IG_HUD_*` ids; they are there under `HUD_<English>`
keys.

## Functions

| Address | Name | Confidence | What it is |
| --- | --- | ---: | --- |
| `0x0881f2ec` | `Hud_UpdateMessages` | 85 | Drains `*(*(hud+0x40)+0xdc)` into four slots (`hud+0xd0`, sixteen bytes each); called from `Hud` update `0x0881e6bc`. |
| `0x0881f780` | `Hud_UpdateTimeDiff` | 85 | The gap display, also from `0x0881e6bc`. |
| `0x0881e2a4` | `Hud_Construct` | 80 | Resolves the phrases through `FUN_088b8d0c(table, key, 0)` into `hud+0xa0`-`+0xb0` (`HUD_Ready...`, `HUD_GO!`, `HUD_Wrong Way!`, `HUD_You are %.1f secs ahead` at `+0xac`, `... behind` at `+0xb0`). |

## The eight bits and their keys

All eight keys are **language-table keys that resolve** (`HUD_Perfect Lap!` ->
"Perfect Lap!", checked against the loaded table on 2026-10-12). The id strings
are at `0x08a46330`-`0x08a463c0`; `Hud_UpdateMessages` pushes each through
`FUN_0881fca0(hud, id)` (the first free slot, timer `100.0`, **no good byte**).

| Bit | Key | Setter in a race |
| --- | --- | --- |
| `0x1` | `HUD_Perfect Lap!` | the lap function, `0x08927c50` |
| `0x2` | `HUD_New Lap Record` | the lap function, `0x08927bf4` |
| `0x4` | `HUD_Final Lap!` | the lap function, `0x08927d68` |
| `0x8` | `HUD_Contender Eliminated!` | **none found** |
| `0x10` | `HUD_Perfect Zone!` | none found |
| `0x20` | `HUD_Perfect Boost!` | none found |
| `0x40` | `HUD_New Zone Record` | none found |
| `0x80` | `HUD_New Score Record` | none found |

A scan of the whole disassembly for `sw rN,0xdc(rM)` finds seventeen stores; the
three above are the only ones that `ori` a bit into a loaded flag word through
`lui/lw 0x40(...)` the status block. `0x0892a034` increments a profile counter
(`addiu 1`, a jump table of statistics), `0x088b5a78` and the rest are other
structs. So the five non-lap bits have no setter in Pure's race path (75, a census
and not a live watch), the same as Pulse's `PERF_ZONE` and the rest.
**`HUD_Perfect Zone!` and `HUD_New Zone Record` are therefore strings Pure ships
and never raises**; the earlier lead that Pure announces a medal in Zone through
these lines is closed negative (its `zone_bronze`/`silver`/`gold` voice lines are
the Zone announcer and a separate question).

## The slot law: Pulse's, drawn white

`0x0881f2ec` is Pulse's machine: the gate `0x3f4ccccd` (0.8 s), life `0x40800000`
(4.0 s), the waiting sentinel `100.0`, alpha `sin(pi * (1 - (t/4)^6)) * 255`, the
`MESSAGE` cue (`FUN_088308f0(.. 0x2423d8, 0x400)`, the string at `0x08a463d8`). The
differences:

1. **Every line is white.** The one colour call per slot per tick is
   `FUN_088d2264(widget, alpha << 24 | 0xffffff, 0)`: there is no good/bad
   argument and no `0x30ff30`/`0xff3030`. (`FUN_088d2264` is the text-colour setter
   Pure's gap also uses, the counterpart of Pulse's `FUN_088cd290`.)
2. **The outline does not follow the alpha.** Pulse's drain follows the colour call
   with `FUN_088cd3a4`, which scales the border's alpha with the text's. Pure's
   has no second call, so the layout's `BorderColor` stays as authored while the
   text fades. 70: the absence of a call is read off one decompile, not seen live.
3. **No `Cont_Elim` good/bad pair**: one bit, one key.
4. **Life is always 4.0**: no 8.0 branch for a later mode.

Pure's layouts author `Info1`-`Info4` (`docs/ui/hud.md`), centred; the loaded
layout has no `FrameMiddle1` sprite, so there is no backing to draw.

## The lap function's conditions

The three setters are in the function containing `0x08927a00`-`0x08927d70`
(Pure's counterpart of `Craft_UpdateLapProgress`; its prologue was not located
and it is left unnamed). `r16` is the craft, `r17` the game state:

- **`0x2` new lap record**: the profile test `0x08806414`
  (`Profile_IsNewLapRecord`'s counterpart, called as `jal 0x2414` from `0x08927bc0`) is nonzero (a stored record to beat),
  `craft+0xa4c == craft+0xa48` (this lap is the race's best; `0xa4c` is set from
  `0xa48` when it is zero or beaten and a second test, `0x08924758(craft)`,
  agrees), the craft is the local one (`+0x488 == 0`) and not finished
  (`+0xa32 == 0`): Pulse's law.
- **`0x1` perfect lap**: `+0x488 == 0`, `+0x9a6 == 0` (no wall hit since the last
  completed lap; cleared at `0x08927d40` when a lap completes) and `+0xa32 == 0`.
- **`0x4` final lap**: `+0xaa4 == game+0xc` (the crossing count equals the lap
  count) and `+0xa32 == 0`.

None fires on the finishing crossing (`+0xa32` is set at `+0xaa4 == game+0xc + 1`
before them). The lap time `+0xa48` is `(int)(seconds * 100.0)` (`0x42c80000`),
centiseconds.

## The gap

`Hud_UpdateTimeDiff` reads the status bytes `PLAYER_HUD+0x54..0x56`, `+0x4c`,
`+0x50` into `hud+0x90..0x9c` exactly as Pulse's does, and **the display is
Pulse's, with Pure's words**:

- the held gap is the language-table phrase at `hud+0xac` (`HUD_You are %.1f
  secs ahead`, green `0xff10c010`/`0xff30ff30`) or `hud+0xb0` (`... behind`, red
  `0xffc01010`/`0xffff3030`) with `%.1f` filled by the gap, flashing at 5 Hz for
  3.5 s, drawn for `|gap| < 30` - **a sentence, in `TimeDiffText` alone**. Pure's
  layouts author no `TimeDiffIcon`;
- the count-up is `"%.1f"` (`0x08a463ec`) in `0xff60ff60` while the player leads
  and `FUN_0880f394(race_manager, 0) > 1` craft are live, under 30 s;
- **no place stop**: Pulse's hold flag is re-read from `PLAYER_HUD+0x10`; Pure's is
  not.

The table is `race_manager + 0x9c + crossings * 0x18` (`+0` the clock truncated to a
whole second, `+4` a human over, `+5` an AI over) and holds **six** crossings
(`sltiu 6` at `0x08927968`), against Pulse's twenty-one. The first craft over is
stamped; the second, when one is the player and neither is finished, raises the
gap against the first, as on Pulse.

## Wired

`oag_pure::hud::ART.lap_hud` (`oag_title::lap_hud`): the three keys,
`MessageTint::White` (so `oag_hud::messages::MessageLine::fades_border` is
false), `GapText::Phrases { ahead, behind }`, `table_crossings: 6`,
`green_hold_needs_first_place: false`. Disc-backed:
`crates/game/tests/lap_messages_ground_truth.rs`
(`pure_raises_its_own_keys_white_and_words_the_gap_as_a_sentence`), checked on
`01_Vineta_K` in a Time Trial (the three lines) and a single race ("YOU ARE 2.0
SECS BEHIND", red, no icon).

## Open

- Whether the outline really stays at full alpha while the text fades (a live
  PPSSPP frame would tell).
- The setters of the five unraised bits, if any exist outside the race path
  (Zone, Eliminator).
- The lap function's prologue and a name for it.
