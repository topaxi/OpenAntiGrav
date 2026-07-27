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

Confidence: **94** for the container layout and **92** for the contents. The
chain closing to the byte and the divisibility census are exact arithmetic; the
stereo interleave and the sample rate are statistical and cross-referential
rather than declared anywhere, which is strong but not exact. Per the
[rubric](../reverse-engineering/confidence-rubric.md) data agreement caps at 94
either way.

## Reproducing

```sh
just unpack extract data/images/pulse-ps2-eu.chd '*PS2MUSIC*' -o /tmp/ps2music
```

Then, for track 0, skip the 196-byte table and write a WAV header for 48 kHz
16-bit stereo, or:

```sh
ffmpeg -f s16le -ar 48000 -ac 2 -ss 0 -i <(tail -c +197 /tmp/ps2music/54748/PS2MUSIC.WAD) \
  -t 30 /tmp/track0.wav
```

The ground-truth test is
[`crates/formats/tests/audio_ground_truth.rs`](../../crates/formats/tests/audio_ground_truth.rs).

## Not determined

- **The names.** Entries are keyed by the same name hash as the WAD container
  and, as there, the names are not stored. `Data\Music\...` is the likely prefix
  from the PSP's [path templates](vex.md#path-templates), but nothing has been
  matched.
- **Whether the game streams these directly.** The PS2 disc carries Sony's
  `SCREAM.IRX` audio module, and how the streaming path is set up - including
  where the 48 kHz comes from at runtime - is a question for the executable,
  not the data. Worth one look in `SCES_547.48` when someone is in there.
- **Loop points.** Nothing in the container carries any, so if race music loops
  the boundaries come from elsewhere.
