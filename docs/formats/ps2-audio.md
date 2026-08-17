# PS2 music archive

**Status: understood.** `PS2MUSIC.WAD` is a flat archive of **48 kHz 16-bit
stereo PCM**, one entry per music track, with no compression and no codec.

Despite the extension it is **not** the [WAD container](wad.md) the rest of the
game uses. The header is different, the entry fields are in a different order,
and there is no compression flag because nothing in it is compressed.

## Layout

```text
+0x00  u32          entry_count
+0x04  entry[entry_count], 12 bytes each

entry:
  +0x00  u32  name_hash    the same CRC-32 variant as the WAD container
  +0x04  u32  size
  +0x08  u32  offset       from the start of the file
```

That is the whole format. There is no directory terminator, no alignment
padding and no per-entry header: entry 0 starts at offset 196, immediately after
the table, and each entry starts exactly where the previous one ended.

The field order is the first thing to get wrong. The ordinary WAD container
stores `{hash, offset, size_uncompressed, size}` in 16 bytes; this stores
`{hash, size, offset}` in 12. Reading it as the WAD entry layout gives an
offset of 36,018,740 for the first track.

## Contents

Signed 16-bit little-endian PCM, two channels interleaved left first, **48,000
Hz**. Nothing in the file says any of that; it is established below.

On the EU disc there are 16 entries totalling 558 MiB, each a whole music track
of roughly three and a half minutes.

Storing a disc's entire soundtrack uncompressed is less surprising than it
sounds here: the disc also carries `PADZ2`, a 2.75 GiB padding file, so more
than two thirds of the DVD is empty. The bandwidth was free and the CPU cost of
decoding was not.

## Evidence

### The chain closes exactly

Taking the second word as the size and the third as the offset, each entry's
`offset + size` is the next entry's offset, for all 16, and the last one lands
on **585,799,372** - the file's exact length, to the byte. There is no slack
anywhere in the file, which is also what rules out any per-entry header: there
is nowhere for one to be.

### The frame size is 4 bytes, from the sizes alone

Every one of the 16 entry sizes is divisible by 4. Only 6 of them are divisible
by 8, and only 2 by 16, so the alignment is exactly 4 and not more. Sixteen
independent sizes all landing on a 4-byte boundary while behaving randomly at 8
is not something an arbitrary blob does; 16-bit samples times 2 channels is.

### It is interleaved stereo, not mono, and channel 0 is the left one

Two statistics over a 4 MB slice of each track, both computed on the same
samples:

- **Neighbour distance at lag 1 against lag 2.** In interleaved stereo, lag 2 is
  the *same* channel one sample later and lag 1 is the *other* channel at the
  same instant, so lag 2 has to be the smaller. In mono at double the rate it is
  the larger. Measured ratio `lag2/lag1` runs **0.37 to 0.79** across all 16
  tracks - never above 1.
- **Left/right correlation**, 0.64 to 0.97 across all 16.

A mono stream, a different sample width, or a one-byte misalignment breaks both.
That last one is worth recording because it happened: probing at `size/3`, which
is not a multiple of 4 for 7 of the 16 tracks, made those 7 look like white
noise - `lag2/lag1` of 1.00 and a correlation of 0.00 - and they read as a
different format entirely until the probe was frame-aligned.

Neither statistic can tell **which** channel comes first: both are symmetric
under swapping them. The PSP disc settles that too. Aligning a PS2 track against
its ATRAC3plus counterpart on the mid signal (`ch0 + ch1`, correlation +0.98 at a
lag of -2,495 samples) and then correlating the **side** signal (`ch0 - ch1`),
which *is* antisymmetric under a swap, gives:

| PS2 track vs PSP master | Mid | Side |
| --- | ---: | ---: |
| 187.59 s | +0.984 | **+0.980** |
| 194.01 s | +0.981 | **+0.930** |
| 188.74 s | +0.993 | **+0.963** |

A swapped channel order would put those three side figures at about -0.98, not
+0.98. Channel 0 is left.

### The sample rate is 48 kHz, cross-checked against the PSP disc

Nothing in the container states the rate, and 44,100 and 48,000 both give
plausible track lengths (204 s and 188 s). The PSP disc settles it: it carries
**the same sixteen music tracks** as ATRAC3plus in `Data.wad`, which do state
their rate, and `ffprobe` reads exact durations off them.

| Rate assumed for the PS2 PCM | Mean gap to the nearest PSP track | Matching |
| --- | ---: | --- |
| 44,100 Hz | 5.489 s | collapses - one PSP track claimed by 8 PS2 tracks |
| **48,000 Hz** | **0.011 s** | a clean bijection, all 16 |

At 48 kHz every PS2 track matches a *distinct* PSP track to within 10
milliseconds - 177.23/177.21, 187.59/187.58, 204.35/204.34 and so on down the
list. The residual 10 ms is the ATRAC3plus decoder's trailing frame padding, and
it is the same sign and magnitude on all sixteen.

That is a genuinely independent measurement: the two discs encode the same
masters with different tools, so the durations agreeing to four significant
figures across sixteen tracks cannot come from a wrong rate.

### The pairing itself

Recorded because it is the only thing that lets a name mined on one disc be
carried to the track it names on the other, and because **the two orders are not
the same** - PS2 track 12 is the *second* of the PSP's sixteen in `Data.wad`
order, so indexing one archive by the other's position is silently wrong.
Measured against the USA UMD; the PS2 pressing is EU.

PS2 durations are `size / (4 * 48000)`, PSP durations are the `fact` chunk's
sample count over 44,100. The gap is **-11.4 ms on every one of the sixteen, to
a tenth of a millisecond** - a constant, which is what the ATRAC3+ encoder's
trailing padding is.

| PS2 index | PS2 hash | PS2 s | `Data.wad` index | PSP hash | PSP s |
| ---: | --- | ---: | ---: | --- | ---: |
| 0 | `c1bab1b0` | 187.592 | 452 | `f38af1b6` | 187.581 |
| 1 | `22a44561` | 195.789 | 461 | `7abd1960` | 195.778 |
| 2 | `44068dee` | 188.739 | 459 | `93283c5d` | 188.728 |
| 3 | `93b0dc72` | 193.550 | 465 | `5baf27cd` | 193.538 |
| 4 | `eeae816c` | 189.731 | 467 | `154c596f` | 189.719 |
| 5 | `02330210` | 187.814 | 471 | `34a3fe68` | 187.803 |
| 6 | `46e4d191` | 182.009 | 483 | `ca7e9353` | 181.998 |
| 7 | `a62c213b` | 177.226 | 486 | `10789bd4` | 177.215 |
| 8 | `40001110` | 183.913 | 489 | `b2787ad8` | 183.902 |
| 9 | `f962a9e0` | 202.632 | 491 | `75150993` | 202.620 |
| 10 | `a173e7fc` | 200.624 | 495 | `a59538fc` | 200.612 |
| 11 | `9bd0c692` | 204.347 | 499 | `a481906f` | 204.336 |
| 12 | `877d1753` | 194.013 | 454 | `0d3af1a5` | 194.001 |
| 13 | `8a34b8f9` | 183.440 | 502 | `c33648c4` | 183.429 |
| 14 | `8d641f3a` | 182.143 | 506 | `f0ee1374` | 182.132 |
| 15 | `0d8abd27` | 197.474 | 509 | `f5b9bdf4` | 197.462 |

**A second Studio Liverpool UMD is close enough to matter.** Wipeout Pure's
`Data.wad` carries **nineteen** large stereo ATRAC3+ entries, 205.2 to 326.1 s,
against Pulse's sixteen at 177.2 to 204.3 s. The two populations very nearly
touch: Pure's shortest is 0.87 s from Pulse's longest. So a single track
matching a length proves nothing, and only a complete sixteen-for-sixteen
assignment does. `oag-game` relies on that when it goes looking for the other
disc - see `MusicDiscs` in `crates/game/src/audio.rs`, where trusting the disc
serial instead reported `pure-psp-eu.chd` as a Pulse PSP counterpart.

The population on the PSP side is picked out by what the entries **are**, not by
where they sit: stereo ATRAC3+ at 44,100 Hz above 2 MB. That is exactly sixteen
entries out of 1,142, and the channel count is what carries it - the disc also
holds 32 *mono* ATRAC3+ streams at the same 12,058 B/s and a dozen shorter
stereo ones, so a size filter alone gets it wrong in both directions. Same
argument as [`ps2-voice.md`](ps2-voice.md) makes for the pre-race clips.

The 2 MB floor is arithmetic rather than a round number: the shortest PS2 track
is 177.2 s, and at 560 bytes per 2,048 samples that is `177.2 * 44100 / 2048 *
560` = 2,137,000 bytes of stored stream, so nothing shorter can be one of them.
The closest two tracks are 0.13 s apart, a hundred times the observed 11.4 ms
error, so the assignment has an enormous margin.

Confidence: **93**. An exact arithmetic bijection on sixteen pairs with a
constant residual and a 11x separation to the nearest wrong answer, corroborated
by the null test below on the pair it was run on. One below the container's 94
because the durations on the PSP side are read off a `fact` chunk rather than
decoded and counted.

## The two discs carry the same master, and the PS2 carries more of it

The +0.98 side correlation above proves the same *recording*. It does not prove
the same master, and 48 kHz against 44.1 kHz means at least one side was
resampled. A **null test** on PS2 track 0 against its partner, `Data.wad` entry
452, settles both. Same method as
[`ps2-voice.md`](ps2-voice.md#the-two-discs-carry-the-same-recordings), with one
addition that side did not need: a resampler is in the path here, so its own
error is measured first and separately.

### The resampler is not in the answer

`ffmpeg`'s soxr at `precision=28`, run 44,100 -> 48,000 -> 44,100 on the PSP
decode and nulled against the original. That direction is chosen deliberately:
it loses no band, where 48,000 -> 44,100 -> 48,000 would discard everything
above 22.05 kHz and measure that instead of the resampler.

| | Value |
| --- | ---: |
| Correlation | **+1.00000** |
| Fitted gain | +0.000 dB |
| Fitted fractional delay | 0.00 samples |
| Residual SNR | **82.7 dB** |

That is 66 dB below the residual measured below, so the resampler contributes
nothing to it.

### The null test

PS2 track 0 resampled to 44,100 with the same resampler, aligned against entry
452, a single least-squares gain fitted and subtracted. Alignment is a constant
integer lag of **-2,162 frames** with a fitted fractional delay of **0.00
samples**; 185.6 s compared, edges trimmed.

| | Correlation | Fitted gain | Residual SNR |
| --- | ---: | ---: | ---: |
| Left | +0.98800 | -0.065 dB | **16.22 dB** |
| Right | +0.98746 | -0.065 dB | 16.03 dB |
| Mid (L+R) | +0.98799 | -0.051 dB | 16.22 dB |
| Side (L-R) | +0.98463 | -0.240 dB | 15.16 dB |

Sliding 5-second windows over the mid signal give 37 windows at min +0.9779,
median +0.9879, max +0.9993 - **constant, not drifting**, which is what rules
out both a rate error and an edit difference of the kind six of the 32 voice
clips turned out to have.

**The gain fit is the same-master evidence**: -0.065 dB is a level difference of
0.7%, which is a codec's business rather than a mastering engineer's. A remaster
would show up here as a level, an EQ tilt or a compression difference, and none
of the three is present below 2 kHz.

### Where the 16 dB goes

Splitting the residual by band, on the left channel, says exactly what the
difference is:

| Band | PS2 energy | PSP energy | Fitted gain | Residual SNR |
| --- | ---: | ---: | ---: | ---: |
| 20-500 Hz | -1.27 dB | -1.23 dB | -0.005 dB | **30.20 dB** |
| 500-2,000 Hz | -7.69 dB | -7.64 dB | -0.034 dB | 22.54 dB |
| 2,000-6,000 Hz | -15.07 dB | -14.90 dB | -0.462 dB | 11.28 dB |
| 6,000-12,000 Hz | -13.41 dB | -14.33 dB | -1.057 dB | 4.31 dB |
| 12,000-16,000 Hz | -24.18 dB | -25.81 dB | -1.280 dB | 3.07 dB |
| 16,000-22,050 Hz | -33.37 dB | **-76.36 dB** | -20.831 dB | 0.00 dB |

The bottom row is the finding. Above 16 kHz the PS2 master carries -33 dB of
real content and the PSP encode carries -76 dB, which is nothing: ATRAC3+ at
96 kbit/s stereo lowpasses there, and **43 dB of signal simply is not on the
PSP disc.** Below 500 Hz the two agree to 30 dB. So the residual is a perceptual
codec's quantisation noise, rising with frequency exactly as one does, plus a
hard band limit at the top.

The whole-band 16 dB is worse than the voice archive's 28.9 dB, and that is
expected rather than surprising: this is dense stereo music at 96 kbit/s against
mono speech at the same bitrate.

The side signal's -0.240 dB against mid's -0.051 dB is worth one line: the PSP
encode narrows the stereo image very slightly, which is what joint stereo does.

Confidence: **92** that the two discs carry the same master, and that the PS2
side carries strictly more of it. The gain fit, the zero fractional delay, the
constant windowed correlation and the 43 dB band gap are all measurements on
shipped data, which the [rubric](../reverse-engineering/confidence-rubric.md)
caps at 94; two below that because only one of the sixteen pairs has been
nulled - the other fifteen rest on the duration bijection and the earlier
correlation figures.

## Loop points

**Neither release carries a musical loop point for any of the sixteen
soundtrack tracks.** The PS2 container has no field that could hold one. The
PSP's RIFF wrapper does - a `smpl` chunk - and **none of the sixteen has one**:
all sixteen carry exactly `fmt `, `fact` and `data`, walked to the end of the
entry rather than stopping at `data`.

Eleven of `Data.wad`'s 92 RIFF entries *do* carry a `smpl`, and it is not a
musical loop either. Every one of them declares a single loop, and on all eleven:

```text
loop_end - loop_start == fact - 1
```

exactly. `loop_start` is 2,664 to 3,853 samples and varies per entry;
`loop_end` is fixed by the encoded block count. So the loop is **the whole
decoded stream minus the codec's priming samples** - ATRAC3+ decoder-delay
compensation, written by the encoder, not a boundary anybody chose. The eleven
are the front end's own music (`Data\Music\FEMusic\frontend1.at3` among them)
and other short stereo cues, none of which has a PS2 counterpart.

That is an exact arithmetic invariant repeated over eleven independent entries,
the same class of evidence as the offset chain closing to the byte.

Confidence: **90** that no soundtrack track on either disc carries a loop point,
and **88** that the `smpl` chunks that exist are codec-delay compensation. The
first is an absence measured over both archives; the second is an exact identity
on eleven entries but nothing has been observed reading the chunk at runtime, so
what the game *does* with it is still open.

The practical consequence, and the reason this was measured: swapping which disc
the music comes from cannot change where a track loops, because neither disc
says. Both sides loop the whole buffer.

Confidence: **94** for the container layout and **92** for the contents. The
chain closing to the byte and the divisibility census are exact arithmetic; the
stereo interleave and the sample rate are statistical and cross-referential
rather than declared anywhere, which is strong but not exact. Per the
[rubric](../reverse-engineering/confidence-rubric.md) data agreement caps at 94
either way.

## Reproducing

```sh
just unpack extract data/images/pulse-ps2-eu.chd '*PS2MUSIC*' -o data/cache/ps2music
```

Use `data/cache/`, never `/tmp` - the archive is 559 MiB and a `/tmp` on a
tmpfs quota is how this has broken a shell before.

Then, for track 0, skip the 196-byte table and write a WAV header for 48 kHz
16-bit stereo, or:

```sh
ffmpeg -f s16le -ar 48000 -ac 2 -ss 0 \
  -i <(tail -c +197 data/cache/ps2music/54748/PS2MUSIC.WAD) \
  -t 30 data/cache/ps2music/track0.wav
```

The ground-truth test is
[`crates/formats/tests/audio_ground_truth.rs`](../../crates/formats/tests/audio_ground_truth.rs).

### The pairing, the loop points and the null test

`just unpack extract data/images/pulse-psp-usa.chd '*Data.wad' -o data/cache/psp`,
then walk the 16-byte WAD directory (see [`wad.md`](wad.md)), take every entry
over 2 MB whose blob starts `RIFF`, and read its `fmt ` and `fact` chunks. That
gives the sixteen and their lengths; the `smpl` walk is the same loop carried
**past** the `data` chunk rather than stopping at it, which is the one thing
easy to get wrong here - a walk that stops at `data` cannot see a chunk that
comes after it and would report "no loop points" without having looked.

For the null test, decode the PSP entry with `ffmpeg -i entry.at3 -f f32le -ac
2 -ar 44100`, resample the PS2 slice with `-af
aresample=resampler=soxr:precision=28:osr=44100`, cross-correlate the mid
signals for the integer lag, fit one least-squares gain, subtract, and take
`10*log10(sum(ref^2)/sum(residual^2))`. Run the 44,100 -> 48,000 -> 44,100
control first: without it the residual has no scale.

`f32le` rather than `s16le` throughout, so the comparison is not measuring its
own requantisation. `ffmpeg` logs a "non monotonically increasing dts" error per
ATRAC3+ block and none of them mean anything - see `crates/game/src/at3.rs`.

The engine reads the same populations at run time rather than from a table:
`crates/game/src/music.rs` and `Soundtrack::nearest` in
`crates/game/src/audio.rs`.

## The PSP names are recovered; the PS2 archive still has none

**Settled 2026-08-17, and only on one side.** Both PSP titles' plugin
definitions, `Data\Plugins\PI001\Definition.xml`, declare a `PI_Music` node per
track carrying the directory it lives in; `MusicManager.cpp`'s own `%s\%s` and
`music.at3` strings supply the join. Every one of Pulse's sixteen and Pure's
nineteen resolves to a real `Data.wad` entry - so `Data\Music\...` was indeed
the prefix, and the missing half was the *leaf*, which is `music.at3` on both
and not a per-track name at all.

`PS2MUSIC.WAD` is unaffected: it keys by position and stores no name, so the
length pairing above remains the only bridge between the two releases. Nothing
in this page's measurements changes.

This build reads the declaration for **Pure only**, deliberately - see
[`pure-status.md`](pure-status.md#music-recovered-by-name-and-played), `HANDOVER.md`
and `oag_title::Music::tracks`.

## Not determined
- **Whether the game streams these directly.** The PS2 disc carries Sony's
  `SCREAM.IRX` audio module, and how the streaming path is set up - including
  where the 48 kHz comes from at runtime - is a question for the executable,
  not the data. Worth one look in `SCES_547.48` when someone is in there.
- ~~**Loop points.**~~ Answered above: neither release carries one for any of
  the sixteen, and the `smpl` chunks that exist elsewhere in `Data.wad` are
  codec-delay compensation. What remains open is whether the *executable* reads
  a `smpl` chunk at all, which is a question for `BOOT.BIN` rather than the
  data.
- **Which circuit or menu plays which track.** The pairing above says which two
  entries are one recording; nothing says which recording belongs where.
  `oag-game`'s menu plays index 0 of whichever disc booted for exactly that
  reason - see `crates/game/src/audio.rs`. **A race plays a cycling playlist
  through all sixteen, in the booted disc's own order, starting one past the
  menu's own track** - this is an authored choice for this reimplementation,
  not a recovered mapping, and does not narrow the open question above: it is
  still unknown which recording, if any, the original intends for a given
  circuit.
