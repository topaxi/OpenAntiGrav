# `.bik`: Bink video, which is what Wipeout HD's movies are

**Status: container understood, codec deliberately not implemented.** The
header is read by [`oag_formats::bik`] and validated against all 37 files on the
disc; the pictures inside are decoded out of process by `ffmpeg` into the
[AV1 cache](../architecture/adr/0008-av1-movie-cache.md), which is where `.PMF`
and `.IPF` video already goes.

The disc is `hdfury-ps3-eu-dec.iso`, serial `BCES-00664`,
[layer-1 decrypted](ps3-disc.md); everything is read through the
[`.psarc` reader](psarc.md). This page is the format half of
[hd-status](hd-status.md), whose table listed `.bik` among the extensions
nothing read.

## The headline

| Question | Answer | Confidence |
| --- | --- | --- |
| What is the container? | RAD's Bink 1, `BIKi` on **all 37** files | 92 |
| Byte order? | **Little-endian**, on a big-endian console | 92 |
| Does the header layout close? | Yes - three arithmetic invariants, 37 of 37 | 92 |
| Does it decode? | Yes, through `ffmpeg`, losslessly into the AV1 cache | 94 |
| What is in the logo reel? | The Studio Liverpool ident, 8.86 s of it | 94 |
| Is a pure-Rust decoder available? | One exists and its **licence excludes it** | - |
| Is the audio played? | **No.** Measured, not decoded - see [below](#the-audio-is-inside-the-video-file) |
| Is any of this verified under an emulator? | **No.** Nothing on this page is | - |

## Reading it yourself

```sh
just test-data                     # runs crates/game/tests/hd_movie_ground_truth.rs
python3 scripts/psarc.py cat \
    'data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC' \
    /data/fe/images/studioliverpool.bik > /tmp/logo.bik
ffprobe -hide_banner /tmp/logo.bik
```

`fd` and `rg` respect `.gitignore` and return nothing under `data/`; use
`/bin/ls`, `find` or `grep`, or pass `--no-ignore`.

## The layout

```text
+0x00  u8[3]    "BIK"
+0x03  u8       revision; 'i' on all 37 shipped files
+0x04  u32le    file length minus 8
+0x08  u32le    frame count
+0x0c  u32le    largest frame, in bytes
+0x10  u32le    frame count, again
+0x14  u32le    width
+0x18  u32le    height
+0x1c  u32le    frame rate numerator
+0x20  u32le    frame rate denominator
+0x24  u32le    video flags; zero on all 37
+0x28  u32le    audio track count, `n`
+0x2c  u32[n]   largest decoded audio frame, per track
       u32[n]   audio flags, per track: sample rate in the low half
       u32[n]   track id, per track
       u32[f+1] offset of each frame, and the end; bit 0 is a keyframe flag
```

The three per-track arrays are parallel and in that order, which is the part a
reader can get wrong without noticing - see the third invariant below.

Audio flags, in the high half of the word:

| Bit | Meaning | All six shipped tracks |
| --- | --- | --- |
| `0x4000` | 16-bit samples | set |
| `0x2000` | stereo | set |
| `0x1000` | the DCT codec rather than the RDFT one | set |

Independently confirmed: `ffprobe` reads the same tracks as
`binkaudio_dct, 48000 Hz, stereo`, which is two readings of one field agreeing.

## It is little-endian on a big-endian console

**This is the one thing about the file a reader of this repository will expect
to be wrong.** Every other HD payload is Pulse's format byte-swapped - `XXEV`
against `VEXX`, `WOtd` against `dtOW`, which is the finding
[hd-status](hd-status.md#the-headline) is built on - and `oag_formats::ByteOrder`
exists to carry that. A `.bik` is not: `+0x14` reads `80 07 00 00` and the
picture really is 1920 pixels wide.

The reason is that Bink is not Sony's or Studio Liverpool's. It is the authoring
tool's container, written once on a PC by RAD's encoder and shipped unchanged,
so `oag_formats::bik` takes no byte order and never will. Confidence **92**:
read the other way, every field on every one of the 37 files is nonsense
(`0x80070000` pixels wide), and the three invariants below could not close.

## Three invariants, 37 of 37

Measured over every `.bik` entry in all seven archives, and asserted by
`every_bik_on_the_disc_reads_its_own_header` in
`crates/game/tests/hd_movie_ground_truth.rs`:

1. **The length at `+0x04` plus 8 is the entry's real size.** 37 of 37.
2. **The count at `+0x10` repeats the count at `+0x08`.** 37 of 37.
3. **The first entry of the offset table, keyframe bit masked off, lands
   exactly where this layout says the header ends.** 37 of 37.

The third is the load-bearing one, and it is why the score is not lower. It
walks *past* the three variable-length per-track arrays, so it can only agree
when the track count, the array order and the array widths were all read right -
and 31 files have no tracks where 6 do, so it is tested against both shapes
rather than one. A reader that had the arrays in the wrong order, or read two
of them where there are three, would land somewhere else on all six.

**Confidence 92**, which is the rubric's "exact agreement across many real
files" band. It falls short of 95 because nothing has been seen to *consume*
these files on a PS3: no emulator capture exists, and the executable has not
been read for the loader.

## What is on the disc

37 files, 101.2 MiB, all `BIKi`, in three populations:

| What | Count | Size | Rate | Audio |
| --- | ---: | --- | --- | --- |
| Studio logo reels | 2 | 1920x1080 | 531 @ 59.94, 558 @ 60 | 4 tracks each |
| Circuit previews | 16 | 352x256 | 600-625 @ 59.94 | 4 of 16 |
| Front-end mode icons | 19 | 250x250, 400x400, 400x320 | 60-80 @ 60 or 29.97 | none |

The two logo reels are `/data/fe/images/studioliverpool.bik` (`DATA02`, 531
frames, 8.8588 s) and `studioliverpool_fury.bik` (`DATA00`, 558 frames, 9.30 s),
which are the two files
[hd-frontend](hd-frontend.md#the-declared-boot-chain)'s `Studio Logo` screen
names in its two skin families. **Which one a PS3 plays is still unknown**, for
the reason that page records: it depends on which of the six `skin.xml` copies
the runtime loads.

The 16 circuit previews are one per track, under
`/data/environments/<circuit>/fe/preview.bik` - ten seconds of footage each, at
a size that says they are meant for a panel in a menu rather than for the
screen. The four Zone circuits' previews are the only ones with sound.

## What is in the logo reel

**The Studio Liverpool ident**, confirmed by decoding it on 2026-08-17: a
concentric blue disc resolving out of black, four craft assembling around it,
and a final card reading `STUDIO Liverpool` with the studio's mark. 531 frames
at 59.94 Hz.

This is worth stating because it closes a question rather than opening one.
`oag_hd::frontend::names::STUDIO_LOGO_MOVIE`'s doc comment recorded that the
file was "Bink, which nothing in this project decodes, so what is inside it is
unknown" and that **the name was the whole of the evidence**. It is not any
more: the name and the content agree. Confidence **94** - the cap on a claim
resting on static reading with no emulator capture behind it.

## Why `ffmpeg` and not a decoder in this workspace

[ADR-0024](../architecture/adr/0024-in-process-codecs-and-ffmpeg-as-a-last-resort.md)
asks that this check be *made* rather than inherited from "video means
`ffmpeg`", because inheriting it by habit is the mistake that ADR was written
about. It was made on 2026-08-17:

- **Category 1, a pure-Rust library.** One candidate exists:
  `infinitier_bik_decoder` 0.0.14, which does target Bink 1 `BIKi`. It is
  **GPL-3.0-or-later** - the strong copyleft that ADR's licence bar names as the
  one kind that is not acceptable, because it would reach this project's own
  sources. `nihav` carries Bink decoders and is not published to crates.io at
  all. So the category is closed on licence, not on quality.
- **Category 2, our own decoder.** Closed by the rule. Bink is RAD's format, not
  Wipeout's; it is exactly the "published standard we did not invent" case, and
  a wrong Bink decoder fails the way a wrong MP3 decoder does - as
  plausible-looking noise rather than as an error.
- **Category 3, `ffmpeg` out of process through the cache.** Which is where that
  ADR already puts the video path, and this is the version of that sentence that
  was derived rather than assumed.

The **container header** is a different question from the codec and is read
here, exactly as [`.PMF`](pmf.md)'s and [`.IPF`](ipf.md)'s are. That is what
buys something a program stream cannot offer: a machine with no `ffmpeg` still
gets a correct frame count, size and rate off a `.bik`, so the sequencing stays
faithful and only the picture is missing.

## The transcode

`crates/game/src/movie/bink.rs`. The whole file goes to `ffmpeg`, container and
all - there is no demux step of ours, which is what this path has in common with
the PS2's raw `.PSS` program streams and not with `.PMF` or `.IPF`.

`the_bink_cache_is_lossless` asserts what comes back out of the cache is, byte
for byte, what `ffmpeg` decodes straight from the `.bik`. It also covers a case
no other movie on any disc does: the logo reel has four audio tracks *inside*
the file, so it is the first transcode input this project has handed `ffmpeg`
that holds more than a picture, and what must come back is an IVF holding video
alone.

**The cost is real and worth knowing before wiring it to a boot.** Measured
2026-08-17 on the 1080p reel, with the tree's own `-cpu-used 6` lossless flags:
60 frames in 11.0 s, so **about 98 seconds and 21 MiB of cache for all 531**,
once. That is the same order as the PSP intro's 80 s at a twenty-sixth of the
pixels. The ground-truth test caps itself at 4 frames for that reason.

## The audio is inside the video file

Six of the 37 files carry audio, and **none of it is played**. The tracks are
measured - count, rate, channels, codec, all asserted on the disc - and the
decode is not built.

The reason is structural rather than an omission. In a `.PMF` the ATRAC3+ sits
*beside* the video and `oag_game::movie::MovieAudio` is shaped for that: it
holds codec blocks and an `at3::Format` and decodes through `crate::at3`. Bink's
audio is inside the same container, so playing it needs a second route through
`ffmpeg` and a widened `Movie::audio`, which is a change to a shared type and to
`boot::load_movie_sound`'s report.

It is not built because **nothing would consume it**: HD's front end is not
wired at all ([hd-frontend](hd-frontend.md) records why), so no HD movie plays
yet and there is no playhead for a track to pace. The
[ADR-0019](../architecture/adr/0019-atrac3plus-out-of-process.md) consequence
applies whenever it is built - a movie clocked by its own sound is a playhead
two consumers pace against rather than one.

## See also

- [hd-status](hd-status.md) - the format-layer probe this continues
- [hd-frontend](hd-frontend.md) - which screen names the reel, and why the boot is not wired
- [pmf](pmf.md) and [ipf](ipf.md) - the other two containers through the same cache
- [ADR-0008](../architecture/adr/0008-av1-movie-cache.md) - the cache itself
- [ADR-0024](../architecture/adr/0024-in-process-codecs-and-ffmpeg-as-a-last-resort.md) - the rule this page applies

[`oag_formats::bik`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/formats/src/bik.rs
