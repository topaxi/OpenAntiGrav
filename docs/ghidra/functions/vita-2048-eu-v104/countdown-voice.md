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

## Which cue a name is: the `+0xc` field, not the bucket (2026-11-02, lane `countdown-voice`)

**Every table below used to name the wrong cues.** A v5 bank's name record is 16 bytes,
`{u32 name offset, u32 hash, u16 bucket, u16 next, u16 cue, u16}`. The lookup `FUN_813529f2` (called by
`Scream_FindCueByName`, which hashes with `FUN_81352b9c`) takes `bucket = hash % cue_count`
(`FUN_813b1400(hash, short at bank+0x16)`), checks the slot's u16 at `+8` equals the slot, walks the chain by
the u16 at `+10`, compares hash and string, and **returns the u16 at `+0xc`**. The reader returned the u16 at
`+8`, which is the bucket. Confidence 90: the decompile, plus two checks off the data.

- In `speech.bnk` the thirty names take thirty distinct `+0xc` values (a permutation of the thirty cues) and
  `+8` repeats (`ready`, `rockets` and `energycritical` all 21) and leaves eleven cues unnamed.
- `HBEAT`'s `+0xc` is cue 27, ten voices in three groups (3, 3, 4) on 125 / 205 / 205 ticks: **exactly HD's
  `HBEAT`** (`crates/raceplay/tests/countdown_voice_hd_ground_truth.rs`). The bucket gave cue 26, the
  Eliminator call.
- 33 hashed banks of the base archive, all names: `+8` and `+0xc` agree on a handful of names per bank, so
  the old reader was wrong for nearly every name it resolved, in every bank (the crossfade engine and the
  circuits' emitters included; see [2048-audio.md](../../../formats/2048-audio.md)).

## What the banks author (corrected)

| Cue | Bank | Cue index | Cue / commands | Body |
| --- | --- | --- | --- | --- |
| `ready` | `speech.bnk` | 0 | 1 command, a lone `0x08` | hands its voice to child cue 2, `321_GO`, as on HD |
| `321_GO` | `speech.bnk` | 2 | 16 commands | a goto over three stages (pairs of 1.62 s and 0.83 s on 125, 205, 205 ticks) to its marker, which keys a 3.00 s pair at 30 and 330 degrees |
| `go` | `speech.bnk` | 3 | 9 commands | a goto to its marker: a 5.14 s pair at 30 and 330 degrees (the cue holds three pairs of 2.72, 4.67 and 5.14 s) |
| `HBEAT` | `speech.bnk` | 27 | 15 commands | three groups of 3, 3 and 4 voices at ticks 125, 330 and 535 (1.75 s and 0.83 s clips), the last pair at 30 and 330 degrees |
| `321_NGP` | `Weapons_NGP.bnk` | 70 | 9 commands | three groups at ticks 125, 330 and 535: a 0.50 s voice at 0 and a 0.67 s pair at 90 and 270 degrees |
| `Go_NGP` | `Weapons_NGP.bnk` | 68 | 20 commands | one 0.66 s pair at 90 and 270 degrees keyed at tick 100, behind three `0x22` and gotos |
| `ready` / `go` / `321_GO` | `speech_det.bnk` | 0 / 2 / 1 | | the Detonator build; not re-read under the corrected field |

The walks are complete (no unread opcode, no unresolved child). Everything the earlier version of this
page said about `321_NGP` and `Go_NGP` (an `0x04` LFO, looping key-ons keyed off by `0x29`, four uniform
arms of one waveform) was read off cues 2 and 44 of `Weapons_NGP.bnk`, which are `PLASMA` and another
cue: **retracted**. The opcode handlers named below (`Scream_OpLfo` `0x8134ee78`, `Scream_OpKeyOffVoices`
`0x813502fa`, `Scream_OpGotoRandomMarker` `0x8134fe5c`, the 240 Hz master tick) are real and read
correctly; they just are not what these two cues use.

## What the port plays (2048)

| Race | Before the correction | Now |
| --- | --- | --- |
| a named mode (race box, free play) | `ready` the 0.87 s cue also named `rockets`, `HBEAT` the 0.94 s Eliminator call, `go` cue 24 | `ready` (cue 0 into `321_GO`: the 3.00 s pair at tick 91), `HBEAT` (the three beat groups from tick 91) and `go` (cue 3: the 5.14 s pair at tick 271), as timelines on the 240 Hz tick |
| a campaign event (`load_event`) | `Go_NGP` bucket cue 44 (four 1.8 to 2.7 s clips), no `ready` | **nothing**: `321_NGP` and `Go_NGP` are named in the loader report and not played |

- **Why the event pair is not played.** Both cues pan a pair to 90 and 270 degrees. `oag_audio::spatial::pan_of_angle`
  returns `None` for 90 (`a' = 180`, the first angle of the rear branch of `Scream_PanVolumePair`, which also
  phase-inverts a channel), so the timeline builder refuses the walk, and a flat pick would be a different
  sound. **Open, ready to wire**: model the `a' = 180` edge (hard right; the sign flip of a lone voice is
  inaudible) *scoped to these two cues*, or the whole rear half.
- **The split** is `oag_title::CountdownEvent` on `oag_2048::race::COUNTDOWN_VOICE`; `load_event` calls
  `Banks::use_event_start`, which swaps the event pair in for the named one. Multiplayer events (also CRC ids
  in the original) are not distinguished: a 2048 multiplayer race plays the named pair.
- **Timing is Pulse's, chosen, not measured** (`origin: Chosen`): `ready` at tick 91, `go` at tick 271. The
  corrected `ready` is a 3.00 s pair, which ends exactly on `go` 180 ticks (3.0 s) later; that agrees with
  the spacing but does not measure it.
- **Pinned by** `crates/raceplay/tests/vita_2048_countdown_ground_truth.rs` (`the_start_names_resolve_to_the_cues_the_original_returns`
  fails on the old reader: `ready` resolved to cue 21).

## Omega

**Checked, differs; nothing to play, 2026-11-02.** Omega's banks are Audiokinetic Wwise, not `SBlk`: `speech.bnk` is a
32-byte `BKHD` header (version 118), its audio lives in `.wem` files, and the cue names are Wwise events. Its
executable carries `_321_NGP` and `Go_NGP` (and `HBEAT_ZCHANGE`, not `HBEAT`) as event names and no `ready` or
`go`. The race-state setter `FUN_012993b0` (state 1) posts `_321_NGP` through `FUN_0173ac30` for every mode but
the demo (`DAT_01f999e4 != 2`), plus `RACE_Combat_Start` in modes 8 and 0x14 or for combat flags, and the
countdown's end `FUN_0129a360` posts `Go_NGP`, then `Ship_Gear_up_PirProto`, on the same condition. **So Omega
has no `< 0x17` split in these two functions: it plays the NGP pair on every non-demo mode.** Confidence 80
(decompile, no capture).

**New: no shipped bank defines the events.** The event id is Wwise's FNV-1 over the lower-cased name
(`oag_formats::wwise::name_hash`). None of `_321_NGP`, `321_NGP`, `Go_NGP`, `HBEAT_ZCHANGE` or `HBEAT` is an
event in any of the 53 banks of `data00.psarc` or of the patch's `data08.psarc` (the empty stub banks that
should hold them are 32-byte headers, [wwise.md](../../../formats/wwise.md)). Pinned by
`crates/raceplay/tests/omega_countdown_ground_truth.rs`. So the start cannot be played from the data we hold;
a later archive (or a patch layer) that defines them reopens it.

**What the port did before:** `oag_omega::race::COUNTDOWN_VOICE` lent Pulse's `ready` and `go` from `speech.bnk`;
that bank is the Wwise stub the `SBlk` reader refuses ("unsupported bank version 1145588546"), so the
load report read `sfx: ready not loaded` and the start was **silent** under a title field claiming a voice.
It is now `countdown_voice: None`. A maintainer report of "wrong cues" on Omega is therefore not from our start
voice (we play none there): unexplained, see the lane report.
