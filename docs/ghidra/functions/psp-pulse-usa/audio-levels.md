# Audio levels: the volume chain from a group to the DAC

What scales a sound between the mixer and the speaker, and what it is set to.
The companion to [sound.md](sound.md), which covers how a cue reaches a SAS
voice at all; this page is only about **level**, and it exists because the port
needed to know whether the original reserves any headroom for eight craft
sounding at once. It does not.

Addresses are `pulse-psp-usa` `BOOT.BIN`, image base `0x08804000`. The
decompiler renders call targets and globals in this region as image-base
offsets (`func_0x00272cf4` is `0x08a76cf4`), so both forms appear below.

## The chain

```text
SCREAM voices --> group volume [0x10 groups] --> master volume --> master << 5
                                                                       |
                                            sceAudioOutputPannedBlocking(ch, v, v, buf)
```

| Address | Name | Conf | What it is |
| --- | --- | ---: | --- |
| `0x089956d8` | `Audio_SetGroupVolume` | 85 | Sets one group's volume, or the master |
| `0x089906e4` | `Audio_Init` | 82 | Opens the output and sets every volume to `0x400` |
| `0x089909d0` | `Audio_Shutdown` | 82 | Takes the master to `0` and tears the thread down |
| `0x0898ca54` | `Audio_OutputThread` | 82 | Double-buffers the mix and blocks on the DAC |
| `0x08a2b590` | `Audio_OutputPannedBlocking` | 88 | The library wrapper on the `sceAudio` import |
| `0x08a2ac90` | `Sas_Init` | 88 | `sceSasInit` at 44,100 Hz, 32 voices, and each voice's opening state |

### `Audio_SetGroupVolume` (`0x089956d8`), confidence 85

```c
void Audio_SetGroupVolume(uint group, int volume)
{
  if (0x400 < volume) volume = 0x400;
  if (volume < 0)     volume = 0;
  if (group == 0xf) { func_0x00192708(0x53, 0, 0, 0, 0); return; }
  *(int *)(group * 4 + 0x2bf910) = volume;
  ...
  if (group != 0x10) { func_0x00191884(1 << (group & 0x1f)); }
  else               { _DAT_002bfa10 = volume; }
}
```

**`0x400` is unity and the ceiling.** Sixteen group volumes live in the table at
`0x2bf910`; index `0x10` is the master and lands in the gp-relative global the
output thread reads. Index `0xf` is special-cased into something else entirely
and is not a volume - noted, not chased.

The master global is written as `_DAT_002bfa10` and read back as
`lw a1, -0x5f0(fp)` at `0x0898ce14`, so it is gp-relative. Its absolute address
is not confirmed and it is deliberately **not** in `names.tsv`.

### `Audio_Init` (`0x089906e4`), confidence 82 - every volume opens at `0x400`

```c
iVar2 = 0;
do {
  Audio_SetGroupVolume(iVar2, 0x400);
  iVar2 = iVar2 + 1;
} while (iVar2 < 0xf);
Audio_SetGroupVolume(0x10, 0x400);
```

Fifteen groups and the master, all at the ceiling the setter clamps to.
`Audio_Shutdown` (`0x089909d0`) closes with `Audio_SetGroupVolume(0x10, 0)`,
which is the second, independent confirmation that `0x10` is the master and
`0x400`..`0` is its range.

**Both names are corroborated by their one call site each**, which is what
takes them past "a function that happens to set volumes":
`Audio_Init` is reached only from `FUN_0893ab54`, itself called only from the
sound manager's constructor `FUN_08939fe0` - the same constructor that writes
`mgr+0x168 = 1.7`, the volume-curve gamma
[positional-audio.md](positional-audio.md) already measured. `Audio_Shutdown`
is reached only from `FUN_0893a1c8`, the matching destructor, which unregisters
the two callbacks (`0x08804000 + 0x136ac4`, `+ 0x136af0`) the constructor
registered. Construction and destruction of the sound manager, either end.

### `Audio_OutputThread` (`0x0898ca54`), confidence 82 - `master << 5`

```c
iVar4 = _DAT_002bfa10 << 5;
...
iVar4 = Audio_OutputPannedBlocking(_DAT_002bf220, iVar4, iVar4, buffer);
```

`0x400 << 5` is `0x8000` - `PSP_AUDIO_VOLUME_MAX`, the largest value
`Audio_OutputPannedBlocking` (`0x08a2b590`) will pass on: it returns
`0x8044000a` for anything above `0x8000` before it reaches the import. Both
channels get the same number, so the master is not a pan.

The alternative branch, taken when `_DAT_002bf234` is set, is
`sceAudioOutput2OutputBlocking(iVar4, buffer)` - the same value into the other
output API.

## What this settles

**The original's level chain is unity end to end.** Nothing between a SCREAM
voice and the DAC attenuates by default: every group opens at the ceiling, the
master opens at the ceiling, and the ceiling maps exactly onto the hardware's
own maximum. The mix lands in a 16-bit buffer, so a sum past full scale
saturates there, in hardware, and that is the console's whole answer to
overload.

That is the evidence behind the clamp in
[`oag_audio::mixer::Mixer::render`](../../../../crates/audio/src/mixer.rs): the
port saturates because the console does, not because a limiter was wanted. It
is also why **no master attenuation was ported** - there is no measured number
to port, and inventing one would be exactly the plausible-looking stand-in
`CLAUDE.md` forbids.

Measured against this workspace on 2026-08-24, one 30-second capture of the EU
PSP disc. **Against a pinned config, not the machine's**, because every volume
row now scales what the dump holds and a settings file drifts:

```sh
mkdir -p /tmp/pin/oag && printf '[audio]\nmusic_volume = 100\nsfx_volume = 100\nspeech_volume = 100\nmaster_volume = 100\nmusic_source = "auto"\n' > /tmp/pin/oag/settings.toml
XDG_CONFIG_HOME=/tmp/pin cargo run --release -p oag-game -- data/images/pulse-psp-eu.chd \
  --race --mode single_race --autopilot --ticks 1800 \
  --screenshot /tmp/pin.png --dump-audio /tmp/pin.wav
```

Re-run with `music_volume = 0` and `sfx_volume = 0` for the other two rows:

| Mix | Samples at full scale |
| --- | ---: |
| Music alone | 2 of 2,880,000 |
| Race SFX alone | 11,956 (0.42 %) |
| Both | 47,071 (1.63 %) |

The SFX bus overloads **on its own**, before the music is added, and it does so
where the pack is still together: eight craft inside `ExhaustFlare_Init`'s
50-unit engine radius is eight `~ENGINE` voices summing, each at up to unity
gain by [exhaust.md](exhaust.md)'s own volume law. Once the field spreads the
capture falls back to a single engine and stops clipping entirely.

## What is not recovered, and why it is the next thing to read

**The per-voice SAS volume.** `Sas_Init` (`0x08a2ac90`) opens all 32 voices at
`sceSasSetVolume(core, v, 0, 0, 0, 0)` - silence - alongside
`sceSasSetPitch(core, v, 0x1000)` and `sceSasSetSimpleADSR(core, v, 0xf,
0x5fc0)`. This page originally read that as the **only** call to
`__sceSasSetVolume` (`0x08a76c24`) anywhere in the binary - **wrong**, corrected
2026-09-06: [sound.md](sound.md#correction-2026-09-06-the-guess-was-backwards-and-0x40-is-the-per-voice-volume)
found a second, per-voice, per-frame call through `Sas_CommitVoices`'s bit
`0x0f` dispatch, previously misread as an argument-count guess at "four for
ADSR". So a sounding voice's level *is* written every frame through
`__sceSasSetVolume`, from the voice record's `+0x40`/`+0x44` (dry L/R) and
`+0x48`/`+0x4c` (effect-send L/R) - that part of "something else" is now
found.

**What is still not recovered is what value lands in those four fields.**
Nothing read yet traces `+0x40` back to a writer: `sound.md`'s correction
names `SoundInstance_UpdateSpatial` (`0x08939e58`, this page's own
`positional-audio.md` cross-reference) as the only known producer of a
comparable volume pair, but it writes into a SCREAM software instance through
its own fade/ramp engine (`func_0x0898d614`), not into the SAS voice record
`Sas_CommitVoices` walks - and nothing read connects the two. **That link is
the next thing to read**, not the four wrappers, which are now resolved.

Until that lands, "our race mix is louder than the console's" is a hypothesis
with no measurement behind it, and the honest port is the one that reproduces
the recovered law and saturates where the hardware saturates.

**Recovered, live, 2026-09-06 - see the next section.** The section above is
kept as written because it is what motivated the capture; the value itself
is below.

## The per-voice SAS volume, measured live (2026-09-06)

A running `pulse-psp-usa` (PPSSPP v1.20.4 under Xvfb, disc booted from the
cached ISO, [`ppsspp-debugger.md`](../../../reverse-engineering/ppsspp-debugger.md)'s
recipe) was driven into a **Single Race** (`psp-drive.py menu --single-race`,
the only reachable race type with a full eight-craft grid), then broken on
`Sas_CommitVoices` (`0x0898c7e8`) for several hundred consecutive hits with
thrust held, reading the whole 32-record voice table (`g_sas_voices`,
`0x08b893d0`, `0x6c`-byte stride) in one `memory.read` per hit and decoding
`+0x40`/`+0x44`/`+0x48`/`+0x4c` as the four `int32` fields
`__sceSasSetVolume` takes past `(core, voice)` - dry L/R and effect-send L/R.

**Confirmed independently at the same time, from the running binary rather
than from Ghidra's decompilation, and twice over.** `memory.disasm` at
`0x08a2ae08` (the wrapper `Sas_CommitVoices` calls for bit `0x0f`, now named
`Sas_SetVolume`; the one `sound.md`'s correction could not get a decompile
for - "no function found") shows real code there, ending in
`jal zz___sceSasSetVolume` at `0x08a2ae50`, guarded by the same
`DAT_08b2d300`-style "is SAS initialised" check `Sas_SetVoice` uses. Ghidra's
own `get_function_by_address` still resolves `0x08a2ae08` to the *interior*
of an unrelated neighbouring function (`FUN_08a2ade0`, a generic
table-dispatch helper with nothing to do with SAS) - a Ghidra
function-boundary defect around this one address, not a wrong reading of
what the disc runs. Separately, `memory.disasm` at `Sas_CommitVoices`'s own
entry (`0x0898c7e8`) shows `jal z_un_08a2ae08` at `0x0898c868` with the
loop's record pointer loaded four instructions earlier from
**`li s1,0x08B893D0`** - a plain 32-bit immediate, read directly off the
executing binary, that is `g_sas_voices`'s documented address exactly. That
retires a mismatch an earlier draft of this page raised (the decompiled loop
shows this base as the bare literal `0xafc38`, which is neither this address
nor this address under the usual image-base-offset convention): the live
immediate is the ground truth, and `0xafc38` is a decompiler artifact on
this one literal, not evidence of a second, different table.

**The value varies by voice, and within a voice over time - it is not a
constant and not a stand-in.** Three captures (90, 200 and 300 hits, all
during the same single race, both stationary on the grid and moving) each
recorded every voice's four fields on every hit, whether or not that voice
committed that frame - so a slot's leftover value from a sound that finished
playing minutes earlier reads identically to a live one until the two are
told apart. **Every reading below is filtered to voice/tick pairs where the
dirty-flag word's `0x0f` bit was actually observed set at that hit** -
meaning `Sas_CommitVoices` really did call `Sas_SetVolume` for that voice on
that frame - which is what distinguishes a driven engine from a stale
record sitting in a freed slot. All three captures carry at least one such
stale slot (values that never change and whose dirty bit is never seen),
excluded below:

- **Different voices carry different, unrelated magnitudes at the same
  instant**: one 90-hit window held voice 0 committing at
  `(1125,1125,1125,1125)`, voice 1 at `(324,324,324,324)`, voice 4 at
  `(214,214,0,0)` and voice 8 at `(545,545,0,0)`, all in the same tick.
- **A newly-sounding voice ramps rather than jumps.** Voice 3 stepped
  `(0,0,0,0) -> (20,16,0,0) -> (40,33,0,0) -> (61,50,0,0)` across consecutive
  *committing* hits in one capture, and voices 6, 8 and 10 show the same
  smooth, many-step climb from zero elsewhere. That is a fade-in, and it is
  a live sighting of the missing link `sound.md` flagged: something reaching
  the SAS voice record gradually rather than in one write, consistent with
  `SoundInstance_UpdateSpatial`'s own software fade/ramp engine
  (`func_0x0898d614`) - though nothing here re-derives the exact function
  that bridges the two, so that specific connection is still not nailed
  down to an address.
- **Dry L and dry R usually differ, and by a consistent ratio while a voice
  holds one heading** - e.g. voice 5's committed readings climb from
  `(1080,770,0,0)` to `(1398,840,0,0)` across nineteen distinct samples with
  L consistently above R throughout. That is stereo pan, not noise: a
  craft's engine sound panned by its position relative to the listener,
  exactly as `positional-audio.md` already describes for the pre-SAS gain.
- **The effect-send pair (`fx_l`/`fx_r`) is usually `(0,0)`** on a genuinely
  committing voice, and only matches the dry pair exactly
  (`vol_l==fx_l`, `vol_r==fx_r`) on a handful of them - a different category
  of sound (plausibly one with reverb send enabled) from the dry-only
  majority.
- **The highest value seen among genuinely-committing voice/ticks across
  three independent captures (590 total commit-loop hits) is `3155`**, on
  voice 7 in one capture, at a tick where its dirty bit was observed firing -
  a real, driven value rather than a leftover. A commonly-cited figure for
  `sceSasSetVolume`'s own range is `0`-`0x1000` (4096), which would put this
  at 77 % - **background only, not part of this page's confidence score**:
  nothing read here derives that ceiling from this binary (`Sas_Init`'s own
  use of `0x1000` is for the *pitch* argument, a different parameter with
  its own scale, and the one bounds check found nearby, in the unrelated
  `FUN_08a2ade0`, rejects values above `0x8000`, not `0x1000`). Treat the
  4096 figure as unverified recall, not a measurement.

**A scale-free check that does not need that ceiling: how much do
simultaneously-committing voices sum to, against the loudest one of them
alone?** For each tick, summing dry L across every voice that committed at
least once somewhere in that capture (so a leftover stale slot is excluded
throughout, not just at the tick it happened to be read) and comparing
against that tick's own loudest single voice:

| Capture | Voices committing at once (max seen) | Sum | Loudest single voice | Sum / loudest |
| --- | ---: | ---: | ---: | ---: |
| 1 (90 hits) | 8 | 2305 | 1125 | 2.05x |
| 2 (200 hits) | 13 | 6639 | 1125 | 5.90x |
| 2, late in the same capture | 12 | 10084 | 3155 | 3.20x |
| 3 (300 hits) | 10 | 3483 | 614 | 5.67x |

Every one of these is measured well after the capture begins, not only at
the very first tick, so this is not an artefact of every voice
initialising at once when the race starts - a busy multi-craft race keeps
several voices committing together throughout. **A sum two to six times a
single voice's own value, sustained rather than momentary, is enough on its
own to push a mix that has no reserved headroom past full scale** - this is
the mechanism, read directly off the recovered per-voice values themselves,
with no assumed absolute ceiling required to interpret it.

**Confidence 90.** Runtime trace (per the
[confidence rubric](../../../reverse-engineering/confidence-rubric.md), capped
at 94 short of a second binary) against three independent captures that
agree with each other and with the call signature `sound.md` already derived
from the decompilation; not corroborated against `psp-pulse-eu`.

### Does this explain the 2.78 % clipping? No, and here is why

**The per-voice SAS volume is not itself the saturating value, and no
single voice is set wrong.** Nothing recorded repeats a clamped-at-maximum
plateau, and every committing voice ramps in smoothly rather than jumping
to a rail - the opposite of what a mis-set individual voice would look
like. What the samples show instead is **several independently reasonable
per-voice volumes summing to two-to-six times what any one of them carries
alone**, sustained across a capture rather than momentary - which is
exactly the mechanism this page's own earlier measurement already
established from the DAC/group-volume side ("the original's level chain is
unity end to end", "no headroom reserved"): eight craft's engines, each
honestly attenuated and panned on its own terms, overload the mix once
several sum, not because any one of them is set wrong.

So the number this page named as "the next thing to read" has been read,
and it **narrows rather than reopens** the earlier finding: the saturation
is not a bug in what any single voice is told to be, and no attenuation is
invented here either, per the same rule that applied before this
measurement existed.

## The original measured live (2026-09-07): it does not meaningfully clip

**The load-bearing result on this whole page: our music bus alone, on its
own, measures louder than the original's entire race mix - engines
included.** That single comparison is what actually settles whether the
summing-headroom theory above (several honestly-scaled voices adding past
full scale) explains our port's saturation, and it does not: adding
uncorrelated sound sources cannot reduce RMS, so no SFX-side mechanism, on
either side, can produce our music track alone outmeasuring the original's
whole mix. The problem is in the music path, not in how SFX voices sum.

### Method: PPSSPP's own audio dump, bracketed by PSP cycle-counter ticks

Everything above this section was read from the decompilation or from a
static memory layout. This section is the first time the original was run
live for its audio specifically: `PPSSPPSDL` v1.20.4 under its own Xvfb and
websocket port, `[General] DumpAudio = True` appended to its config, booting
`data/images/pulse-psp-usa.chd` directly (no `chdman` step needed on this
version). `DumpAudio` writes every mixed HLE audio buffer to a `.wav` under
the profile's `PSP/AUDIO/`, continuously from boot - not from when a race
starts, so a segment has to be sliced out.

**Two things cost real time to find here, recorded so nobody re-derives
them:**

- **The wav's timeline is 1:1 with emulated seconds
  (`cpu.status`'s `ticks / 222_000_000`), not wall-clock seconds.** This
  PPSSPP logged `Secondary instance 2 - silencing audio` on boot (another
  PPSSPP instance was detected elsewhere in the shared sandbox this was run
  in) and ran anywhere from 0.68x to 2.46x real time depending on phase -
  menus and loading ran far faster than real time, the race itself measured
  directly at 0.80x. A slice taken by wall-clock elapsed time would be off by
  a large, phase-dependent factor. The fix: read `cpu.status`'s `ticks`
  immediately before and after the window of interest, convert to a sample
  range with `sample = ticks / 222_000_000 * 44100`, and slice the wav on
  that. Cross-checked at the end of a session: final `ticks / 222e6` = 328.3 s
  against a final wav duration of 334.4 s (the process was killed about 6 s
  after the last tick read) - matches, confirming the mapping.
- **`sceSasSetVolume`'s ceiling claim in the previous section carried an
  unresolved absolute address for the group table.** Resolved here the same
  way `sound.md` resolves call targets in this region: this page's own
  image-base-offset convention (`func_0x00272cf4` is `0x08a76cf4`) applies to
  data too, so `Audio_SetGroupVolume`'s table at the decompiled offset
  `0x2bf910` is `0x08804000 + 0x002bf910 = 0x08ac3910` at runtime, named here
  `g_audio_group_volumes` (`0x08ac3910`, confidence 85), the sixteen-entry
  table `Audio_SetGroupVolume` indexes into. Read live with `memory.read`
  while sitting at the Main Menu (a fresh profile, `scripts/psp-drive.py`'s
  own first-boot walk): **all 15 usable group slots read exactly `1024`
  (`0x400`)** - the documented ceiling, now a genuine runtime measurement
  rather than the static decompile's own claim about what `Audio_Init`
  writes. Confidence **85**: one live read, one instant (idle at the Main
  Menu, not mid-race), one binary. The master (`_DAT_002bfa10`,
  group `0x10`) was not read - its absolute address is still unconfirmed (see
  above) and it is not needed for the finding below, since it would move both
  the original's own music and its own SFX equally and could not explain an
  anomaly that is specific to the music bus.

Driven into a race with `scripts/psp-drive.py`'s own `menu()` (imported as a
module, `--single-race` for the full eight-craft grid), then held `cross` for
a tick-bracketed window post-countdown, free-running - no breakpoints, so the
CPU is never stopped mid-race and the timing bracket above is exact.

### Measurement 1: the race mix, original versus ours

24.07 s window, mid-race, full throttle, full eight-craft grid:

| | Peak | RMS | Clipped samples |
| --- | --- | --- | --- |
| **Original** (`pulse-psp-usa`, live, PPSSPP) | 0 dBFS | -15.27 dBFS | 930 / 2,123,332 (**0.044 %**) |
| **Our port** (`pulse-psp-eu.chd`, `--race --autopilot`, 30 s, this page's earlier 2026-08-24/09-06 captures) | 0 dBFS | -7.52 dBFS | 80,164 / 2,880,000 (**2.783 %**) |

**Read this as "the original does not meaningfully clip," not "both clip,
ours more."** 930 samples in 24 s, each at exactly one rail, is the signature
of isolated transients (a collision, a pad, a pickup chime) touching full
scale - not a mix sitting on the rail the way our port's continuous 2.783 %
does.

**Cross-release caveat, stated plainly rather than left to be noticed:** the
original-side capture is `pulse-psp-usa`; the port-side number is
`pulse-psp-eu.chd`, from this page's earlier captures. There is no reason to
expect the two releases differ in mix level, but this has not been
corroborated, and it specifically weakens this clip-rate table. It does not
touch Measurement 2 below, which is entirely within our own port plus a raw
disc decode and is release-independent.

### Measurement 2: our music bus alone already outmeasures their whole mix

This is the comparison that actually carries the conclusion, and it needs no
further isolation of the original at all:

- **Our port, race context, `sfx_volume = 0`** in our own settings
  (`music_volume = 100`, `master_volume = 100`): peak 0 dBFS, **RMS
  -11.53 dBFS**, 2 of 2,880,000 clipped (0.000 %). That clip count matches
  this page's own "Music alone: 2 of 2,880,000" row above exactly, which is
  what confirms this new capture is calibrated against the earlier one
  rather than being a fresh, uncorroborated number.
- **Our music decode is faithful to the disc's own authored PCM.** The
  soundtrack track this race plays logs as "race music PSP soundtrack track
  0, 187.6 s as its own disc lists it, 187.7 s". An already-cached raw
  decode of it exists on this machine from earlier project work:
  `data/cache/audio/11ac097f4895d45e-2ch-44100hz.s16le`, 33,103,872 bytes,
  which at 2 channels x 2 bytes x 44,100 Hz is `187.66 s` - matching the
  logged duration to two decimal places (an inference from duration, not a
  hash match against the disc's own entry, though no other file under
  `data/cache/audio/` lands within a second of it). Its own RMS, computed
  directly off the raw samples with no mixer involved at all: **-11.37
  dBFS** - within 0.16 dB of our music-alone dump above. This directly
  confirms `crates/game/src/audio.rs`'s own documented claim that an
  unattenuated voice "round-trips the disc's own PCM sample-for-sample": our
  music path adds no gain of its own, and the disc's own track is simply
  mastered this loud.
- **Our music bus alone (-11.53 dBFS) is louder than the original's entire
  race mix, music and eight craft's engines together (-15.27 dBFS).** Adding
  uncorrelated sound sources cannot reduce RMS - so no amount of SFX-side
  voice-summing headroom, on either side, reconciles these two numbers.
  Something makes the *original's actual playback* of this same,
  faithfully-decoded music quieter during a race than the raw track is on
  its own, and it is not anything either this page or `sound.md`'s per-voice
  SAS measurement already covers - both were measured to be honest and
  unattenuated. It points at whatever varies a group's volume **after**
  `Audio_Init`'s one-time ceiling write, during an actual race - which
  nothing measured so far, on this page or elsewhere, actually reads mid-race
  rather than at rest.

**No attenuation is invented here either.** The gap is real and now narrowed
to "something scales a group after boot, during a race, that we do not
model" - but which group, by how much, and on what trigger is unmeasured. See
the next section for the specific, already-documented mechanism this points
at.

## One more thing on this page's own subject

`Audio_UpdateGroupVolumes` (`0x0893ac70`, confidence 82) is called once a frame
from the already-named `SoundManager_Update` (`0x0893a2b0`), with that
function's own `dt`, and writes nine of the sixteen groups - `0`, `2`, `3`, `4`, `5`, `0xa`,
`0xc`, `0xd`, `0xe` - from a fade scalar at `+0x90` scaled by `1024.0`, i.e.
straight onto the `0`..`0x400` range above. Group `0` alone is multiplied by a
second factor at `+0xcc` that ramps toward `1.0` at `+0xc8` per second and
retreats toward `+0xc4` when the flag at `+0xc0` is set - the shape of a duck,
almost certainly music under speech. The trigger is not recovered, so nothing
of it is ported.

**This mechanism now has a second, independent reason to be worth reading,
beyond a speech duck.** The live group-volume read in the section above found
all 15 groups at `1024` - but that read was taken idle, at the Main Menu, not
mid-race, and a `memory.read` while the CPU is running costs about 520 ms
(see [`ppsspp-debugger.md`](../../../reverse-engineering/ppsspp-debugger.md)),
which would have stalled the tick-bracketed timing that race capture needed.
So whether group `0` (or whichever group turns out to be music) reads below
`1024` while actually racing is still an open, direct measurement - and per
Measurement 2 above, something has to be attenuating the music bus during a
race for the original's numbers to make sense at all.

**Corrected below (2026-09-07): read mid-race, the duck's trigger is now
recovered, it is not speech, and none of these nine groups is music at all.**
Read on.

## Music does not go through this chain - it has its own volume, measured live (2026-09-07)

The prior section's "almost certainly music under speech" was a guess dressed
in plausible language, and this pass is what the confidence rubric means by
"below 50, do not rename, write the hypothesis down instead" done right the
first time. It should have stayed a hypothesis rather than "almost certainly."
Corrected now with the actual call chain, decompiled and cross-checked live.

**`get_xrefs_to` and `get_function_callers` are both empty for
`Audio_UpdateGroupVolumes`** - the relocation defect the rest of this project's
docs already warn about. `scripts/psp-relocate.py callers 0x0893ac70` finds the
one real call site instead: `0x0893a2e4`, inside `SoundManager_Update` itself,
matching what this page already said. The same tool is what unravelled
everything below; `get_xrefs_to`'s emptiness on every address in this section
is expected, not a dead end.

### The two options-menu sliders write to two entirely different structures

`FUN_08809bc8` (confidence 82, not renamed - see below) is reached from the
settings-apply path and reads exactly the two option strings
[ADR-0027](../../../../docs/architecture/adr/0027-three-mix-buses.md) already
named: `"Music Volume"` (`0x08a78658`) and `"SFX Volume"` (`0x08a78668`).
Decompiled:

```c
uVar1 = func_0x000046b8(param_1,0x274658);       // read "Music Volume", 0-100
*(undefined4 *)(_DAT_002bddd8 + 0x40) = uVar1;   // -> the music player, +0x40
*(undefined4 *)(_DAT_002bddd8 + 0x44) = uVar1;   // -> the music player, +0x44
iVar2 = func_0x000046b8(param_1,0x274668);       // read "SFX Volume", 0-100
func_0x00136938((float)iVar2 * 0.01,_DAT_002bde10);  // -> Audio_SetSfxFadeTarget(fraction, mgr)
*(float *)(_DAT_002bde10 + 0xd0) = (float)iVar2 * 0.01;  // belt-and-suspenders on the same field
```

**`_DAT_002bde10` is the sound-manager pointer
[positional-audio.md](positional-audio.md) already resolved** to
`0x08ac1e10` (dereference for `mgr`) - the *same* `mgr` `Audio_UpdateGroupVolumes`
takes as `param_2`. **`_DAT_002bddd8` is a second, previously undocumented
global pointer at `0x08ac1dd8`**, to a wholly different object this page did
not know existed until this pass: named `g_music_player_ptr` below.

So: **the SFX Volume slider is one of several things that drive
`Audio_UpdateGroupVolumes`'s fade** (`mgr+0x90` current, `mgr+0xd0` target,
`func_0x00136938` = `Audio_SetSfxFadeTarget` (`0x0893a938`), confidence 85 -
decompiles unambiguously to exactly those two writes: `mgr+0x90 = param_1`
always, `mgr+0xd0 = param_1` only when `param_1 > 0`). **Nine call sites**
(`scripts/psp-relocate.py callers 0x0893a938`, not the one this page first
assumed), spot-checked at two of them: the sound manager's own constructor
(`FUN_08939fe0`, an initial snap to full) and a reset/reinit step
(`FUN_0893aba4`) that reads five bank-name hashes and re-arms the duck
constants (`+0xc0=0`, `+0xc4≈0.6`, `+0xc8≈0.4`, `+0xcc=1.0` - matching the
live-read defaults below exactly) before re-snapping the fade. Neither
caller, nor the options-menu one this page already had, touches anything
music-shaped - all three read or reset SFX-side state. **What this actually
establishes, precisely**: the Music Volume slider does not write to
`Audio_SetGroupVolume`, `Audio_UpdateGroupVolumes`, `mgr`, or
`Audio_SetSfxFadeTarget` at all - it writes straight into `g_music_player_ptr`'s
own `+0x40`/`+0x44`, a `0`-`100` field on an entirely separate object, in the
same statement that handles the SFX slider. That is not the same claim as "no
SAS voice is ever assigned to one of these nine groups" - group membership for
a *voice* was not read this pass, only the *group's own volume knob*. If a
music voice ever does sit in a SAS group, the six groups
`Audio_UpdateGroupVolumes` never touches (`1, 6, 7, 8, 9, 0xb`) are where to
look next - though all sixteen read `1024` live below regardless, so it would
not change this page's conclusion either way.

### The music player, briefly

`0x08938428` (confidence 75, named `MusicPlayer_Init` below - a constructor:
publishes itself as `g_music_player_ptr`, then sets `+0x40`/`+0x44` to `100`
each - the same default `"Music Volume"` opens at - `+0x48` to `-1`, and four
floats at `+0xa4..+0xb0` (`0.35`, `2.5`, `1.0`, `1.0`) shaped like a crossfade
duration and two blend factors, unread by anything traced so far). **This is
as far as this pass chased it.** Where `+0x40`/`+0x44` actually reach the
decoded ATRAC/`sceMp3` stream - the step that would show whether they are a
linear gain, a `0x400`-style fixed-point scale, or something else entirely -
is not found. That is the next thing to read, not this pass's own claim.

### Group `0`'s duck: recovered, and it is not speech

The trigger `mgr+0xc0` field search (`scripts/psp-relocate.py field 0xc0`,
filtered to non-stack `sb` writes) finds exactly one write to it anywhere in
the binary outside `Audio_UpdateGroupVolumes`'s own read/clear of it: inside
a large, **not renamed** function at `0x08829778` (confidence too mixed across
the whole function to name it - it also handles camera/doppler-shaped work
this pass did not chase), one line reads

```c
if (*(int *)(param_2 + 0x7c8) == 4) {
  *(undefined1 *)(_DAT_002bde10 + 0xc0) = 1;
}
```

`_DAT_002bde10` is `mgr` again - so **this is the one and only trigger for the
group-`0` duck in the whole binary**, gated on some other object's `+0x7c8`
field equalling `4` (a race/game-state enum value, meaning not identified).
Confidence in the trigger address and its target is 80 (unambiguous
decompilation, single call site found by exhaustive field search); confidence
in what state `4` *means* is 0 - not claimed, not named. Either way: **whatever
this ducks, it is not music** - group `0` is one of the nine SFX-side groups
`Audio_UpdateGroupVolumes` writes, and music was already shown above to never
reach that function's `mgr` argument at all. "Almost certainly music under
speech" is retracted.

### Read live, mid-race: everything above sits at its own ceiling

PPSSPP under Xvfb (own display, own websocket port `47833`, the machine's
shared PPSSPP profile since this session's sandbox disallows overriding
`HOME`/`XDG_CONFIG_HOME`), `pulse-psp-usa.chd`, `scripts/psp-drive.py --port
47833 menu --single-race --any-track` into a live eight-craft grid, then
`cross` held for 24 s while free-running (no breakpoints - see
`ppsspp-debugger.md`'s cost table for why: 520 ms per read while running is
fine for six spaced samples, not for a per-tick capture). Six reads, 4 s apart,
covering `t=4s` through `t=24s` mid-race:

| Field | Value, every sample |
| --- | --- |
| All 16 entries of `g_audio_group_volumes` (`0x08ac3910`) | `1024` |
| `mgr+0x90` (SFX fade, current) | `1.0` |
| `mgr+0xd0` (SFX fade, target) | `1.0` |
| `mgr+0xc0` (group-`0` duck flag) | `0` at each sample |
| `mgr+0xcc` (group-`0` duck multiplier) | `1.0` at each sample |
| `g_music_player_ptr+0x40`, `+0x44` (Music Volume, `0`-`100`) | `100`, `100` |

**`mgr+0xc0` reading `0` at each 4 s-spaced sample does not by itself show the
duck never fired** - `Audio_UpdateGroupVolumes` clears that flag itself every
frame once handled (`*(undefined1 *)(param_2 + 0xc0) = 0;` in its own body,
quoted above), so a one-shot trigger between two samples would already read
`0` again by the next one. **`mgr+0xcc` is the field that actually shows the
duck was inactive throughout**: it is the persistent multiplier the duck ramps
away from `1.0` and back, and it held exactly `1.0` - untouched - at all six
samples. Taken together with `mgr+0xc0` never being caught in the `1` state
either, the more precise statement is: the duck did not fire in any of the six
instants sampled, and did not leave the multiplier anywhere off its rest value
in between them.

**Nothing here is below ceiling, on either path, at any of the six samples.**
This is the direct answer to the open question this page left in the section
above: the group volumes do *not* drop mid-race under a stock, unmodified
settings profile - not because the mechanism is idle by construction, but
because the SFX Volume slider (which is all this mechanism is) sits at its own
default, `100`, the same as the music player's own slider. **Confidence 88**
on the read itself (runtime trace, single binary, six independent samples
across a live race all agreeing with the static reading and with each other).

**This retires the group-volume/fade mechanism as an explanation for the music
loudness gap a second time, on stronger grounds than before**: not only does
it never drop below ceiling during a race, it was never wired to music in the
first place. "Measurement 2" above (our music bus alone, `-11.53` dBFS, already
louder than the original's *entire* race mix, `-15.27` dBFS) still has no
located cause.

### What is not recovered, and is the next thing to read

**Where `g_music_player_ptr+0x40`/`+0x44` reach the actual decoded audio.**
This pass traced the *setting* of the music volume all the way from the
options-menu string to its resting place on the music player object and
confirmed live that it sits at the same unattenuated default the settings
screen opens with - but never found where those two fields are read back out
and turned into a gain applied to the `sceAtrac3plus`/`sceMp3`-decoded stream
(`imports.md`'s "PCM output and streamed music" row, and `frontend-video.md`'s
note that the one `sceAtracSetDataAndGetID` cross-reference "belongs to the
separate music path", not the movie player). Whatever happens between that
decode and the DAC is where the missing ~4 dB most plausibly still lives -
this page has now ruled out every other stage of the chain the DAC end
(`Audio_OutputThread`'s `master << 5`) and the group-volume end
(`Audio_UpdateGroupVolumes`, above) both cover, and the per-voice SAS side is
separately ruled out in [sound.md](sound.md). **No number is ported from this
pass** - nothing here was found to be anything other than unity, so there is
nothing measured to apply, and inventing one now would be exactly the
plausible-looking stand-in `CLAUDE.md` forbids.

## The 4 dB gap is a fixed 0.44 trim on the music bus alone, found and measured live (2026-09-07)

Picking up exactly where the section above left off: "where
`g_music_player_ptr+0x40`/`+0x44` reach the actual decoded audio." They do not
reach it directly - a third field does, on a third global entirely, and it is
a plain constant, not a fader.

**`scripts/psp-relocate.py xrefs 0x08ac1dd8`** (the `member`/`field`
subcommands and `get_xrefs_to` all being either the wrong shape for this or
broken - see this page's own earlier notes) turns up every `lui`/`lo16` pair
that loads `g_music_player_ptr`'s own address, 35 sites across 27 functions.
Two of those sites - `0x08938090` and `0x0893c208` - sat in address gaps
Ghidra had never boundaried as functions at all (`get_function_by_address`
returned nothing for either, even though the bytes were already disassembled)
because nothing reaches them by a direct `jal` Ghidra's own analysis found -
they are called through a function pointer, the same indirect shape this
page's `Sas_CommitVoices` callers already showed. `create_function` at both
addresses (Ghidra MCP, `dry_run` first) carved out two real, complete
functions immediately - not data, not padding.

Both decompile to the same law:

```c
int MusicPlayer_ComputeStreamGain(void)
{
  int gain = (int)((float)(int)((float)g_music_player_ptr->vol_current *  // +0x40, 0-100
                              g_music_player_ptr->blend) *                 // +0xb0, 1.0 outside a crossfade
                  0.01 * g_music_master_gain * 32768.0);
  // ... a fade-*out* toward zero, armed by a Stop(fade: bool) call and
  // decaying 0.05 per invocation until it clamps at 0.0, not a fade-in -
  // then clamp the whole result to [0, 0x8000]
  return gain;
}
```

The correction matters: `DAT_002bddd0` (armed) and `DAT_002bddd4` (the decaying
multiplier) are set by the stop path, not a start path - `FUN_089385b4`
(`0x089385b4`) writes `DAT_002bddd0` from its own `fade: bool` parameter on a
stop, and `DAT_002bddd4` only ever counts down toward `0.0` here, never up.
This is the music player winding itself down on `Stop`, unrelated to
`g_music_master_gain` or the slider - noted here because an earlier draft of
this section called it "a separate one-shot fade-in ramp," backwards.

`MusicPlayer_ComputeCallbackGain` (`0x0893c208`) is byte-for-byte the same
expression, called from two sites near the channel-open function
(`0x0893bb5c`) rather than from the streaming decode-fill loop
(`0x089381f0`) `MusicPlayer_ComputeStreamGain`'s one caller sits in - two
playback paths, the same gain law. `MusicPlayer_ComputeStreamGain`'s own
caller is unambiguous: it decodes into a buffer (`func_0x00134164` x2) then
calls `func_0x0018864c` (`0x0898c64c`) as `(gain, gain, sampleCount, buffer)`
immediately before the result is queued (`imports.md`'s `sceAtrac3plus` row
is what decodes the buffer being handed in).

**`0x0898c64c` (named `Audio_EnqueuePannedOutput` below, confidence 78) was
decompiled, not assumed, to check what `[0, 0x8000]` actually means to it.**
It gates on a channel-ready flag
(`_DAT_002bf238`), then - for exactly the two sample counts this caller ever
passes (`0x100`, `0x900`, both a multiple of `0x100`) - stores `param_1 >> 3`
and `param_2 >> 3` (the two gain arguments, right-shifted by 3) alongside the
unshifted sample count and buffer pointer into a small fixed staging area
(`_DAT_000afc10`..`_DAT_000afc1c`), and returns the SDK-shaped error code
`0x80440011` on the gated-off path. That is the shape of `sceAudio`'s own
panned-output call being fed - `sceAudioOutputPannedBlocking`'s volume
parameter is documented PSP SDK-wide as `0`-`0x8000` (`PSP_AUDIO_VOLUME_MAX`),
and `0x8000 >> 3 = 0x1000` is exactly that hardware's known 12-bit volume
register width. So `[0, 0x8000]` is not an in-place PCM sample multiply this
page invented a plausible reading of - it is the software volume clamped to
the audio HAL's own documented maximum, which is unity by that API's own
definition. This also means the two identical gain-law copies are not
scaling *samples*, they are computing the *panned-volume argument* handed to
the hardware output call once per buffer - same conclusion for
`g_music_master_gain`'s effect either way.

**Two things corroborate `0x8000` as unity independently of this decompile,
worth citing since the one function this leans on is a single reading:**

- `0x0898c64c` sits about 1 KiB before `Audio_OutputThread` (`0x0898ca54`,
  same translation unit), which this page already documented as handing
  `master << 5` to the DAC - `Audio_Init`'s master ceiling `0x400` shifted
  by `5` is `0x8000` too, the same convention on the DAC-facing side of the
  mixer as on this music-facing side.
- The arithmetic floors it independently: at any trim much weaker than
  `0.44` the original's own music-alone RMS would not fit under its measured
  whole-mix RMS of `-15.27` dBFS. A trim of `0.88` (half as strong, `-1.1`
  dB less than half of `0.44`'s effect), for instance, would put music alone
  at roughly `-12.6` dBFS off the same raw track - already louder than the
  *whole mix* including engines, the exact contradiction this thread opened
  with. `0.44` is comfortably inside the range the measurement itself
  requires, not merely the range one decompile suggests.

**`g_music_master_gain` (`0x08ac1e18`, previously `_DAT_002bde18`) is a plain
`f32`, set once, in `MusicPlayer_Init` (`0x08938428`), to the bit pattern
`0x3ee147ae`.** That decodes to exactly **`0.44`, about -7.13 dB**. Nothing
else in the whole binary writes to it -
`scripts/psp-relocate.py xrefs 0x08ac1e18` finds exactly three references:
the one write (the constructor) and the two reads (the two gain-law copies
above). It is not a slider, not a duck, not a per-track table - a single
hardcoded trim on the music bus alone, entirely separate from
`Audio_SetSfxFadeTarget`'s SFX/Speech chain, which never touches this global.

**Read live, mid-race, corroborating the static decompile exactly.** PPSSPP
under its own Xvfb/port, `pulse-psp-usa.chd`, driven into a full-grid Single
Race with `scripts/psp-drive.py`'s `menu()`, `cross` held through a 24 s
window, six samples 4 s apart:

| Field | Every sample |
| --- | --- |
| `g_music_master_gain` (`0x08ac1e18`) | `0.44` |
| `g_music_player_ptr+0x40` (current volume, 0-100) | `100` |
| `g_music_player_ptr+0x44` (target volume, 0-100) | `100` |
| `g_music_player_ptr+0xb0` (crossfade blend) | `1.0` |

Nothing moved through the window: the slider fields sit at their own ceiling
exactly as the prior section already found, and the trim sits at its one
hardcoded value throughout. **Confidence 92** (a clean, unambiguous static
decompile - three references total, one write, two reads, both reads doing
the same arithmetic - corroborated live with six agreeing samples; not yet
corroborated against `pulse-psp-eu`, the release the port itself is measured
against below).

### Ported

`crates/audio/src/mixer.rs` gains `pub const MUSIC_MASTER_TRIM: f32 = 0.44`,
documented with this evidence and confidence, applied to `Bus::Music` alone -
`Audio::apply` (`crates/game/src/audio.rs`) now sets
`mixer.set_bus_gain(Bus::Music, settings.music_volume.gain() * MUSIC_MASTER_TRIM)`,
leaving the SFX and Speech buses untouched, matching the original's chain
exactly: this trim has no analogue on the SFX/Speech side because the
original's own code never puts it there either.

### Re-measured: race mix after the trim

30 s capture, `pulse-psp-eu.chd`, `--race --autopilot --ticks 1800`, default
settings (all volumes at 100), same methodology as the pre-trim measurement
this section corrects:

| | Peak | RMS | Clipped |
| --- | --- | --- | --- |
| Original's whole race mix (`pulse-psp-usa`, live, for reference) | 0 dBFS | -15.27 dBFS | 0.044% |
| Our port, whole mix, before this trim | 0 dBFS | -7.52 dBFS | 2.783% |
| **Our port, whole mix, after this trim** | 0 dBFS | **-7.87 dBFS** | **2.312%** |
| Our port, music bus alone, before this trim | 0 dBFS | -11.53 dBFS | 0.000% |
| **Our port, music bus alone, after this trim** | **-7.13 dBFS** | **-18.66 dBFS** | **0.000%** |

**The contradiction this whole thread started from is gone**: the music bus
alone is no longer louder than the original's entire race mix (`-18.66` vs
`-15.27` dBFS) - it is now the quieter of the two, which is the relationship
adding an uncorrelated engine mix on top of it should produce. Both music-side
numbers moved by exactly `-7.13` dB (peak `0` -> `-7.13`, RMS `-11.53` ->
`-18.66`), matching `MUSIC_MASTER_TRIM`'s own dB value to the last digit
`f32` arithmetic allows - the trim reached the right place and nothing else on
the music path moved.

**The whole mix barely moved (`2.783%` -> `2.312%` clipped), and that is
expected, not a sign the trim did too little.** The clipping this thread
opened with was already isolated to the SFX side before this pass ever
started - several honestly-scaled voices summing two to six times a single
voice's own value, with no reserved headroom - which this pass does not touch
and was never asked to. The music bus was never the majority contributor to
the *clip count* - only to the RMS gap this thread was actually chasing - and
that gap is now closed.

**Listening check**: a waveform-shape read, the same kind an earlier pass over
this subject already used, since this sandbox has no audio device. Before
this trim, the music bus alone (`-11.53` dBFS) sat close enough to the whole
mix (`-7.52` dBFS) to plausibly mask the engines under it; after, the whole
mix's RMS (`-7.87` dBFS) sits a clear `10.8` dB above the music bus alone
(`-18.66` dBFS), which is the shape of "engines and effects in front, music
behind" rather than "music competing with everything else" - nothing in
either capture drops to silence or a stuck loop, and the music bus's own
clip count went from 2 samples to 0, so quieter, not gone.

The full re-measurement, the SFX-side summing-headroom mechanism this section
does not touch, and the gate result for this change live in the project's
own work-in-flight tracker, per this project's rule that a `docs/` page never
references it - the thread citing this page's own section anchor is the one
to read for that.

## The SFX-side gap does not need summing at all - it is there at one voice (2026-09-07)

With the music path settled, this pass isolated the SFX bus the same way the
music bus was isolated above: mute music at the source, dump the rest, and
compare like for like. **The result overturns this page's own
"several honestly-scaled voices summing 2-6x" framing as the explanation.**
The gap is present, at nearly the same size in dB, with **one voice**.

### Method: mute `g_music_player_ptr`'s volume live, instead of the options menu

Rather than navigate the options menu (which would persist a profile change
and cost a screenshot-driven walk for no reason), this pass wrote zero
directly to the already-recovered fields from the section above:
`g_music_player_ptr` (`0x08ac1dd8`) dereferenced to `mgr`, then
`memory.write_u32(mgr+0x40, 0)` and `memory.write_u32(mgr+0x44, 0)` - the same
`vol_current`/`vol_target` pair the options-menu slider writes, set to the
same value muting the slider would produce. Confirmed by reading them back
(`100, 100` -> `0, 0`) before resuming. This is a measurement technique, not a
mixer change: it isolates the SFX bus at its own existing volume input, the
same way pinning `music_volume = 0` in our own settings isolates it on the
port side, and it touches no persisted PPSSPP profile.

### Measurement 1: full eight-craft grid, SFX only

24 s window (tick-bracketed, same method as the section above), `pulse-psp-usa`,
Single Race, full grid, thrust held, music muted live:

| | Peak | RMS | Clipped |
| --- | --- | --- | --- |
| Original, SFX alone (music muted live) | 0 dBFS | **-16.61 dBFS** | 633 / 2,117,378 (**0.030%**) |
| Original, whole mix (for reference, from the section above) | 0 dBFS | -15.27 dBFS | 0.044% |
| Our port, SFX alone (`sfx_volume=100`, `music_volume=0`, 30 s, `--race --mode single_race --autopilot`) | 0 dBFS | **-8.46 dBFS** | 47,115 / 2,880,000 (**1.636%**) |

**Muting the original's music barely moves its own clip rate (0.044% ->
0.030%)** - confirming music was never a meaningful contributor to the
original's clip count, consistent with the music-alone measurement above
(2 samples in 30 s). The gap between the two sides is **8.15 dB of RMS** and
close to two orders of magnitude in clip rate, and it is entirely on the SFX
side - exactly what this thread's title already suspected, now isolated
rather than inferred from the whole mix.

### Measurement 2: one craft, one engine voice, no summing possible at all

If the gap above were the "2-6x sum" mechanism this page already measured
(the per-voice SAS section, above), it should shrink sharply with only one
voice able to sound. It does not.

**The two captures are not the same scenario, and that matters more than the
RMS number below - said plainly rather than left to be noticed.** The
original side is `menu`'s own hold-`cross`-and-release, no steering: on
Talon's Junction that free-runs into the first wall rather than following the
racing line, the shape `ppsspp-debugger.md` already documents for an
open-loop hold. Our side is `--autopilot`, which stays on the line at
110-167 speed with real airtime and landings. Different trajectories, and a
different population of one-off cues (wall scrapes on one side, landing
thumps on the other) riding on top of the steady engine tone either way.

**What makes the engine-tone half of the comparison legitimate anyway:
`exhaust.md`'s own law saturates intensity within about four seconds,
regardless of speed.** `i += dt * 0.25` clamped to `[0, 1]` reaches `1.0`
`4/0.25 = 4` ticks... no - `1 / 0.25 = 4` seconds after the engine comes on,
and speed only ever moves *pitch*, never `i`. So past the first four seconds
of a 24 s window, both sides' engine voice sits at the law's own ceiling,
`0.6 * 1.0 + 0.4 = 1.0`, for the rest of the capture - the steady-tone half
of each capture is comparable even though the two trajectories are not,
which is a read off `exhaust.md`, not an assumption this pass is making to
paper over the mismatch.

Time Trial, one craft, full throttle, music muted the same way, 24 s
tick-bracketed window on the original; our own port run with `--mode
time_trial` (no opponents, no weapon pads - `main/cli.rs`'s own doc comment:
"`single_race` is the only one that races with weapons"), same settings pin:

| | Peak | RMS | Clipped |
| --- | --- | --- | --- |
| Original, one craft, engine alone (music muted live), un-steered hold-`cross` | **-4.14 dBFS** | **-19.65 dBFS** | 0 / 2,117,446 (**0.000%**) |
| Our port, one craft (`--mode time_trial --autopilot`, on the racing line, otherwise identical pin) | 0 dBFS | **-11.15 dBFS** | 4,534 / 2,880,000 (**0.157%**) |

**Lead with the peak, which is scenario-proof in a way the RMS delta is
not: the original's one craft never reaches 0 dBFS anywhere in 24 s of
full-throttle engine, and our port hits full scale 4,534 times in the same
window shape.** No trajectory difference explains a single voice never
touching the ceiling on one side and routinely pinning it on the other. The
RMS gap (**8.50 dB**) is corroborating, not load-bearing on its own, and it
is the same size, to within 0.35 dB, as the eight-craft gap above despite the
two captures' different cue populations. **A gap that does not shrink when
the voice count drops from eight to one is not a summing effect.** This
retires "several honestly-scaled voices summing 2-6x, no reserved headroom"
(this page's own 2026-09-06 section) as the saturation's cause: that
mechanism is real - the per-voice SAS values do sum 2-6x, measured - but it
cannot be *the* reason our port clips, because the same-sized gap exists with
only one voice able to sum with anything.

### What actually scales it: a pipeline stage this port has never had

[`positional-audio.md`](positional-audio.md#the-two-hardware-volumes-and-the-table-that-makes-them)
already documents, from a 2026-09-01 pass unrelated to this thread, a stage
between `SoundEmitter_ComputeVolumeAndAngle`'s `0..1024` output (which *is*
faithfully ported - `oag_audio::spatial::Emitter::place`'s `volume_curve`,
runtime-verified 1,610/1,610 against the original) and the two hardware
channel volumes: `Scream_PanVolumePair` (`0x08995a9c`) recombines **four**
terms, `p1 * p2 * p4 * p6 * 0x102 / 0x7f^3`, **some of them squared under a
mode mask** the page already reads (mask `10`) but does not yet know the
*meaning* of - only that two of the four terms get squared and two do not.

**This entire stage is absent from the port.** `crates/audio/src/spatial.rs`
takes `SoundEmitter_ComputeVolumeAndAngle`'s output (`volume_curve(atten *
volume)`) and hands it to the mixer as the *final* linear gain; `grep` across
`crates/audio` and `crates/game/src/audio` for `Scream`/`PanVolumePair` finds
nothing. The original does not stop at that `0..1024` value - it is one of
(at least) four inputs multiplied together, with two of them squared, before
the result divides by `0x3fff` into the actual channel gain. A product of
several sub-unity terms is systematically quieter than any single term alone,
which is the shape of exactly what Measurements 1 and 2 above show: a gap
that is there per-voice, before any summing, and consistent in size whether
one voice sounds or eight do.

**Confirmed live, not left as a maybe: at real full-throttle engine values,
this stage attenuates by 4-13 dB, not gains.** `combined`'s theoretical
ceiling (`0x7ffe`, all four terms at `0x7f`) divides to `2.0` - a gain, if
every term happened to sit at its own maximum, which would have made this
stage the *wrong* lead entirely. It doesn't happen. `Scream_PanVolumePair`
(`0x08995a9c`) was read live with `memory.disasm` (PPSSPP's own disassembly
API, not Ghidra) while broken on its entry, during an actual full-grid Single
Race with thrust held: the stack word at `sp+0` is the mode mask (read as
`10`, matching `positional-audio.md`'s static decompile exactly, now
independently confirmed from the running binary), and the instruction
sequence resolves the four terms and which two the mask squares:

```text
combined = a0 * (a3 * ((t1_sq * (a1_sq * 0x102)) / 0x7f)) / 0x7f) / 0x7f
  where a1_sq = mask&2 ? a1*a1/0x7f : a1   (squared: mask bit 1 is set)
        t1_sq = mask&8 ? t1*t1/0x7f : t1   (squared: mask bit 3 is set)
  a0 and a3 are used unsquared (mask bits 0 and 2 are clear)
```

`a0`-`a3` are the standard MIPS o32 register arguments; `t1` is a fifth value
already resident in a callee-saved-by-convention register at the function's
own entry, not one of the four visible arguments - who sets it up is exactly
the caller-side question left below. Ten breakpoint hits at
`Scream_PanVolumePair`'s entry, full throttle, full grid, read
`(a0, a1, a3, t1)` directly off `cpu.getAllRegs` and ran through this exact
sequence:

| `(a0, a1, a3, t1)` | `combined` | gain (`combined/0x3fff`) | dB |
| --- | ---: | ---: | ---: |
| `(126, 50, 127, 110)` | 3,637 | 0.222 | -13.07 |
| `(127, 50, 127, 110)` | 3,666 | 0.224 | -13.00 |
| `(77, 90, 127, 127)` | 9,854 | 0.602 | -4.42 |
| `(79, 67, 127, 110)` | 4,201 | 0.256 | -11.82 |

Mean **-11.7 dB** across these ten hits (four distinct value tuples, several
repeats) - the same size, within the noise a ten-hit sample carries, as the
8.15-8.50 dB RMS gap Measurements 1 and 2 above already isolated. **`a3` read
`127` (max, unsquared) on every single hit** - consistent with the
already-established finding that the group-volume table sits at ceiling
throughout a race, so this term is not the mystery; **`a0`, `a1` and `t1` all
varied** and are the ones actually doing the attenuating.

**This settles the direction, which the theoretical ceiling alone could not:
`Scream_PanVolumePair` is a genuine, currently-unported attenuation on the
SFX path, of about the right size, not merely a stage that happens to exist.**
It is not ported here anyway - what remains unread is **where `a0`, `a1` and
`t1` are set on the caller side**, i.e. what each one physically represents
in terms this port already computes (`SoundEmitter_ComputeVolumeAndAngle`'s
`0..1024`? a per-bank or per-cue base volume? something read back from a SAS
voice?). Without that, wiring the same arithmetic into
`oag_audio::spatial::Emitter::place` would mean *guessing* which of the
port's own values maps to which of these three terms - exactly the invented
mapping `CLAUDE.md` forbids, even though the arithmetic itself is now
exactly known. **Nothing is changed in `crates/audio` or
`crates/game/src/audio` by this pass.** The next step is tracing
`Scream_PanVolumePair`'s callers (`Scream_SetSoundVolume` and neighbours,
reached from `SoundInstance_UpdateSpatial`) to learn what feeds `a0`/`a1`/`t1`
- either with the Ghidra bridge once it is available again, or with more of
the same live-breakpoint technique this section used, walked one call frame
further out.

**Confidence 85** on the diagnosis (mechanism is per-voice, not summing -
two independent live measurements agreeing to 0.35 dB) and **82** on
`Scream_PanVolumePair` being a real, measured attenuation of roughly the
right size at real gameplay values (ten live hits, one binary, one session;
not yet corroborated against a second capture or against `pulse-psp-eu`) -
up from no confidence score, now that the direction is read rather than
bounded.

### Whole mix, unchanged, because nothing was changed

30 s capture, `pulse-psp-eu.chd`, `--race --autopilot --ticks 1800`, all
volumes at 100, same recipe as every prior row in this table - measured fresh
this pass to confirm nothing drifted, not because anything was expected to:

| | Peak | RMS | Clipped |
| --- | --- | --- | --- |
| Original's whole race mix (`pulse-psp-usa`, live, for reference) | 0 dBFS | -15.27 dBFS | 0.044% |
| Our port, whole mix (previously measured) | 0 dBFS | -7.87 dBFS | 2.312% |
| **Our port, whole mix (this pass, unchanged code)** | 0 dBFS | **-8.11 dBFS** | **2.001%** |

The small differences from the previously-recorded row (0.24 dB, 0.31 points)
are **not attributed to a code effect - no line in `crates/audio` or
`crates/game/src/audio` changed this pass** - but the exact cause of the
drift itself is not pinned down either: candidates include a different
debug-profile build, a fresh settings file resolving a different default
track than the prior pass's own pin directory, and ordinary autopilot/AI
variance. Not chased further since 0.3 points is noise against the 8+ dB
finding above. The whole mix still clips at essentially the same rate this
thread's table has shown since the music trim landed, because the diagnosis
above is a *pipeline stage identified as missing*, not yet a port.

**Listening check**: waveform-shape only, no audio device in this sandbox.
The one-craft capture (Measurement 2) never reaches full scale on the
original at all and reaches it routinely on our port; nothing in either
capture drops to silence or loops on itself. This is the same shape as every
prior listening check on this thread - "louder than it should be," not
"broken" or "missing."
