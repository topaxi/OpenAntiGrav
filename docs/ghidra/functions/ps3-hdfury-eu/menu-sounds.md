# Which sounds Wipeout HD / Fury's front end plays, and when

Wipeout HD / Fury PS3 EU (`/hdfury/EBOOT-ps3-hdfury-eu.elf`), 2026-10-08, lane `hd-menu-sfx`. Static reading through
the Ghidra bridge (decompile of every call site, no live run). Two names recovered:
`Sound_PlayBankCue` and `RedirectController_Update`; the rest stay `FUN_`.

Pulse's side is [menu-sounds.md](../psp-pulse-usa/menu-sounds.md). The shape is the same, the vocabulary is not: Pulse has
four navigation cues, HD has eight, one per direction, with a second spelling of accept and reject for the Fury style.

## `frontend.bnk` has a name table

A reviewer claimed HD's bank had none. It has: `SBlk` version 3, label `FRNTEND`, **27 named cues, 113 waveforms**
(`crates/game/examples/hd_frontend_bnk_probe.rs` over `hdfury-ps3-eu-dec.iso`; entry `Data\Sound\frontend.bnk`, 2,058,896
bytes). Only 2048's bank is nameless. Confidence 95.

| Cue | Waveforms | Played by the executable |
| --- | ---: | --- |
| `navUp`, `navDown`, `navLeft`, `navRight` | 16 each (bound) | the menus, below |
| `accept`, `accept_fury` | 2, 2 | confirm, below |
| `reject`, `reject_fury` | 30, 3 | back or refused, below |
| `TextBox01`..`TextBox04` | 16 each | `HorizMenu`/`List` text boxes (strings seen at 5 sites; trigger not read) |
| `unlockcell`, `unlockscreen` | 3, 9 | campaign cell unlock (`0x00216bb8`), `ShowUnlocks_Screen` (`0x0024d010`) |
| `~podiumticker`, `podiumtickend` | 3, 4 | `EndRacePodium_Screen` (`0x0021f488`) |
| `wipe_med` | 6 | `FlyerSelection_Screen` (`0x002308a0`), from a different bank slot |
| `TextBox05`, `EOM`, `camera_shot`, `pause`, `wipe_sm`, `whooshUp/Down/Left/Right`, `~testText` | | **no call site**: not a string in the EBOOT, `DFENGINE.SPRX` or any front-end XML (see below) |

`whoosh*` is reached as a *child* of `nav*` (below), which is why no code spells it.

## The play call

`FUN_002ffa58` (`0x002ffa58`) is the front end's only sound call: `(manager, bank, name, 0x400, 0, 0, 0, 0)`. It builds a
request on the stack with the bank-pointer flag `0x20000000` and hands it to the trampoline `FUN_00679688`, which lands
in `Sound_PlayNamedCue` (`0x0062c500`, [sound.md](sound.md)). `0x400` is unity pitch and there is no emitter, so every
menu cue is dry, as on Pulse. Every navigation site passes `*DAT_00b6cc28` (the manager) and `*DAT_00b6cc14`: the sound
manager's boot-bank table `PTR_DAT_008b4b54 + 0xc4` is `0x00b6cc14`, and `0x00301338` stores `frontend.bnk`'s handle
there first ([sound-bank-loads.md](sound-bank-loads.md)). The team and circuit name voices use `0x00b6cc30` instead.
Confidence 85 for the bank, 75 for the call's name.

Pad buttons are read with `FUN_006770c8(pad, n)`, a just-pressed test: `0` up, `1` down, `2` and `3` left and right in
the grid controller, `5` confirm. Sites were found by scanning `.text` for `lwz rD, d(r2)` whose TOC slot holds a cue
string's address, **under each function's own TOC** (`0x008ad4d8`; the second TOC `0x008bd3c4` is the engine and has
none). Scanning under the entry point's TOC alone gives 242 sites in 124 functions, 138 of them false; the right count
is 104 sites in 35 functions (`scripts/ps3-toc.py` explains the defect).

## Which event plays which cue

All eight are `FUN_002ffa58` with the cue spelled in the function's TOC string pool.

| Event | Cue | Where | Confidence |
| --- | --- | --- | --- |
| Cursor moves up / down a vertical menu or list | `navUp` / `navDown` | `Block_Update` (`0x0018d588`, pad 0 / 1), `0x00192288`, `0x001a4a68`, `List_Update` (`0x001c03e0`; silent when its `+0xcc` flag (`param_1[0x33]`) is `1`), `0x002103b0` (VertMenu, once per entry stepped over), `0x00254348` (Team Selection), `0x002096f0` | 85 |
| Cursor or strip moves left / right | `navLeft` / `navRight` | `0x001b3e18` (HorizMenu, once per entry stepped over), `0x001ac168` (grid controller, all four directions), `0x00245d78` (ProgressSelection) | 85 |
| A row's value steps | `navLeft` / `navRight` | List value cycle `0x001bd8d0`/`0x001bda20`, slider `0x001f94c8`/`0x001f9608`/`0x001faf40`, playlist `0x001ecff8`/`0x001ed168`, music list `0x001d90a8`/`0x001d9310` | 85 |
| A step that cannot go further and does not wrap | `reject` | the same functions (`0x001bd8d0`: `+0x126` is the wrap flag; without it `reject` is played and nothing moves) | 85 |
| Confirm that went through | `accept_fury` in the Fury style, `accept` otherwise | `RedirectController_Update` (`0x001f5478`): the target resolved and is not locked; skipped when the widget's `+0x279` byte is set | 80 |
| Confirm on a locked target | `reject_fury` / `reject` | same function, `FUN_001cad10(target) == 0`; also skipped when `+0x279` is set | 80 |
| Secondary button (back) | `reject_fury` / `reject` | same function, unconditional on the `+0x1ac` button, does not read `+0x279` | 80 |
| Scrolling list confirm | `accept` / `accept_fury` | `0x001a4a68` (pad 5, by `FrontEnd_IsFuryStyle`), `0x001a5618` (`accept`), `0x001a5d00` (`reject`) | 75 |
| A screen's own confirm, back | `accept(_fury)`, `reject(_fury)` | Controls (`0x0021bb28`), MemoryStickWarning (`0x00236c70`), Tournament selection (`0x002593a0`), Track selection (`0x0025d558`) | 70 |
| Team selection changes team | the team-name voice (`TMTX_NAME`), then `accept_fury` in the Fury style only | `0x002502d8`; the circuit's is `0x00259e08` (`TRTX_NAME`) | 80 |
| A page transition by itself | **no sound** | `Redirect` plays through the button that caused it, as on Pulse | 70 |

`FrontEnd_IsFuryStyle` (`0x0015b620`) picks every `_fury` spelling at run time. The port stands in for it with the served
page's colour (`oag_game::boot::sprites::fury_style`); a boot picks one style for the whole session.

### `nav*`, `accept` and `reject` are timelines, not single sounds

`frontend.bnk`'s command list for `navUp` is `05 <child whooshUp>`, then `19 07 02 00`, then fourteen key-ons: a stereo
`whooshUp` plus **one of seven stereo ticks** (the `0x19` alternate group, count 7, two commands each), then a longer
stereo tail. `navLeft`/`navRight` are `whooshLeft`/`whooshRight` plus the same seven. `reject` is two such groups, 25
ticks apart (49 combinations); `accept` is a stereo pair; `reject_fury` is three stereo hits at 0, 15 and 20 ticks.

**HD's `0x19` keeps its count in the operand's high byte** (`19 07 02 00` is seven alternates of two), as
[psp-audio.md](../../../formats/psp-audio.md) records; the timeline walker read Pulse's low byte and so left every one
unread and the loader fell back to a flat pick of all 18 waveforms. `WalkModel::hd_alternates` reads HD's layout, and
`SequenceTick::alternates_in_high_byte` switches it on for **the front end's cues only**, so HD's race cues keep the
flat pick they were measured with. `whooshUp`/`whooshDown` key a voice at 180 degrees, the rear half the pan law does not
model: such a voice is played centred and unpanned, **chosen, not measured**. The original's no-repeat rule for a
`0x19` pick (HD's handler never repeats a pick twice in a row) is not applied by the port's menu generator.

## Class attribution

The 35 functions sit between constructors of the `*_Item.cpp` classes in link order (`scripts/ps3-toc.py map`), which
puts each in its class's file to the same standard [menu-blocks.md](menu-blocks.md) uses (a function before class B's
constructor and after class A's belongs to A, 70). `Block` (focus move), `List`, `VertMenu`, `HorizMenu`,
`GridController`, `Slider`, `RedirectController`, `Playlist`, `MusicList`, `Ticker`, `StatCounter`, `TagInput`, and the
`*_Screen` files for the screen-specific ones. Only `RedirectController_Update` (72) and `Sound_PlayBankCue` (75) are named
here; the others are left because the class is inferred from position and their role was read for their sounds only.

## The front-end XML names no sound

A census of every `.xml` entry in `DATA00`..`DATA06` for `navUp`, `whoosh`, `wipe_`, `camera_shot`, `TextBox05`,
`unlockcell`, `Sound` and `Cue`: no hit. The only sound-adjacent attributes are string ids (`FE_ACCEPT`, `FE_DECLINE`).
`DFENGINE.SPRX` has none of the cue names either. The triggers are code, as on Pulse.

## What the port plays

Pad and pointer, on the menus, the campaign screens, the pickers and EndRace: a move plays `nav<dir>` for the way it went
(a pointer hover plays up or down by the row it landed on, **chosen, not measured**), a value step plays `navLeft` or
`navRight`, a confirm `accept`, a back or a refused confirm `reject`, each in the served style. See
[menu_sfx_ground_truth.rs](../../../../crates/game/tests/menu_sfx_ground_truth.rs). Not played, recovered: `unlockcell`,
`unlockscreen`, `~podiumticker`/`podiumtickend`, `wipe_med`, the team and circuit voices, `TextBox01..04`; and the
non-wrapping edge `reject` (the port's rows always wrap). Not recovered: `whoosh*` as an event of its own, `pause`,
`wipe_sm`, `camera_shot`, `EOM`, `TextBox05`.

Voices: a press keys 6 to 12 voices (`navUp` is a stereo whoosh and a stereo tick and a stereo tail), against a pool of 32.
`pressing_every_menu_cue_twenty_times_leaves_no_voice_behind` presses every role 20 times, 30 ticks apart, and asserts
every one starts and the mixer is empty afterwards. A windowed `--no-audio` run never renders its mixer (`Audio::tick`
only renders into a dump), so there the pool fills after five presses and later cues are refused; with a device or the
dump backend it drains.

Levels: the HD walk peaks at 23,148 (HD style) and 27,750 (Fury) of 32,767, no clipped sample. Pulse's `ACCEPT` and
`DECLINE` clip; HD's do not.

## Omega

Omega's `eboot.bin` carries the same `navUp`/`navDown`/`accept_fury`/`reject` strings, and its `data00.psarc` ships a
Wwise `frontend.bnk` (`Data/audio/sound/English(US)/frontend.bnk`, with `frontend.txt`) whose events are
`navUp__frontend`, `navDown__frontend`, `navLeft__frontend`, `navRight__frontend`, `accept__frontend`,
`accept_fury__frontend`, `reject__frontend`, `reject_fury__frontend`, built from the same `FE_FX_text02_*` and
`accept_fury_01` waveforms. **Checked, applies, not wired**: the triggers are HD's (its front end is HD's), but the port
plays no Wwise event as an effect (`oag_formats::wwise` resolves events to media, [wwise.md](../../../formats/wwise.md), and
nothing wires one to a cue), so `SoundBanks::frontend` stays `None` there.
