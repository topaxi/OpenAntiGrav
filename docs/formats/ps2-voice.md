# PS2 pre-race voice archive (`PRERACE.WAD`)

`54748/PRERACE.WAD`, 89,325,308 bytes, the third-largest file on the PS2 disc
after `PS2MUSIC.WAD` and `WADS2.WAD`. It holds 32 clips of **44,100 Hz** 16-bit
dual-mono PCM speech - the pre-race dialogue, and the same 32 master recordings
the PSP disc ships as ATRAC3plus.

Despite the extension it is **not** the WAD container, for exactly the reason
[`PS2MUSIC.WAD`](ps2-audio.md) is not: the first word is an entry count, not a
version. `oag-wad` reads it as one and fails - *"unknown WAD version 32"* here,
*"unknown WAD version 16"* for `PS2MUSIC.WAD`. The file is fine; the reader is
looking at the wrong field.

That error now names the alternative and points at `crate::ps2_music` instead of
leaving the reader to hunt for a WAD variant that does not exist. **It is still
an error**: `oag-wad` cannot list or extract a count-first archive, and teaching
`Archive` to open one is a separate piece of work. Use the format module
directly, as the ground-truth test does.

## Layout

Byte-for-byte the [PS2 music](ps2-audio.md) container, and
[`crate::ps2_music`](../../crates/formats/src/ps2_music.rs) parses it with **no
changes at all**:

```text
+0x00  u32  entry_count            32
+0x04  entry[entry_count], 12 bytes each

entry:
  +0x00  u32  name_hash
  +0x04  u32  size
  +0x08  u32  offset               from the start of the file
```

Two arithmetic checks pin it, both asserted by the ground-truth test rather than
eyeballed:

- Entry 0 begins at **388** = `4 + 32 * 12`, so the directory is exactly full -
  no slack, no reserved fields.
- The offset chain **closes to the byte** on 89,325,308, the file's exact
  length. All 32 entries, no gaps and no padding.

Every entry size is divisible by 4, the same 16-bit-stereo frame the music
archive uses.

## The payload is dual-mono

This is what separates it from the music archive. The two channels are
**byte-identical** - a mono recording written into a stereo stream:

```text
02aea540  14 e5 14 e5  95 d7 95 d7  fd e1 fd e1  76 f7 76 f7
02aea550  57 0c 57 0c  4f 17 4f 17  04 16 04 16  50 10 50 10
```

Measured over a 2 MiB probe a third of the way into each clip, the fraction of
frames whose left and right samples are equal is **1.0000 for all 32 clips** -
not "highly correlated", identical. That is why the music test's
`correlation > 0.3` check would be meaningless here: it is 1.0 by construction,
and asserting dual-mono instead is what keeps the two archives from being
conflated.

It also explains the disc sniff's low entropy reading of 0.084: the first 64 KiB
of the file is near-silence, the lead-in to a voice-over.

## Sizes

Clip sizes run 2,271,168 to 3,376,824 bytes (567,792 to 844,206 frames),
clustered tightly - consistent with 32 takes of one line each. At the 44,100 Hz
established below that is 12.88 to 19.14 seconds.

## The sample rate is 44,100 Hz

Two independent measurements, one exact and one statistical.

### Every clip is padded with exactly 44,100 silent samples

Each of the 32 clips opens and closes with a run of literal zero samples. The
**shortest leading run across all 32 is 44,100 samples exactly**, and so is the
shortest trailing run:

| | Leading zero run | Trailing zero run |
| --- | ---: | ---: |
| Minimum over the 32 clips | **44,100** | **44,100** |
| Maximum | 44,111 | 44,180 |
| Clips landing on 44,100 exactly | **30 of 32** | 19 of 32 |

The pad is one second of digital silence at each end, and the handful of extra
samples are simply where the speech itself starts or ends on a zero crossing.
A round 44,100 is not a number an authoring tool picks for any other reason; at
48,000 Hz the same pad would be 0.919 s, and the tool would have had to choose
44,100 samples deliberately. This is the same class of evidence as the offset
chain closing to the byte: exact arithmetic, repeated across every file.

### The PSP disc carries the same recordings, and only 44,100 aligns them

`PSP_GAME/USRDIR/Data.wad` entries **867-898** are 32 ATRAC3plus streams. That
range is not a guess about where the population starts and stops - it is where
the RIFF headers say it does:

| Entries | What they are |
| --- | --- |
| 866 and below | not RIFF at all; sound banks, first bytes `03 00 00 00 02 00 00 00` |
| **867-898** | RIFF/WAVE, format tag `0xfffe`, ATRAC3plus subformat GUID, **1 channel, 44,100 Hz**, `nAvgBytesPerSec` 12,058, `nBlockAlign` 560. `fact` sample counts 590,696 to 853,582, i.e. 13.394 to 19.356 s |
| 899-930 | RIFF, same codec, still mono, but **half the bitrate** - `nAvgBytesPerSec` 6,029, `nBlockAlign` 280 - and 2.30 to 3.23 s. Their `fact` counts repeat exactly between 899-914 and 915-930, so this is 16 clips stored twice, not 32 clips |
| 931 and above | not RIFF |

Every RIFF stream in the whole archive is ATRAC3plus at 44,100 Hz, and there are
three populations, not two: the 32 above, the 32 half-bitrate ones, and **28
stereo streams at the same 12,058 B/s**, scattered rather than contiguous. So
bitrate alone does not isolate the pre-race set - it takes the channel count as
well, and a size filter would have got it wrong in either direction. With both
applied the set is exactly 32 and exactly contiguous, so the 32-against-32 count
is a real population boundary and not an artefact of where somebody drew a line.

The **USA UMD agrees**, which matters because it is a build of the same game by
different people on a different date. Its `Data.wad` has 1,142 entries against
EU's 1,138, so the run sits one index later at **868-899** - but all three
population counts are identical (32, 32, 28) and the 32 entries' **name hashes
are the same 32 values in the same order**, starting `16e968ce` and ending
`f5a66cc7`. The set is a fixed part of the game, not a regional edit.

Both sides were then reduced to one mono signal - the PS2 side by taking channel
0 of the dual-mono frames, the PSP side by decoding with `ffmpeg` - and every
PS2 clip was cross-correlated against every PSP clip, band-limited to 0-5,512 Hz
and normalised over the whole clip. The PS2 signal was run twice: once read as
44,100 Hz (no resampling needed, both sides are then the same rate) and once
read as 48,000 Hz and resampled to 44,100.

| PS2 read as | Distinct partners | Best match per row | Largest competing value in that row |
| --- | ---: | --- | ---: |
| 48,000 Hz | 17 of 32 | 0.040 to 0.067, mean 0.049 | up to 0.053 |
| **44,100 Hz** | **32 of 32** | **+0.4925 to +0.9848, mean +0.918** | at most 0.065 |

The 44,100 matrix with its 32 matched cells masked out gives the noise floor:
mean `|r|` **0.039**, 99th percentile 0.056, largest single value 0.065. The
48,000 run's best-per-row mean of 0.049 sits exactly where the best of 32 draws
from that distribution would - it is not a weak signal, it is the maximum of a
sample of noise. An 8.8% rate error accumulates 1.3 s of drift over a 15 s clip,
which destroys any correlation, and that is what happens. At 44,100 Hz the
assignment is instead a clean bijection, with a separation of at least 7.6x and
usually 20x between a clip's partner and its nearest rival.

## The two discs carry the same recordings

Aligning each pair at its correlation peak and then re-correlating in sliding
0.5 s windows shows a **constant** offset, not a drifting one, and a local
Pearson correlation of `+0.999` throughout. Over the longest constant-offset run
of each pair, on the non-silent samples:

| | Value |
| --- | --- |
| Correlation, worst pair | **+0.99810** |
| Correlation, mean over 32 pairs | **+0.99931** |
| Correlation, best pair | +0.99966 |
| Residual signal-to-noise, per pair | 24.2 to 31.7 dB, mean 28.9 dB |

That is stronger than the +0.98 that settled the music, and the residual tells
you why the numbers are not 1.0: 29 dB is a lossy codec's quantisation noise.
What that rules out is the PS2 PCM being a *decode* of the PSP stream - a decode
would leave no residual at all. It does **not** rule out the PSP stream having
been encoded from this very PCM, which would produce the same 29 dB; only the
six clips with differing edits below say anything against that, and only for
those six. Either way the recording is one recording.

The alignment offset varies per clip - the PSP stream leads by 0.883 to 1.789 s,
because the PS2 side uses a fixed 1 s pad and the PSP side does not.

### Six clips have one edit difference

26 of the 32 pairs hold a single constant offset from first sample to last. The
other six step exactly **once**, and the correlation stays above `+0.999` on
both sides of the step:

| `PRERACE` clip | Step | Splice at | RMS of the extra PS2 material, against the clip's own |
| ---: | ---: | ---: | ---: |
| 0 | -284 ms | 11.03 s | 11.7% |
| 5 | -271 ms | 11.22 s | 0.2% |
| 7 | -401 ms | 8.76 s | **26.6%** |
| 22 | -157 ms | 9.97 s | 3.4% |
| 23 | -290 ms | 8.03 s | 3.4% |
| 26 | -213 ms | 11.49 s | 1.2% |

The sign is the same in all six: the PS2 version runs 157 to 401 ms **longer**
across that point than the PSP version, so it is the PS2 side that carries
material the PSP side does not. In five of the six that material is quiet
relative to the line around it - a pause the PSP master had shortened. In clip 7
it is not: 401 ms at a quarter of the clip's own RMS is audible content, and
that one is a genuinely different edit rather than a different pause length. The
last column is measured over exactly the `|step|` samples beginning at the
splice, and it is worth stating that the splice itself is located only to within
the 256-sample search step, so the percentages carry a little of the
neighbouring speech.

Whole-clip correlation for these six reads +0.49 to +0.78 as a result, which is
what "same recording, different edit" looks like and why the whole-clip figure
alone would have understated the result. Everywhere else in those same six
clips the local correlation is `+0.999`, the same as the other 26.

## Why duration matching could not have settled this

[`ps2-audio.md`](ps2-audio.md) established the music rate by matching durations:
at 48 kHz all 16 PS2 tracks matched a distinct PSP track to within 10 ms. That
method **fails here**, and the failure is structural rather than a matter of
precision:

- The 32 clips span roughly 6 seconds, so consecutive durations are about 0.19 s
  apart. Nearest-neighbour matching on durations that close is noise.
- Reading the true decoded sample count out of each PSP stream's `fact` chunk
  rather than trusting a decoder's reported duration tightens the mean gap from
  0.305 s to 0.085 s. It does not help: the matching still collapses to **19
  distinct partners of 32**.
- Against the pairing the correlation actually establishes, duration
  nearest-neighbour agrees on **9 of 32**. It would have produced a mostly wrong
  answer with a plausible-looking mean error.

Under the true pairing the PSP stream is on average 0.208 s longer than the PS2
clip, ranging from 0.385 s shorter to 0.734 s longer - the padding and edit
differences above, not a rate discrepancy.

## The two discs order the clips differently

The bijection is not the identity and not a rotation. `PRERACE` index to
`Data.wad` entry:

```text
 0 -> 877    8 -> 898   16 -> 874   24 -> 883
 1 -> 888    9 -> 867   17 -> 875   25 -> 884
 2 -> 892   10 -> 868   18 -> 876   26 -> 885
 3 -> 893   11 -> 869   19 -> 878   27 -> 886
 4 -> 894   12 -> 870   20 -> 879   28 -> 887
 5 -> 895   13 -> 871   21 -> 880   29 -> 889
 6 -> 896   14 -> 872   22 -> 881   30 -> 890
 7 -> 897   15 -> 873   23 -> 882   31 -> 891
```

Both archives are hash-keyed, so neither order is meaningful on its own; the
table is recorded because it is the only thing that lets a PSP-side name, once
one is mined, be carried to the PS2 clip it belongs to.

## Confidence

**92** for the container: the directory closes to the byte on the file length,
which is the same evidence that carries `PS2MUSIC.WAD` at 94, one step down
because the frame size here is inherited from that archive rather than
independently established (the divisible-by-8 census that pins a 4-byte frame
for the music has not been repeated here).

**90** for dual-mono 16-bit PCM: 32 of 32 clips at exactly 1.0000, which is a
measurement rather than an inference.

**94** for **44,100 Hz**, the ceiling the
[rubric](../reverse-engineering/confidence-rubric.md) allows for agreement with
shipped data. Two independent legs: an exact arithmetic invariant (a 44,100
sample pad, minimum over all 32 clips, at both ends of every clip) and
corroboration in a second binary (a 32-of-32 correlation bijection against the
PSP disc that exists at 44,100 and does not exist at 48,000). Not 95+ because
nothing has been observed at runtime - no trace shows `SCES_547.48` programming
a rate, and the declared constant in the executable has not been found.

**94** for **the PS2 and PSP clips being the same 32 recordings**: +0.99931 mean
local correlation with a 28.9 dB residual, against a 0.065 ceiling for any
mismatched pair. Same measurement class, same cap, same reason for the cap.

Ground truth:
[`crates/formats/tests/audio_ground_truth.rs`](../../crates/formats/tests/audio_ground_truth.rs),
`the_ps2_prerace_archive_chains_exactly_and_holds_dual_mono`,
`the_prerace_clips_are_padded_to_one_second_at_44100` and
`the_psp_prerace_streams_are_32_mono_atrac3plus_at_44100`.

The correlation itself is **not** locked in by a test, because re-running it
needs an ATRAC3plus decoder that this workspace does not have. What the tests do
lock in is both of its endpoints: the 44,100 sample pad on the PS2 side, and the
32 mono 44,100 Hz ATRAC3plus streams on the PSP side - on **both** PSP builds,
asserting that their name hashes agree entry for entry.

## Reproducing the correlation

```sh
just unpack extract data/images/pulse-ps2-eu.chd '*PRERACE*' -o data/cache/voice/ps2
just unpack extract data/images/pulse-psp-eu.chd '*Data.wad'  -o data/cache/voice/psp
```

Slice `PRERACE.WAD` with the 12-byte directory above, keep every other 16-bit
sample (the channels are identical), and write a header for 44,100 Hz mono. For
the PSP side, slice `Data.wad` entries 867-898 out with the 16-byte WAD
directory and hand each to `ffmpeg -i clip.at3 -f s16le -ac 1 -ar 44100 clip.raw`
- ffmpeg n8.1.2 decodes ATRAC3plus without help. Then cross-correlate the two
sets. Use `data/cache/`, never `/tmp`.

## Not determined

- **What the executable does with the rate.** 44,100 is established from the
  data, not read out of `SCES_547.48`. An immediate `44100` near the
  `SCREAM.IRX` stream setup, or near the code that looks up the `PRERACE` name
  hashes, would show the engine agreeing with the data rather than only the data
  agreeing with itself. Worth one look when someone is in that binary. Note that
  finding the constant does **not** on its own lift the score past 94 - the
  rubric reserves 95+ for a runtime trace, so what closes this is a breakpoint
  in PCSX2 catching the rate as it is programmed. The second-binary half of that
  requirement is already met by the PSP disc.
- **The clip names.** Hash-keyed like every other archive here, and unmined. The
  executable's `Data\Sound\PreRaceDialogue\%d_Track.at3` is the PSP-side
  equivalent path and may be the way in. The bijection above means one mined
  name resolves two entries.
- **What `Data.wad` 899-930 are.** 32 entries, 16 distinct clips stored twice,
  2.3-3.2 s each at half the bitrate of the pre-race set. Short enough to be
  stingers or per-pilot barks rather than dialogue, and they have no PS2
  counterpart in `PRERACE.WAD`. Not investigated.
- **Whether the game streams these directly**, and through what - the PS2 disc
  ships `IOP/SCREAM.IRX`, the same open question `ps2-audio.md` records.
