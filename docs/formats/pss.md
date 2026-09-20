# PS2 MPEG-2 program stream (`.PSS`) audio

**Status: understood**, for the audio. The video (a raw MPEG-2 program stream,
no header of its own) was already **understood** and unchanged by this page -
`ffprobe`/`ffmpeg` read the whole container and it transcodes into the same
AV1 movie cache every other title's video goes through, exactly as
[the format table](README.md) already described. What this page adds is
`private_stream_1`: the audio track `ffmpeg`'s own `mpegps` demuxer never
reports on this container, demuxed by
[`oag_video::pss`](../../crates/video/src/pss.rs) and read by
[`crate::movie::track::pss_audio`](../../crates/game/src/movie/track.rs).

Both intro cuts on the PS2 disc carry one:

| File | Packets | Body | Duration (`ffprobe`) |
| --- | ---: | ---: | ---: |
| `54748/DATA/MOVIES/INTRO640.PSS` | 1,795 | 7,303,168 bytes | 38.071367 s |
| `54748/DATA/MOVIES/INTRO512.PSS` | 1,794 | 7,296,000 bytes | 38.000000 s |

Reproduce the numbers with:

```sh
cargo nextest run -p oag-video --run-ignored all -E 'binary(pss_ground_truth)'
just play data/images/pulse-ps2-eu.chd --dump-audio /tmp/intro.wav \
    --screenshot /tmp/intro.png --ticks 2300 --movie-frames 30
```

## Layout

`private_stream_1` is MPEG stream id `0xbd`, the same convention `.PMF` uses
for ATRAC3+ - see [`pmf::AUDIO_STREAM_ID`](../../crates/video/src/pmf.rs). What
rides in it here is a completely different codec (below), so `oag_video::pss`
does its own pack/PES walk rather than sharing `.PMF`'s, and deliberately
never touches the video stream: a `.PSS` this size carries tens of megabytes of
it, and nothing here needs a single byte.

Every `private_stream_1` payload opens with the same 4-byte sub-header,
measured constant across every packet of both files (1,795 + 1,794 = 3,589
packets, all identical):

```text
+0x00  u8      substream id, 0xff
+0x01  u8[3]   ff a0 00 00 - the whole four bytes, never seen to vary
```

The **first** packet's payload, past that prefix, carries a small metadata
block before the audio body starts:

```text
"SShd"          4 bytes, ASCII tag
header_len      u32le, 24 in both files
format          u32le, 1 in both files - see "format 1" below
sample_rate     u32le, 48000 in both files
channels        u32le, 2 in both files
interleave      u32le, 512 in both files
(gap)           8 bytes of 0xff in both files - not assumed, the walk finds
                "SSbd" rather than hardcoding this length
"SSbd"          4 bytes, ASCII tag
body_len        u32le - the exact byte count of what follows, across every
                packet including this one
```

No packet after the first repeats `SShd` - resolving the thread's own open
question. The framing on every later packet is the 4-byte prefix alone, and
everything past it is a straight continuation of the body. Concatenating
(packet 0's payload, past its prefix and metadata block) with (every later
packet's payload, past its own prefix) reproduces `body_len` exactly, for both
files - the strongest evidence the framing above is read right rather than
guessed, and what `pss_ground_truth.rs` asserts.

## `format 1` is 16-bit PCM, not PS-ADPCM

The investigation that found the framing above first read `private_stream_1`'s
payload as PS-ADPCM, on the reasonable assumption that PS2 movie audio usually
is. It measured the framing right and the codec wrong.

Decoding `INTRO640.PSS`'s body as 16-byte PS-ADPCM blocks - with or without a
512-byte channel split - leaves under 6% of blocks in spec (predictor under 5,
shift at most 12) at any of the 16 possible byte alignments. That is what
unrelated bytes give by chance, not a four-value predictor codec: the PSP's own
sound banks, genuinely PS-ADPCM, are in spec on all but 224 of 530,916 blocks
(99.96%) - see [psp-audio.md](psp-audio.md).

Reading the same bytes as **16-bit signed little-endian PCM**, 512 bytes (256
samples) per channel at a time, gives a track that:

- runs 38.037 s against `ffprobe`'s own 38.071367 s for the container - within
  34 ms, not the 133 s a PS-ADPCM reading of the same bytes would need;
- has a mean sample-to-sample step of about 946 against roughly 21,845 for
  uniform noise at 16 bits - the actual data is over 20 times smoother;
- shows no seam at the 256-sample block boundary: the mean step across a join
  (959) is the same as the mean step everywhere else (946), which is what a
  real per-channel waveform assembled correctly looks like, not what an
  accidentally-scrambled interleave would.

All three agree on PCM and none agree on ADPCM. Both `.PSS` files on this disc
declare `format 1`; [`pss::Format::is_pcm16`](../../crates/video/src/pss.rs) is
the gate `pss_audio` checks before trusting that reading rather than assuming
it again for a file that turns out to say something else. A `format`,
`channels` or `interleave` this build has not measured is silence with a
logged reason, never a guess.

## What plays it

`pss_audio` (`crates/game/src/movie/track.rs`) de-interleaves the body into
standard left-first interleaved 16-bit PCM and hands it to `MovieAudio`, the
same type `.PMF`'s ATRAC3+ track uses - widened from "undecoded ATRAC3+ blocks
plus an `ffmpeg` decode" to "either that, or already-decoded PCM with nothing
to decode at all". `MovieAudio::decode` returns the samples straight through
for the PCM case; there is no codec, so there is nothing that can fail.
`crate::boot::load_movie_sound` plays either kind through the one route,
unchanged from the PSP's.

Verified end to end with `--dump-audio`: the boot log reports
`audio: 2 channel(s) at 48000 Hz, 16-bit PCM, no compression to decode, decoded
to 38.04s` for `INTRO640.PSS`, and the WAV it writes is not silence - mean
volume climbs from -30 dB in the first five seconds to about -18 dB by fifteen,
consistent with a logo swell rather than noise or a static tone.

## Open questions

- **What the other three prefix bytes (`a0 00 00`) mean.** Read as opaque
  because they never varied across either file measured; a third `.PSS` with a
  different value would say whether the `a0` is fixed or this disc's own.
- **What a `format` code other than 1 would be.** Neither file on this disc
  uses one, so `pss_audio` has nothing to decode against and refuses on
  principle rather than guessing PCM again.
- **HD/Fury's Bink audio still does not play.** Same shape of fix
  (`MovieAudio` needed to hold something other than ATRAC3+ blocks, which it
  now does), different container and a different blocker - see
  [bik.md](bik.md#the-audio-is-inside-the-video-file).
