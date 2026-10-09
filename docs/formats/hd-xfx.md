# HD's `.xfx`: the per-team engine crossfade table

**Container confidence: 92. Per-tick drive law: channels 0 and 3 measured live (85); channels 1 and 2 stayed 0 in every sample (60); channel 0's `X` term is the first queued hover probe's length, followed live to the mean probe clearance plus about 1.12 (62). Pitch unit: a SCREAM bend (82). Level: a layer's volume word reaches the speaker squared, `0x400` is unity, final gain `0.295 * level * (word / 1024)^2` on every engine voice (88, two boots, [Level](#level-measured-live-2026-10-05)); the per-craft distance factor is a fit (55). Wired into the game 2026-10-05 with `X` held at its grid value, see [`xfade.md`](../ghidra/functions/ps3-hdfury-eu/xfade.md#2026-10-05-lane-hd-engine-wire-what-x-is-the-pitch-unit-and-the-wiring).**

Wipeout HD / Fury drives a craft's engine sound from `data/sound/xfship_<team>.xfx`,
one file per team: **13 files** (`DATA01.PSARC` holds twelve, `DATA00.PSARC` holds
`xfship_det.xfx`; an earlier note said twelve). The magic is `XFDX`. It is the
only engine-sound mechanism in the build: `shiphd.bnk` has no `~ENGINE` cue.
Wipeout 2048's own 23 tables are the same record layout little-endian with five channels: [2048-xfx.md](2048-xfx.md).
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

**Coverage: the reader's pieces tile all 13 files with no hole and no overlap**
(`all_thirteen_tables_parse_and_account_for_every_byte`: the sizes sum to the
file length, and `Xfx::coverage()` places every piece by its offset and leaves
no gap). Goteki is
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

The curves sit at `+0x30` and `+0x430` of their own record on every layer of all
13 files (asserted by `Layer::curve_offsets`).
Gain runs `0..=0x400` on every layer of every file (`0x400` is unity: measured,
see [Level](#level-measured-live-2026-10-05)); the pitch curve runs `0..=0x400`, and the
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

## The per-tick law (confidence 85 for channels 0 and 3)

`FUN_000d5968` (`0x000d5968`) is the ship's per-tick audio update, called twice
from `FUN_000eadb8`. It writes one number per channel through `FUN_00314618`,
truncated to an integer and clamped to `0..=511`. TOC `0x008ad4d8`, read from
the function's own `.opd` entry (`0x008749c0`); every constant below is read
from that table, not from the decompile.

`speed_field = 3.6 * (*(ship + 0x6944))->[+0x4c4]`. The same product
`EngineFlare_Update` reads; [engine-trail.md](../ghidra/functions/ps3-hdfury-eu/engine-trail.md)
measured it as proportional to world speed (about 5.3 x), so it is speed-like and
not a raw thrust force. `ctrl = *(*(ship + 0x5fac) + 0x84)`. `*(ship + 0x5fac)` is
the `0x3a0`-byte body entry whose first word is `g_CraftVtable` (`0x008636e0`).

| Channel | Stored at ship | Value |
| --- | --- | --- |
| 0 | `+0x5f20` | `0.5 * speed_field + 5.0 * X`, with `X = (*(ship + 0x5fac))->[+0x260]` |
| 1 | `+0x5f2c` | `speed_field * 0.01 * ctrl[+8]`, only when `ctrl` is set; every role |
| 2 | `+0x5f30` | `speed_field * 0.01 * ctrl[+0xc]`, only when `ctrl` is set; every role |
| 3 | `+0x5f24` | `5.12 * ctrl[+4]`, for the local player only (`ship+0x628c == 0`), skipped in four race modes |

**Measured live, 2026-10-04**, RPCS3 under its GDB stub on the EU disc
(`scripts/rpcs3-hd-engine-xfade-probe.py`): a Campaign race, the player's craft
(the one with `craft+0x7a60 == 0`), thrust held from a standing start, then
steering and an `L1` hold, then coasting. Twelve pauses, each reading the stored
channels and the terms together:

| Stage | speed field | X | channel 0 stored | `0.5*sf + 5*X` | channel 3 stored | throttle (`ctrl[+4]`) |
| --- | --- | --- | --- | --- | --- | --- |
| grid | 0.2 | 2.173 | 10 | 11.0 | 0 | 0 |
| thrust-0 | 0.5 | 2.167 | 11 | 11.1 | 0 | 0 |
| thrust-1 | 263.1 | 2.911 | 144 | 146.1 | 511 | 100 |
| thrust-2 | 447.4 | 4.189 | 244 | 244.6 | 511 | 100 |
| thrust-3 | 360.2 | 3.660 | 198 | 198.4 | 511 | 100 |
| thrust-4 | 434.2 | 4.195 | 238 | 238.1 | 511 | 100 |
| thrust-5 | 581.5 | 2.908 | 305 | 305.3 | 511 | 100 |
| thrust-left-0 | 423.4 | 4.494 | 234 | 234.2 | 511 | 100 |
| thrust-left-1 | 431.8 | 4.099 | 236 | 236.4 | 511 | 100 |
| thrust-airbrake-0 | 351.1 | 3.554 | 193 | 193.3 | 511 | 100 |
| thrust-airbrake-1 | 428.9 | 4.119 | 235 | 235.1 | 511 | 100 |
| coast | 87.8 | 3.997 | 63 | 63.9 | 0 | 0 |

Channel 0 equals the truncated formula on 11 of 12 samples; the twelfth
(`thrust-1`, mid-acceleration) is 2.1 under it, which fits the pause landing
between the audio tick and the next physics step. Channel 3 reads
`511` exactly when the throttle reads `100` and `0` when it reads `0`:
`ctrl[+4]` is a throttle percentage, a digital `100`/`0` of the kind
[physics.md](../ghidra/functions/ps3-hdfury-eu/physics.md) records. **Channels 1 and 2 read `0` on all twelve samples**, including
while steering left and holding `L1`, and `ctrl[+8]`/`ctrl[+0xc]` read `0`
throughout; what they are is not measured (whether `L1` was the airbrake in this
config was not checked).

**`X` is the first queued probe's length (62).** `body[+0x260]` is the float
slot of the first entry in the body's eight-slot probe queue, pushed by
`Physics_QueueRayProbe_q` / `Physics_QueueSegmentProbe_q`. Live it sat at about
2.2 on the grid and 2.9-4.5 while driving, and it equals the mean of the four
probe clearances `body[+0x354]` plus 1.1246 on the first three samples and plus
1.12-1.26 after (it moves with the craft's up vector). The update arms a flag on
`4.0 < X < 4.5`. The writer, the slot layout and why the first search missed it
are in [`xfade.md`](../ghidra/functions/ps3-hdfury-eu/xfade.md). **This
simulation does not keep its probe clearances on the ship state**, so the port
holds `X` at 2.164 (chosen, not measured); it moves channel 0 by at most about 12
of 511.

**What the layer update does with the smoothed value** (`FUN_00314b00`,
`0x00314b00`, one call per instance per frame after the smoother):
`x = clamp((state + jitter) >> 16, 0, 511)`; `gain = A[x]`, `pitch = B[x]`;
voice volume `= A[x] * slot_gain_1 * slot_gain_2 >> 20` (slot gains default
`0x400`), voice pitch `= (B[x] - 0x200) * 0x7fff >> 9` plus two slot offsets,
clamped to `+-0x8000`. **That word is a SCREAM bend** (82, traced to
`Scream_SetVoiceBend` and `Scream_UpdateVoiceBend` in
[`xfade.md`](../ghidra/functions/ps3-hdfury-eu/xfade.md)): linear in semitones
over the cue descriptor's own bend range, `range * bend / 32768` below zero and
`/ 32767` above, so `0x200` is no bend. Which bend range each layer's cue carries
is read from `shiphd.bnk` at play time.

## Level, measured live 2026-10-05

**How.** RPCS3 on the EU disc, Fury campaign default walk (Feisar, `concept1`
hull, Talons Junction, eight craft), twice from cold boot. A GDB client paused the
emulator and read every `XFadeSystem` layer slot (instance array at
`*(*0x008b5064 + 0x18)`, eight instances of `0x10` bytes, slot array `0x80` per
layer), the SCREAM voice each slot holds (`Scream_GetVoice`, array
`*0x008c0100`, `0x18c` per voice) and the hardware slots that voice owns (table
`*0x008c0044`, `100` bytes each, bitmask at voice `+0x24`). 126 hardware voices
over five scans. Scripts: `scripts/rpcs3-hd-engine-audio-capture.py` (`--park`
runs a file under the one GDB connection the stub serves, `--probe` the fixed
scans).

**The chain, each link read off the same voice** (confidence 88 for all four):

| Stage | Law | Live check |
| --- | --- | --- |
| layer slot `+8` and voice `+0x86` | `curve[x] * slot[+4] * slot[+6] >> 20`, `slot[+6]` is `0x400` | `slot[+8] == voice[+0x86]` on every voice |
| hardware `level` (`+0x20`) | `((a*a/127) * (t*t/127) * 258) / 127` with `a = voice[+0xc]` (the cue's volume) and `t` the waveform's volume, integer divisions | `(70, 100) -> 6021`, `(110, 120) -> 21808`, `(70, 100)` jets and `(110, 120)` noise layers exact on all of them |
| hardware gain (`+0x28`, `+0x2c`) | `hypot(gL, gR) = K * level / 32766 * (voice[+0x86] / 1024)^2`, then a constant-power pan by the voice's azimuth (`+0x24`, degrees: `gL = cos((az + 90)/2)`) | `K = 0.2945` on the six player layers of the first boot, `0.295` on the same six of the second, `0.27-0.29` on 32 voices of one scan |
| pairs | every audible layer owns **two** hardware voices of the **same** waveform, `164` bytes apart in the data, at azimuth `+-30` (jets, afterburner) or `+-70..90` (noise layers) | sample words `0x3584d19c` and `0x3584d240` |

So **`0x400` is unity and the volume word is squared**: a layer at half volume is a
quarter as loud. Pulse's engine and this port's old HD path used the PSP's
`x^(1/1.7)`, which at half volume is two and a half times louder than this.
**Non-engine voices in the same scans carried `K` of 0.31, 0.41, 0.64 and 0.79**
(five cues, a handful of voices each), so `K` is not one number for the platform;
the engine's is the steady one.

**The per-craft factor, `slot[+4] / 1024`.** One word per instance, shared by its
layers. The player's is 1015-1018 (0.991-0.994). On the countdown grid the eight
read 1015, 994, 960, 897, 851, 789, 737, 676, which fit the eight craft's distances
from the camera (8.7, 34, 47, 70, 86, 109, 127, 148 units in this port's own spawn
and camera) with a straight line `1.0626 - 0.0027 d` inside 0.006 past 34 units. After the
start the opponents drive away and every opponent's word reads `0`, so their
voices are stopped (`slot[+0x40] == 0`). **The writer of the word was not found**
(`Ship_UpdateEngineCrossfade` does not write it), the fit assumes the original's
camera stood where this port's does, and the near craft reads below 1; confidence 55.
It is also not Pulse's engine law (a 50-unit cull): craft stay audible to about 390
units.

**What changed in the port.** The layer's voice gain is `ENGINE_BUS_RATIO * (curve
* distance_factor)^2` where `ENGINE_BUS_RATIO = 0.2945 / 0.6377 = 0.462` is the
engine's `K` over the nearest non-engine voice's, because this port's other cues
sit on the PSP-derived scale and the engine must sit right against them
(`audio/sfx/xfade.rs`). **Superseded 2026-10-05**: on a title with an authored mix the engine plays
on group 7 with ratio `1.0` and the group's own law carries the level; the ratio stays for a title
without one. The remainder of the gap to the original was **not the engine's** (section below). The mixer's pool grows to 128 when a race has an engine
(`Mixer::grow_pool`, `HD_VOICES`): the original keeps every layer resident (62
hardware voices of 128 in one scan) and a full grid at speed wants 33. Pulse and
Pure never call it and render byte-identical WAVs.

**Absolute level against the original, same state, same circuit** (RPCS3's own
audio dump, float, mono mean of the two channels; ours `--dump-audio`, music 0,
ambience on). The original's windows are one boot, the engine-only claims are not
separable because music and ambience cannot be turned off there.

| State | Original | Ours before | Ours after |
| --- | --- | --- | --- |
| eight craft on the grid, RMS | 0.05-0.07 (all sources) | 0.31 (peak 1.0) | 0.18 (peak 0.77); ambience alone 0.093, engines alone 0.159 |
| one goteki craft, 2 s rest then 10 s climb, RMS | not capturable alone | 0.243 (peak 1.0, clipped) | 0.072 (peak 0.33) |
| full-throttle climb, RMS | 0.18-0.19 (peak 0.7-0.9, wall hits 1.3-1.5) | 0.30-0.48 | 0.12-0.29 (peak 1.0 from wall hits) |

**The global gap, resolved 2026-10-05** (lane `hd-mix-level`): it was the authored mix, not a
slider default. See "The authored mix" below. The engine's `0.462` above is exactly
`0.68^2 = 0.4624`, the player's `user7` group squared against an ordinary voice at group `1.0`.

## The authored mix, measured live 2026-10-05

**Where it comes from.** `Data\sound\GlobalAudioConfig.xml` (`DATA00.PSARC`, and a second copy in
`DATA01.PSARC`; the emulator held the `DATA00` one) carries `ParameterMaps`: a `GroupVolumes` row
(`music`, `user1` to `user12`) per game state and per channel layout, loaded by `AudioConfig_Load` into
`g_sound_system + 0x6a0 + 0xb8 * state`. The layout byte read `2` on RPCS3, the Stereo branch. A
live array (`+0x308`, thirteen floats) chases the active state's row. Code and addresses:
[`audio-mix.md`](../ghidra/functions/ps3-hdfury-eu/audio-mix.md). Rows (Stereo):

| State | music | user1 | user2 | user3 | user4 | user5 | user6 | user7 | user8 | user9 | user10 | user11 | user12 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| FrontEnd | 0.30 | 1.0 | 1.0 | 1.0 | 1.0 | 1.0 | 1.0 | 0.5 | 0.3 | 0.5 | 0.5 | 0.5 | 0.5 |
| PreRace | 0 | 1.2 | 1.0 | 0.8 | 0.6 | 0.6 | 0.5 | 0.4 | 1.0 | 1.0 | 0.8 | 1.0 | 1.0 |
| Countdown | 0 | 1.2 | 1.0 | 0.8 | 0.6 | 0.6 | 0.5 | 0.5 | **0** | 1.0 | 0 | 1.0 | 1.0 |
| RaceNormal | 0.65 | 1.1 | 0.9 | 1.15 | 0.9 | 0.8 | 0.8 | 0.68 | 0.8 | 0.5 | 0 | 0.8 | 0.5 |

(The other six states are in the file; the loader reads all eight the port models.) The authored
`USER1..12` comment lists the intent: 1 front end and speech, 2 HUD, 3 weapons, 4 pads and turbo, 5
explosions, 6 collisions, 7 ship sounds (engine, airbrake), 8 environment, 9 reserved, 10 pre-race and
radio chat and ship idle, 11 a voice, 12 unlabelled. It is a comment in the `DATA00` copy only.

**The sliders.** The options screen's own XML (`data/plugins/frontend/gui/additional_definition.xml`
and `ingame_definition.xml`) declares `Slider "Music Volume" ... default="80%"`, `Slider "SFX Volume"
... default="80%"` and `List "Auto Volume" ... default="FE_ON"`. The apply step reads each as `0..100`,
multiplies by `0.01`. Live, the profile read `0.8` for both (`cfg + 0x1c`, the SFX object's `+0xe0`),
so **the earlier capture ran on the defaults** and 80 % alone is `x0.8`, not the factor of 3.

**The law, each half isolated by writing the target rows over GDB** (all ten rows set to one
non-zero group, the live array read back each window, the audio dump read in the window; null sink,
verified):

| Test | Result | Reading |
| --- | --- | --- |
| every group `0` | RMS `0.0000` | everything is group-gated |
| music group `x1, x0.5, x1, x0.25` | stereo RMS `0.119, 0.057, 0.129, 0.033` | music is **linear** in the group |
| music slider `0.8 -> 0.4`, group full | `0.065` against `0.12-0.14` | music is **linear** in the slider |
| user7 group `x1, x0.5, x1, x0.25` (live `0.68, 0.34, 0.68, 0.17`) | player's per-voice `K = 0.295, 0.074, 0.295, 0.018`; RMS `0.043, 0.0105, 0.050, 0.0082` | effects are **squared** in the group: `K = (slider * group)^2`, `(0.8 * 0.68)^2 = 0.296` |
| user8 group `x1, x0.5, x1` | RMS `0.052, 0.017, 0.072` | the same, noisier |

So **music** is `slider * group` and an **effects voice** is `(slider * group)^2 * level *
(v86/1024)^2`, per hardware voice, with `level/32766 = (a/127)^2 (t/127)^2` exactly. The ordinary voices
read earlier (`K` 0.64, 0.41, 0.31, 0.79) are `(0.8 * g)^2` for `g = 1.0, 0.8, 0.7, 1.1`: groups the
rows above carry. **Not additive in RMS** (the music-only window exceeds the all-groups one: the master
compressor and the ducking templates are in the chain and are not modelled).

**Windows that must not be used.** A first set of isolation windows ran minutes after the start, with the
player being lapped and the race ending; they read the engine at `0.124` and then `0.043`, and ambience
at `0.05` then `0.00` (no source left). The numbers above are from a fresh race, 10 to 100 s after the
start, with the other craft's `slot+4` words read at each window.

**What the port does** (`crates/sound/src/hd_mix.rs`, `Bus::Group(n)` in `oag_audio`): the Stereo
rows are parsed (no hand-copied table), a live array chases the grid (`Countdown`) or race
(`RaceNormal`) row at `0.025` per 256-sample frame (**chosen**: one reading), the music bus is
`0.8 * slider * group0` and group bus `n` is `(0.8 * slider * group_n)^2 / 2`. The division by two is the
port's own unity: its `pan_volume_gain` is `2 a^2 t^2`, HD's hardware gain `K a^2 t^2`. The port's own
`100 %` setting is the original's `80 %` default (**chosen**). The engine plays on group 7 and the
circuit's emitters on group 8; **every other cue plays on the effects bus at group `1.0`** (**chosen,
not measured**, the cue-to-group assignment is not traced). Pulse and Pure never read the file and keep
their three buses: their `--race --hold cross --ticks 900` dumps are byte-identical (sha256
`3b1f9698...` and `71c9bd51...`).

**Per bus, the original against ours, Talons Junction, Feisar, the player alone, mono mean of the two
channels (stereo RMS in brackets)**:

| Bus | Original | Ours before | Ours after |
| --- | --- | --- | --- |
| front-end music (`frontend1_stereo.mp3`), 20 s of main menu | 0.070 (0.072) | 0.19 stereo | 0.070 (0.13) |
| race music, group `0.65`, slider `0.8` | 0.066-0.108 (0.119-0.138), a track the port does not know | not separable | 0.098 (0.110), a different track |
| engine, user7 at `0.68` | 0.037-0.044 (0.043-0.050) | not separable | 0.021 (0.021) |
| circuit ambience, user8 at `0.8` | 0.042-0.051 (0.052-0.072) | not separable | 0.022 (0.023) |
| the grid, user8 at `0` | silent | `0.093` ambience | `0` |

Reading: **music matches** where the track is the same (mono mean `0.070` against `0.070`). The
engine and ambience read **half** of the original (0.47 to 0.57). The candidates, none confirmed:
the **second hardware voice of each pair** (two voices of one waveform at azimuth `+-30`; in phase
they sum to `1.37 K` on the mono mean against one centred voice's `0.71 K`, ratio `1.93`, but if the
two start offsets 164 bytes apart decorrelate them the ratio is about `1.37`, which does not reach the
observed 1.8-2.3), the **ambience distance law** (the port's emitter law against the original's), and
the engine's own state (`X`, held). The mixer's pan law was checked: `pan_gains(0)` is `0.707, 0.707`,
a hypot of 1 like the original's constant-power pan, so it is **not** the residual. "The factor of 3"
is closed; a remaining factor of about 2 on these two buses is open. The front end's stereo RMS is
`0.13` in ours and `0.072` in the original at the same mono level: the original's output has almost no
side energy and ours has a lot. Not investigated.

**Ordinary effects** (collisions, pads, weapons) were **not exercised** in the idle windows on either
side; the evidence is the per-voice `K` of the first boot's scan (`0.6377 = 0.8^2`, group `1.0`) and the
port plays them on the effects bus at group `1.0`.

**Confidence** (rubric: [`confidence-rubric.md`](../reverse-engineering/confidence-rubric.md)):

| Claim | Score | Boots |
| --- | --- | --- |
| `GlobalAudioConfig.xml` rows are the live rows (eight states read back, Stereo branch, `DATA00` copy) | 90 | 2 (and the disc test) |
| both slider defaults 80 %, profile at the defaults | 88 | 2 (options XML and live) |
| effects voice `K = (slider * group)^2` | 85 | 2 (cap1 and cap2, three groups each) |
| music `slider * group`, linear in both | 72 | 1 (cap2 only) |
| front-end music matches at the mono mean | 70 | 2 (this lane and the previous one's capture, same track) |
| engine on group 7, emitters on group 8 | 75 | 2 (group 7 read off the player's voices; group 8 the only audible one with the rest zero) |
| the per-state glide `0.025` per frame | 35 | 1 reading |
| the remaining x2 on engine and ambience | 0 (unexplained) | 2 |

**Not modelled**: the `MasterCompressor` (ratio 0.2, threshold -6 dB), `MasterEQ`, the ducking
templates, `Auto Volume` (default on; its flag's readers are persistence accessors, not traced), the
`PreRace`, critical-energy and player-dead rows, and the transition speeds the XML carries.

## What is not claimed

- The engine's absolute level against an engine-only original: not separable, see
  above.
- The second voice of each pair: the port plays one voice per layer, so a layer is
  3 dB down in power against two uncorrelated voices and mono instead of spread.
  Which of a cue's five or six waveforms the original picks is also unread.
- `X` as a live term in the port: it is held at its grid value.
- Whether `ctrl[+4..+0xc]` are what their numbers suggest: `ctrl[+4]` is a
  0..100 quantity the code scales by 5.12 into 0..511, which is all that is read.

The wiring and what it chooses are in
[`xfade.md`](../ghidra/functions/ps3-hdfury-eu/xfade.md); the code is
`crates/sound/src/sfx/xfade.rs`.
