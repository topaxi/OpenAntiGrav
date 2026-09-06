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
