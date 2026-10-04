# HD's `.xfx`: the per-team engine crossfade table

**Container confidence: 92. Per-tick drive law: 60-70 for three of four channels, below 40 for the fourth term. Not wired into the game, on purpose.**

Wipeout HD / Fury drives a craft's engine sound from `data/sound/xfship_<team>.xfx`,
one file per team: **13 files** (`DATA01.PSARC` holds twelve, `DATA00.PSARC` holds
`xfship_det.xfx`; an earlier note said twelve). The magic is `XFDX`. It is the
only engine-sound mechanism in the build: `shiphd.bnk` has no `~ENGINE` cue.
Implemented in `oag_formats::xfx`; the every-file test is
[`xfx_ground_truth.rs`](../../crates/formats/tests/xfx_ground_truth.rs).

The game's own vocabulary, from the error strings in its loader
(`XFadeSystem::AddCrossFader: ... maxElementsPerHandle ... maxControllerPerHandle`,
`0x007a46c8`/`0x007a4740`): a **controller** is an input and an **element** is a
playing sound. This project's reader says *channel* and *layer*.

## Layout (confidence 92)

All big-endian. The loader is `XFadeSystem_AddCrossFader` at `0x00312660`; the
header checks below are its three, in its order.

| Offset | Field | Evidence |
| --- | --- | --- |
| `0x00` | `XFDX` | loader: `*param_2 == 0x58464458`, else "Not a crossfader file" |
| `0x04` | `0x02060000` | low 24 bits `0x060000` = file version (else "Incorrect Crossfader file version"), high byte `2` = target platform (else "Incorrect target platform") |
| `0x08` | relocation flag, `0` on disc | loader fixes offsets to pointers when it is `0`, then sets it to `1` |
| `0x0c` | channel count (4 on all 13) | compared against the system's `maxControllerPerHandle` |
| `0x10` | layer count (9; feisar 8) | compared against `maxElementsPerHandle` |
| `0x14` | offset of the channel array, `0x1c` | |
| `0x18` | offset of the layer pointer table | |

Then, in file order: `channels * 0x60` bytes of channels; the triggers of
channel 0 (`0x40` each, the only channel that has any); `layers * 0x830` bytes of
layers; `layers * 4` bytes of layer offsets, which end the file.

**Coverage: the reader accounts for all 13 files to the byte**
(`all_thirteen_tables_parse_and_account_for_every_byte`). Goteki is
`0x1c + 4*0x60 + 26*0x40 + 9*0x830 + 9*4 = 20,976`.
**Feisar's 2,100 bytes smaller is one layer (`0x830` = 2,096) plus its 4-byte
pointer**, and nothing else: it has one `~n7` layer where the other teams carry
two `~n` layers (`feisar_is_one_layer_smaller_...`).

### A layer, `0x830` bytes

| Offset | Field |
| --- | --- |
| `+0x00` | kind byte; `0` on every layer (the game accepts `0..=2`) |
| `+0x01` | the sound's name, NUL-terminated within 15 bytes: `~jet03 03`, `~ABResLoL`, `~afterburner` |
| `+0x14` | the channel this layer reads |
| `+0x16` | `1` except the `~n..` layers of ag_systems, assegai and egx, where it is `0`; tested by the layer update, meaning unread |
| `+0x17` | type; `0` (two curves) on every layer. Type `1` is a piecewise-linear pair (`+0x28`, `+0x2c`) the loader reads and no file uses, so the reader refuses it |
| `+0x18` | flag word; `0x1000` on all but qirex's `~ABResHiL`/`~ABResHiR` |
| `+0x1c` | offset of the gain curve, 512 `i16` |
| `+0x20` | offset of the pitch curve, 512 `i16` |

The curves sit at `+0x30` and `+0x430` of their own record on all 13 files.
Gain runs `0..=0x400` on every layer of every file (`0x400` is unity, the value
the layer slots are initialised to); the pitch curve runs `0..=0x400`, and the
layer update subtracts `0x200` from it, so `0x200` is neutral.

The jets crossfade: on goteki `~jet03 03` falls `1024 -> 0` across the channel's
range while `~jet05 04` rises `0 -> 1024`, with the two `~n8` layers as
shaped-noise fills. Which layers sit on which channel
(`the_channel_assignment_is_jet_then_two_resonances_then_the_afterburner`):

| Channel | Layers on all teams |
| --- | --- |
| 0 | two `~jet..` crossfaded, then two `~n..` fills (one on feisar) |
| 1 | `~ABResLoL`, `~ABResHiL` |
| 2 | `~ABResLoR`, `~ABResHiR` |
| 3 | `~afterburner` (**on `det` it is on channel 0**) |

### Layer names are cues in `shiphd.bnk` (confidence 92)

All 13 files, every layer: `oag_formats::sblk::Bank::parse_as(.., Big)` on
`shiphd.bnk` and `sound_names()` contains it
(`every_layer_name_is_a_cue_in_the_ship_sound_bank`). So a layer plays a named
cue the project can already play; no raw sample ids.

### A channel, `0x60` bytes (confidence 70, static)

Read from the smoother, `XFadeSystem_UpdateChannels` (`0x00313d10`); no unit was
measured.

| Offset | Field |
| --- | --- |
| `+0x00` | four `u16` band edges (`450, 511, 511, 511` on channel 0; `511` x4 elsewhere). The current value, as `edge << 16`, picks the band |
| `+0x08` | four `i32` rates while rising, `16.16` per millisecond. Channel 0: `0x3333` (0.2/ms) in bands 0 and 1, `0` (snap) in bands 2 and 3 |
| `+0x18` | four `i32` rates while falling, same unit |
| `+0x50` | `i32` input scale, `0x10000` everywhere (`XFadeSystem_SetInput` stores `value * scale + (bias << 16)`) |
| `+0x54` | `u16` input bias, `0` everywhere |
| `+0x58` | `i16` trigger count: `26` on channel 0, `0` on the others, on all 13 files |
| `+0x5c` | offset of the triggers |

Channels 1-3 snap to a rate of 1.2-2.4 counts per millisecond; channel 3 rises at
2.0 and falls at 1.0. The fields at `+0x28..+0x50` hold band edges `511` and zero
periods and amplitudes: the smoother's random jitter (period at `+0x30`/`+0x38`,
amplitude at `+0x40`/`+0x48`) is off on every shipped channel.

**Channel 0's 26 triggers are dead data on all 13 files**: none has a name at
`+0x2b` and none a cue index at `+0x3c` (both tested by `XFadeSystem_Trigger`, so
it starts no voice), confidence 80
(`no_channel_zero_trigger_names_a_sound`).

## The per-tick law, partly recovered (confidence 60-70)

`FUN_000d5968` (`0x000d5968`) is the ship's per-tick audio update, called twice
from `FUN_000eadb8`. It writes one number per channel through `FUN_00314618`,
truncated to an integer and clamped to `0..=511`. TOC `0x008ad4d8`, read from
the function's own `.opd` entry (`0x008749c0`); every constant below is read
from that table, not from the decompile.

`speed_field = 3.6 * (*(ship + 0x6944))->[+0x4c4]`. The same product
`EngineFlare_Update` reads; [engine-trail.md](../ghidra/functions/ps3-hdfury-eu/engine-trail.md)
measured it as proportional to world speed (about 5.3 x), so it is speed-like and
not a raw thrust force. `ctrl = *(*(ship + 0x5fac) + 0x84)`.

| Channel | Stored at ship | Value |
| --- | --- | --- |
| 0 | `+0x5f20` | `0.5 * speed_field + 5.0 * X`, with `X = *(ship + 0x5fac)->[+0x260]` |
| 1 | `+0x5f2c` | `speed_field * 0.01 * ctrl[+8]`, only when `ctrl` is set; every role |
| 2 | `+0x5f30` | `speed_field * 0.01 * ctrl[+0xc]`, only when `ctrl` is set |
| 3 | `+0x5f24` | `5.12 * ctrl[+4]`, for the local player only (`ship+0x628c == 0`), skipped in four race modes |

**`X` is not identified.** The only float stores at `+0x260` on a non-stack base
belong to an XML tuning loader (a different class); `X` sits at a resting value
around 4 (the update arms a flag on `4.0 < X < 4.5` and fires channel-0 triggers
on bands at `2.5, 3.0, 3.5, 4.8, 5.5, 7.0, 10.0`), which resembles
`craft+0x344 = 4.12` that
[physics.md](../ghidra/functions/ps3-hdfury-eu/physics.md) records, but that is a
resemblance and not a finding. Channel 0 carries the main jet note, so the law is
not complete without it. See the handover thread for the measurement that would
close it.

**What the layer update does with the smoothed value** (`FUN_00314b00`,
`0x00314b00`, one call per instance per frame after the smoother):
`x = clamp((state + jitter) >> 16, 0, 511)`; `gain = A[x]`, `pitch = B[x]`;
voice volume `= A[x] * slot_gain_1 * slot_gain_2 >> 20` (slot gains default
`0x400`), voice pitch `= (B[x] - 0x200) * 0x7fff >> 9` plus two slot offsets,
clamped to `+-0x8000`. **The pitch unit is not measured** and so no playback
ratio is claimed here.

## What is not claimed

- The unit of the pitch value, and therefore whether the crossfade is audibly
  right when played.
- `X`, the fourth term of channel 0.
- Whether `ctrl[+4..+0xc]` are what their numbers suggest: `ctrl[+4]` is a
  0..100 quantity the code scales by 5.12 into 0..511, which is all that is read.

Wiring is blocked on the first and the second; see
[`xfade.md`](../ghidra/functions/ps3-hdfury-eu/xfade.md).
