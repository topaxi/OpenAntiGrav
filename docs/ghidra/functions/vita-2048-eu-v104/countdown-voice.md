# 2048's race start: two cue families, chosen by the mode id

`/2048/eboot-vita-2048-eu-v104.elf`, read statically (decompile and disassembly), 2026-10-19, lanes
`race-countdown-audio` and `countdown-audio-2`. The two start functions are read at branch level and stay
`FUN_`; the command handlers below are named. Nothing here was captured live.

| Address | Reading | Confidence |
| --- | --- | --- |
| `0x811743e4` | The race-state setter (the sibling of Pulse's `RaceMode_SetState`). State 1 starts the countdown | 80 |
| `0x81175d12` | The countdown's end: runs when the whole seconds left reach zero (`(int)max(0, +0x11b44 - +0x11d48) == 0`), the same test as Pulse's `RaceMode_UpdateCountdown` | 80 |
| `0x81000930` | `GameMode_IsHdLineage`, already on [ships-effects.md](ships-effects.md): mode id `< 0x17` | 85 |

## What plays

State 1 (`0x811743e4`), when not in the `FUN_810016e8` case (the id is `2`, or the mode object's vtable `+0x60` says so):

| Mode id | Cue | Bank handle | Also |
| --- | --- | --- | --- |
| `< 0x17` (a named mode: Arcade, Zone, Elimination, ...) | `"ready"` | `DAT_818c4df4`, the race's speech bank | `"HBEAT"` from the same bank |
| `>= 0x17` (a CRC id: **2048's own campaign and multiplayer events**) | `"321_NGP"` | `DAT_818c4df0` | `"RACE_Combat_Start"` from the speech bank when `FUN_8100193c` (ids 7, 8, 0x14, or the combat flags) holds |

The end (`0x81175d12`), same split: `< 0x17` plays the name at `0x814820b0`, which reads **`"go"`**, from the
speech bank; `>= 0x17` plays `"Go_NGP"` from `DAT_818c4df0`, then `"Ship_Gear_up_PirProto"` when `FUN_8100188e`
holds. Priority argument `2` on all four.

[ships-effects.md](ships-effects.md) shows every named mode is below `0x17` and that none of the 577
instance names in `SP.xml` and `MP.xml` is in the table, so **2048's own events take the `321_NGP` / `Go_NGP`
side**, and only a free-play named mode takes `ready` / `go`.

## What the banks author

Names are FNV-hashed (`Bank::is_hashed`), resolved with `oag_formats::sblk::cue::name_hash`
(`crates/formats/examples/bank_name_probe.rs`).

| Cue | Bank | Cue / commands | Body |
| --- | --- | --- | --- |
| `ready` | `speech.bnk` | 21, cmds 59..61 | two key-ons (a stereo pair), no delay |
| `go` | `speech.bnk` | 24, cmds 65..71 | a goto into its marker: the pair at cmds 68-69 |
| `321_GO` | `speech.bnk` | 27, cmds 75..90 | three stages on delays 125, 205, 205 ticks of the 240 Hz clock, each followed by a `0x1e` register write (`0x1fc`, `0x2fc`, `0x3fc`) and a final `0x1a` random wait. Nothing in the executable names `321_GO` |
| `321_NGP` | `Weapons_NGP.bnk` | 2, cmds 7..11 | opcode `0x04` (an LFO), key-ons at delays `0x96` and `5` (volume 80), then `0x29` with `0x384` |
| `Go_NGP` | `Weapons_NGP.bnk` | 44, cmds 356..370 | a `0x2e` switch and four marker arms (1, 4, 3, 2), each one key-on (volume 110) ending in a goto to marker `0x63` |
| `ready` / `go` / `321_GO` | `speech_det.bnk` | 0 / 2 / 1 | the Detonator build of the same trio |

## The opcodes `321_NGP` and `Go_NGP` use, read 2026-10-19

The dispatch table is at `0x8153b4e0` (one 32-bit pointer per opcode, entry `n` at `+4n`; `0x01` and
`0x09` share `FUN_8134e77e`, the tone-grain start, found by searching the executable for its
address). It has Pulse's and HD's layout, and every handler logs its own name when tracing is on,
which names them.

| Opcode | Address | Name | Confidence | Reading |
| --- | --- | --- | --- | --- |
| `0x04` | `0x8134ee78` | `Scream_OpLfo` | 85 | Logs `"LFO"`, then `FUN_81351f6a` configures one of four 0x30-byte modulator slots (`byte0 % 4`) from a 16-byte record: `byte1` is what it drives (`FUN_81351cd4`'s switch: `1` volume, `2` pan angle, `3` pitch, `4` another quantity, `5`/`6` another modulator), `byte3` the wave shape, the u16 at `+6` the amplitude, the word at `+0xc` the phase step (one cycle is `0x8000000`) |
| `0x23` | `0x8134fc16` | `Scream_OpMarker` | 85 | A no-op returning zero |
| `0x24` | `0x8134fc58` | `Scream_OpGoto` | 88 | Scans the cue's commands for a marker whose **low operand byte** equals its own (`*param_3`), refuses a ninth goto in one tick (`DAT_819d975c < 9`), no marker ends the cue. Pulse and HD compare operand byte 1 (the port's `marker_id`) |
| `0x29` | `0x813502fa` | `Scream_OpKeyOffVoices` | 88 | Keys off the handler's voices and its type-4 children (`FUN_81355dba`) |
| `0x2e` | `0x8134fe5c` | `Scream_OpGotoRandomMarker` | 88 | Logs `"Goto Random Marker Ex"`. Record `[first, last, mode, history]`: mode `0` picks `first + rand % (last - first + 1)` (`FUN_813b1410` is `mls`, a signed remainder), no history; modes `1` and `2` keep a ring of the last picks. Then the goto above |
| master tick | `0x81354b74` | `Scream_RunMasterTicks` | 82 | Accumulates the frames rendered (`DAT_819da2fc`) and runs one tick per `48000 / (240 * m)` frames, `m` = `DAT_8153bc9c`, initialised `1.0` in `FUN_8134af2c` (SCREAM 7.0.29) and written nowhere else; its other reader `FUN_8135718a` is the pitch scale. **240 Hz, read not run** |

**The cues, walked** (`WalkModel::vita`, `crates/formats/src/sblk/timeline.rs`):

- `Go_NGP` (`Weapons_NGP.bnk` cue 44): `0x2e` mode 0 over markers 1..4, each arm one non-looping key-on at
  tick 0 (4 waveforms of 33,520 to 48,448 samples), then a goto to marker
  `0x63`. **Four uniform arms of one waveform: the flat pick plays it exactly.** Confidence 85.
- `321_NGP` (cue 2): an `0x04` LFO (slot 0, drives **pan**, shape 1, amplitude `0x200` = 512, so a swing of
  `512 / 1024 * 360 = +-180` degrees, phase step `0x88888` = 240 ticks a cycle, **1.0 s**), two **looping**
  key-ons (15,936 samples each) at ticks 150 and 155 (0.625 s and 0.646 s) at descriptor angles 270 and 30
  degrees, volume 80, and `0x29` keying them off 900 ticks (3.75 s) later, at tick 1055 (4.40 s). An earlier
  note on this page said the pan swing was under a degree: that was a byte-order slip (`00 02` read as 2, not
  512). Confidence 80 for the swing, 85 for the rest.

## What the port plays (2048)

| Race | Before | Now |
| --- | --- | --- |
| a named mode (race box, free play) | nothing: `ready names no cue in SPEECH` (a hashed bank looked up plain) | `ready` (`speech.bnk` cue 21, a stereo pair, 0.89 s) at tick 91 and `go` (cue 24, a goto into its marker, a pair, 1.15 s) at tick 271, as timelines on the 240 Hz tick; `HBEAT` is not played (unread) |
| a campaign event (`load_event`) | nothing | `Go_NGP` at tick 271, one of its four arms; **no `ready`**; `321_NGP` is authored and **not played** (loader report: `sfx: 321_NGP not played: ...`) |

- **Why `321_NGP` is not played.** It needs two looping voices stopped 3.75 s later and a pan swept through the
  rear half at 1 Hz. The mixer has no key-off schedule, no pan modulation and the pan law has no rear half
  (`oag_audio::spatial::pan_of_angle`), and flattening it to one waveform would be a different sound. It also
  starts 0.625 s into a cue that lasts 4.40 s, against a `ready` to `go` spacing this lane lends from Pulse (180
  ticks, 3.0 s): the loop would outlive `go` by 1.4 s, so the spacing is probably not Pulse's here. **Open**:
  measure the start-to-go spacing on 2048 (Vita3K), add a key-off and a modulator to the mixer, then wire it.
- **The split** is `oag_title::CountdownEvent` on `oag_2048::race::COUNTDOWN_VOICE`; `load_event` calls
  `Banks::use_event_start`, which swaps the event pair in for the named one. Multiplayer events (also CRC ids
  in the original) are not distinguished: a 2048 multiplayer race plays the named pair.
- **Scoped to the countdown.** 2048's `zone_announcer.tick` stays `Unknown`, so no other 2048 cue changed; the
  loader report differs from before by exactly the `ready`, `go` and `cont_elim` lines (`cont_elim`'s trigger on
  2048 is unread, so it is not loaded on a hashed title).
- **Timing is Pulse's, chosen, not measured** (`origin: Chosen`): `ready` at tick 91, `go` at tick 271.
  `crates/raceplay/tests/vita_2048_countdown_ground_truth.rs` pins the two races.

## Omega

**Checked, differs, not wireable.** Omega's banks are Audiokinetic Wwise, not `SBlk`: `speech.bnk` is a 32-byte
`BKHD` header (version 118), its audio lives in `.wem` files, and the cue names are Wwise events. Its
executable carries `_321_NGP` and `Go_NGP` (and `HBEAT_ZCHANGE`, not `HBEAT`) as event names and no `ready` or
`go`. The race-state setter `FUN_012993b0` (state 1) posts `_321_NGP` through `FUN_0173ac30` for every mode but
the demo (`DAT_01f999e4 != 2`), plus `RACE_Combat_Start` in modes 8 and 0x14 or for combat flags, and the
countdown's end `FUN_0129a360` posts `Go_NGP`, then `Ship_Gear_up_PirProto`, on the same condition. **So Omega
has no `< 0x17` split in these two functions: it plays the NGP pair on every non-demo mode.** Nothing here
wires Wwise events to this project's cues (`docs/formats/wwise.md`), so it is not playable. Confidence 80
(decompile, no capture).
