# `.mp4`: ISO Base Media File Format, which is what Wipeout 2048's movies are

**Status: container understood, codec deliberately not implemented.** The
header is read by [`oag_video::mp4`] and validated against all 26 files in the
base package; the pictures inside are decoded out of process by `ffmpeg` into
the [AV1 cache](../architecture/adr/0008-av1-movie-cache.md), which is where
`.PMF`, `.IPF` and `.bik` video already goes.

The package is `PCSF00007` (the EU release), decrypted and extracted to
`data/extracted/vita/PCSF00007/`; everything below is read out of its one
`PSP2/data.psarc` through [`oag_assets::psarc`](psarc.md). This page is the
format half of [2048-frontend](2048-frontend.md)'s "Movies are MP4" section,
which first measured the magic and the file count and left the container
unread.

## The headline

| Question | Answer | Confidence |
| --- | --- | --- |
| What is the container? | ISO Base Media File Format, `ftyp` `mp42` on all 26 | 92 |
| Byte order? | **Big-endian**, the standard's own - no console byte-swap to carry | 92 |
| Does the header layout close? | Yes - three arithmetic invariants, 26 of 26 | 92 |
| Where is `moov`? | **Last**, after `mdat`, on all 26 - not assumed, walked | 92 |
| Does it decode? | Yes, through `ffmpeg`, losslessly into the AV1 cache | 94 |
| What is in `intro.mp4`? | A leaf on wet tarmac, held for several seconds near the start | 90 |
| Is a pure-Rust H.264 decoder available? | No permissively-licensed one exists to speak of | - |
| Does the game draw one? | Plays through `movie::open`'s existing dispatch; boot wiring is a separate lane's work | - |
| Is the audio played? | **Yes**, since 2026-09-21 - exact duration, no approximation - see [below](#the-audio-is-inside-the-container-too) | - |
| Is any of this verified under an emulator? | **No.** Nothing on this page is | - |

## Reading it yourself

```sh
OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-video --run-ignored all \
    -E 'binary(mp4_ground_truth)'                    # the 26-file survey
OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
    -E 'binary(mp4_movie_ground_truth)'               # the lossless transcode

python3 scripts/psarc.py cat \
    data/extracted/vita/PCSF00007/base/PSP2/data.psarc \
    data/Videos/intro.mp4 > /tmp/intro.mp4
ffprobe -hide_banner /tmp/intro.mp4
```

`fd` and `rg` respect `.gitignore` and return nothing under `data/`; use
`/bin/ls`, `find` or `grep`, or pass `--no-ignore`.

## The layout

```text
ftyp  FileTypeBox        major/minor brand and compatible-brands list
moov  MovieBox             container: everything below is inside it
  trak  TrackBox            one per track (intro.mp4 has two, video and
                             audio; every other file has one, video only)
    mdia  MediaBox
      mdhd  MediaHeaderBox   this track's own timescale and duration
      hdlr  HandlerBox       vide or soun - which track this is
      minf  MediaInformationBox
        stbl  SampleTableBox
          stsd  SampleDescriptionBox   codec fourcc, width/height or
                                        sample rate/channel count
          stts  TimeToSampleBox        sample count and duration, run-length
          stsz  SampleSizeBox          sample count, independently
```

Every other box - `free`, `mdat`, `edts`/`elst`, `ctts`, `stsc`, `stco`,
`stss`, `dinf`, `smhd`, `vmhd`, `udta`, `mvhd` - is skipped by size and never
read. See [`oag_video::mp4`]'s own module doc for the full field-by-field
citation of ISO/IEC 14496-12 for each box read above.

### `moov` is last, not first

**All 26 files put `moov` after `mdat`**, the way an authoring tool that
streams the picture out first and appends the index afterward would. A reader
that assumed `moov` came right after `ftyp` would be wrong on every one of
them. `intro.mp4`'s own top level: `ftyp`(24) `free`(8) `mdat`(47,179,717)
`moov`(61,352) - the sample table describing `mdat`'s bytes sits 47 MiB after
them in the file.

**`bb2048Zone8.mp4` is the one file whose middle two boxes swap**: `ftyp`
`mdat` `free` `moov`, `free` after `mdat` instead of before it. [`parse`]
finds `moov` by walking every top-level box's own declared size rather than
assuming any fixed sequence past `ftyp` - which is what catches this file
rather than silently mis-reading it, and is asserted directly in
`crates/video/tests/mp4_ground_truth.rs::bb2048zone8_swaps_free_and_mdat`.

## Three invariants, 26 of 26

Measured over all 26 `.mp4` entries in the base package's `data.psarc`, and
asserted by `every_mp4_on_the_disc_reads_its_own_header` in
`crates/video/tests/mp4_ground_truth.rs`:

1. **Every top-level box's declared size sums to the file's own length.**
   [`top_level_boxes`] cannot return successfully otherwise - a short or long
   leftover fails the next box header read rather than being silently
   dropped.
2. **The video track's `stsz` sample count equals the sum of its `stts`
   entries' sample counts.** Two different boxes counting the same samples:
   `intro.mp4`'s video track is 2,984 by both; its audio track is 4,666 by
   both.
3. **The video track's `mdhd` duration equals `frame_count * frame_delta`.**
   `intro.mp4`: 2,984 samples of 1,001 ticks each is 2,986,984, exactly what
   `mdhd` declares at a 30,000 Hz timescale (99.566 s, agreeing with
   `ffprobe`'s 99.57 s). Every one of the 26 files' `stts` boxes holds
   **exactly one entry** (constant frame rate) - [`parse`] refuses more than
   one ([`Error::VariableFrameRate`]) rather than averaging a rate nothing in
   the file states, since none of the 26 shipped files need it.

**Confidence 92**, the same band `bik.md`'s three invariants earned and for
the same reason: exact agreement across every real file, falling short of 95
because nothing has been seen to *consume* these files on a Vita - no emulator
capture exists, and the executable has not been read for the loader.

## What is on the disc

26 files, 288.6 MiB, all `ftyp` `mp42`, in four populations:

| What | Count | Size | Frames | Rate | Duration | Audio |
| --- | ---: | --- | ---: | --- | --- | --- |
| `intro.mp4` | 1 | 45.0 MiB | 2,984 | 30000/1001 | 99.57 s | mp4a, 48 kHz stereo, 4,666 frames |
| Season recaps (`2048Movie`/`2049Movie`/`2050Movie.mp4`) | 3 | ~2.0 MiB each | 150 | 30000/1001 | 5.00 s | none |
| `bb2048.mp4` | 1 | 153.6 MiB | 10,790 | 30000/1001 | 360.03 s | none |
| `bb2048Zone8.mp4` | 1 | 43.0 MiB | 3,596 | **2997/100** | 119.99 s | none |
| `shipunlocks/<Team>2048_<class>.mp4` (5 teams x 4 classes) | 20 | ~2.0-2.1 MiB each | 150 | 30000/1001 | 5.00 s | none |

`intro.mp4` and every 5-second clip (season recaps, ship unlocks) are
960x544 - the Vita's own screen resolution, confirmed square-pixel: `intro.mp4`'s
`tkhd` declares a 960x544 presentation size identical to its `avc1` coded
picture, measured 2026-09-21. **The two "best bits" reels are a different
size, 768x512**, not measured before this pass.

`bb2048Zone8.mp4`'s frame rate is its own finding worth stating plainly:
**2997/100 is not the same rational as `intro.mp4`'s 30000/1001**, even though
both round to "29.97 Hz". `2997/100 = 29.97` exactly; `30000/1001 =
29.970029970...`. [`Header::frame_rate`] carries each file's own reduced
fraction rather than a rounded float, so the two are never mistaken for one
another - see `crates/video/src/mp4/tests.rs::a_second_rational_close_to_29_97_is_not_rounded_into_the_first`.

Only `intro.mp4` carries an audio track - every recap, best-bits reel and ship
unlock clip is silent by construction (`with_audio == 1` is asserted directly
in the ground-truth test).

## `intro.mp4` is a ship-history short film, not a logo reel

Confirmed by decoding a slice of it on 2026-09-21: the shot held from roughly
frame 45 through at least frame 105 (1.5-3.5 s in) is **a single yellow leaf
lying on wet tarmac**, shallow depth of field, a tree-lined road blurred
behind it - matching [2048-frontend.md](2048-frontend.md#movies-are-mp4---read-since-2026-09-21)'s
own description from watching the file play. A second sample at 49 s shows a
close, motion-blurred shot of a magenta Feisar-liveried craft; a sample near
98 s shows the `WIPEOUT 2048` logo card. Three different kinds of shot in
three samples is itself evidence this is footage, not a static card looping -
the failure mode a broken transcode would produce is a single frozen colour,
not three different pictures.

## Why `ffmpeg` and not a decoder in this workspace

[ADR-0024](../architecture/adr/0024-in-process-codecs-and-ffmpeg-as-a-last-resort.md)
asks that this check be *made* rather than inherited from `.bik`'s answer,
because inheriting it by habit is the mistake that ADR was written about. It
was made fresh on 2026-09-21, for a different codec pair:

- **Category 1, a pure-Rust library.** No permissively-licensed H.264 decoder
  exists on crates.io to speak of: `openh264` is an FFI binding to Cisco's C
  library, not a Rust decoder, and the real Rust implementations that exist
  (in `nihav`, among others) are not published there. AAC has more
  candidates, but a second in-process decoder buys nothing while the video
  track beside it still needs `ffmpeg`.
- **Category 2, our own decoder.** Closed by the same rule that closed it for
  Bink: H.264 and AAC are published standards this project did not invent,
  and a wrong decoder for either fails as plausible-looking noise - exactly
  the failure mode the rule exists to keep out of category 2.
- **Category 3, `ffmpeg` out of process through the cache.** The same
  conclusion `.bik` reached, derived fresh rather than assumed because the
  pipe already existed.

**The container is a different question from the codec, and answering it is
not the same move as the frame-header reader ADR-0024 itself rejects.** A
pure-Rust ISOBMFF *demuxer* does exist to take instead of hand-rolling one -
`mp4parse` (MPL-2.0) and `mp4` (MIT) are both on crates.io, and `symphonia`
(already a dependency of `oag-game`, MPL-2.0, taken for Wipeout HD's MP3
music under this same ADR) ships a `symphonia-format-isomp4` demuxer under the
same licence. None of the three was taken - not on licence, all three would
clear the bar, but because [`oag-video`] is written to depend on **nothing in
the workspace**, so that every container-reading crate carries no codec
dependency in any feature combination
([ADR-0050](../architecture/adr/0050-format-crates-split-by-format-family.md)).
What [`Header`] needs is six numbers off eight box types; a general demuxer
capable of fragmented MP4, `senc`, `sidx` and every other box this project's
26 files do not use would have bought coverage nothing here exercises, not a
smaller surface than the one implemented.

## The transcode

`crates/game/src/movie/mp4.rs`. The whole file goes to `ffmpeg`, container and
all - no demux step of ours, the same shape `.bik`'s own transcode has and
unlike `.PMF` or `.IPF`.

`the_intro_cache_is_lossless` (`crates/game/tests/mp4_movie_ground_truth.rs`)
asserts what comes back out of the cache is, byte for byte, what `ffmpeg`
decodes straight from the `.mp4`, over the first 90 frames (3.003 s) - far
enough in to land inside the leaf-on-tarmac shot, so the test also writes one
decoded frame as a
picture a person can look at rather than only a byte count. 90 frames rather
than all 2,984 for the reason the module doc of that test states: this
project's own `.PMF` intro transcode - a *smaller* picture - already measures
about 80 s for its full length, and a 300-second-per-test budget ceiling
exists precisely to catch a whole-file transcode landing in a test suite by
habit.

**The cost, extrapolated**: 90 frames of 960x544 lossless AV1 (`-cpu-used 6`)
ran in 22 s wall-clock in this measurement, contended with the rest of the
gate - so **the full 2,984-frame `intro.mp4` is on the order of ten to twenty
minutes of one-time transcode**, well past what any committed test should
pay. `crate::prefetch` (if wired to reach 2048's movies) or a first `--movie`
run pays this once; every run after reuses the cache file by content hash, the
same as every other movie this project caches.

## The audio is inside the container too

Only `intro.mp4` of the 26 carries an audio track, and **it is played, since
2026-09-21**. `crate::movie::track`'s third `MovieAudioKind`, `Container`, is
the same route `bik.md`'s own "the audio is inside the video file" section
describes: rather than holding codec blocks, it holds a path to the same
`{key}.mp4` file `mp4::transcode` already writes for `ffmpeg` to read the
picture out of, and `crate::movie::container_audio::decode` shells out a
second time with `-map 0:a:0` to pull the AAC stream out of the same
container. `just play 2048`'s boot report now reads:

```text
audio: 2 channel(s) at 48000 Hz, aac, track 0 of 1, decoded to 99.54s
audio: Boot Intro Movie's own track, 99.54 s, clocking the picture
```

where it used to read `audio: Boot Intro Movie plays silently`.

**Its duration is exact, not approximated.** `oag_video::mp4::AudioTrack`
gained a `frame_delta` field - the audio `trak`'s own `stts.sample_delta`,
1,024 on `intro.mp4` - alongside the `frame_count` it already carried, so
`frame_count * frame_delta / sample_rate` is the track's real length with no
decode at all. That is a genuine difference from Bink, which carries no
equivalent field: `docs/formats/bik.md`'s own audio section explains why
[`ContainerTrack::seconds`] falls back to the *video's* own duration there
instead.

The reason this took a third `MovieAudioKind` rather than reusing ATRAC3+'s is
the same structural one `bik.md` states: `oag_game::movie::MovieAudio` was
originally shaped for a `.PMF`'s ATRAC3+-*beside*-the-video layout, and this
container's AAC track sits *inside* the same file the picture does, the same
as Bink's.

[`ContainerTrack::seconds`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/game/src/movie/container_audio.rs

## Playing one

```sh
# By path, capped so a first run is seconds rather than minutes - see
# "The transcode" above for what the uncapped cost actually is.
just play 2048 --movie 'Data\Videos\intro.mp4' --movie-frames 90
just play 2048 --movie 'Data\Videos\shipunlocks\Feisar2048_speed.mp4'
```

`--movie` reaches an `.mp4` by **path**, the same way it reaches a `.bik` in a
PSARC - `oag_assets::psarc::Archive::read_path` normalises `\`, a leading `/`
and case before matching, so any of `Data\Videos\intro.mp4`,
`data/Videos/intro.mp4` or `/data/videos/intro.mp4` resolves to the same
entry. **This is a debug path into `movie::open`, not the boot chain** -
whether `intro.mp4` plays automatically on 2048's own boot is a separate
lane's work; this page is the format only.

## See also

- [2048-frontend](2048-frontend.md#movies-are-mp4---read-since-2026-09-21) - the screen this movie plays behind, and what is and is not corroborated about the boot chain around it
- [bik](bik.md) - the other title's video container, same shape, same two-part ADR-0024 answer (container by hand, codec by `ffmpeg`)
- [psarc](psarc.md) - the archive every file on this page is read out of
- [ADR-0008](../architecture/adr/0008-av1-movie-cache.md) - the cache itself
- [ADR-0024](../architecture/adr/0024-in-process-codecs-and-ffmpeg-as-a-last-resort.md) - the rule this page applies, twice: once for the codec, once for why hand-rolling the container is not the case that rule forbids

[`oag_video::mp4`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/video/src/mp4.rs
[`oag-video`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/video/src/lib.rs
[`parse`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/video/src/mp4.rs
[`top_level_boxes`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/video/src/mp4.rs
[`Header`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/video/src/mp4.rs
[`Header::frame_rate`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/video/src/mp4.rs
[`Error::VariableFrameRate`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/video/src/mp4.rs
