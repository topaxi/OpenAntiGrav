# PSP movie files (`.PMF`)

**Status: understood.** Header and demuxer implemented in
[`oag-formats::pmf`](../../crates/formats/src/pmf.rs) and validated against all
17 movies on the PSP Pulse disc.

Decoding is deliberately **not** implemented here. See
[ADR-0004](../architecture/adr/0004-asset-pipeline.md) and
[frontend boot](../architecture/frontend-boot.md) for the conversion route.

## Layout

A PMF is a 2048-byte PSMF header followed by an MPEG program stream.

```text
+0x00  u8[4]   "PSMF"
+0x04  u8[4]   version, ASCII: "0012" or "0014"
+0x08  u32be   stream offset, 0x800 in every shipped file
+0x0c  u32be   stream size
+0x54  u48be   presentation start time, 90 kHz ticks
+0x5a  u48be   presentation end time, 90 kHz ticks
+0x80  u16be   stream count
+0x82  16 bytes per stream descriptor
```

Note the endianness: the header is **big-endian** while everything else in these
games is little-endian. That is Sony's format, not the game's.

A stream descriptor is keyed by its MPEG stream id at `+0x00`.

```text
video (0xe0)  +0x04 u32be EP map offset    +0x08 u32be EP map entries
              +0x0c u8    width  / 16      +0x0d u8    height / 16
audio (0xbd)  +0x0e u8    channels         +0x0f u8    frequency code
```

Frequency code `2` means 44,100 Hz. No other code appears, so
[`AudioStream::frequency_hz`](../../crates/formats/src/pmf.rs) returns `None` for
anything else rather than guessing.

The entry-point map is at the offset the video descriptor gives, 10 bytes per
entry: an index byte, a picture-offset byte, a `u32be` presentation timestamp and
a `u32be` byte offset. It is parsed as a count only; nothing needs it yet, since
playback is sequential.

## Program stream

Standard MPEG-1/2 program stream, in 2048-byte packs.

| Start code | Meaning | Handling |
| --- | --- | --- |
| `00 00 01 BA` | pack header | 14 bytes plus a stuffing count in the low 3 bits of `+0x0d` |
| `00 00 01 BB` | system header | length-prefixed, skipped |
| `00 00 01 BF` | `private_stream_2` | length-prefixed, skipped |
| `00 00 01 BE` | padding | length-prefixed, skipped |
| `00 00 01 E0` | video PES | payload appended to the video elementary stream |
| `00 00 01 BD` | `private_stream_1` | a slice of the ATRAC3+ frame stream, behind two more layers of framing - see [audio framing](#audio-framing) |

A PES packet's payload starts after two flag bytes and a length byte, plus that
many bytes of optional fields.

Video comes out as **H.264 in Annex B form**, Main profile: the intro's first
NAL units are an access unit delimiter then an SPS with `profile_idc` 77 and
`level_idc` 21. Audio is ATRAC3+, which `sceMpegAtracDecode` handles on hardware;
see [frontend video](../ghidra/functions/psp-pulse-usa/frontend-video.md).

## Audio framing

**Confidence 92.** An arithmetic invariant comes out exactly on every movie with
a track, and a second, independent reading of the same field agrees with the
disc's own `.at3` files. Short of 95 because nothing has been checked against a
runtime trace or a second binary.

A `private_stream_1` payload is **not** an ATRAC3+ frame. Two layers sit on top,
and both are the container's rather than the codec's.

**Four bytes per PES payload.** The first two are zero on every packet of every
movie; the third and fourth are a big-endian offset, counted from the end of
these four, to the first frame that *starts* inside the packet. The bytes before
it are the tail of the frame the previous packet began, so a payload is a slice
of a byte stream and frames straddle packets freely.

**Eight bytes per frame.**

```text
+0x00  u8[2]  0f d0            sync word
+0x02  u16be  config word      see below
+0x04  u8[4]  00 00 00 00
+0x08         the ATRAC3+ block, `block_align` bytes
```

The config word is `((8 + channels) << 10) | (block_align / 8 - 1)`: the low ten
bits are the block size in eight-byte groups less one, and the top six are a
channel code, 9 for mono and 10 for stereo. **The disc's RIFF-wrapped `.at3`
files carry this same word** in the third and fourth bytes of their `fmt `
chunk's codec extra data, which is what turns a fitted formula into a
corroborated one. That is also what the wrapper written for `ffmpeg` fills the
field with, rather than the one constant it used to copy off `SND0.AT3` - see
[ADR-0019](../architecture/adr/0019-atrac3plus-out-of-process.md).

| `block_align` | channels | Where | Config word |
| --- | --- | --- | --- |
| 280 | 1 | 32 `Data.wad` entries | `0x2422` |
| 560 | 1 | 32 `Data.wad` entries | `0x2445` |
| 560 | 2 | 28 `Data.wad` entries, and 7 movies | `0x2845` |
| 744 | 2 | `Intro.PMF` and `Tutorial.PMF` | `0x285c` |

### What pins it down

- **`Intro.PMF`'s 323 payload pointers all land on a sync word**, 323 out of
  323. They are pointers rather than counters: packet 0 holds two whole 752-byte
  frames and 509 bytes of a third, and packet 1's pointer is exactly the 243
  that complete it.
- **Stripping both layers leaves a whole number of blocks with nothing over.**
  `Intro.PMF`'s stream is 650,480 bytes, its frames are 752 apart, and
  `650,480 / 752` is 865 exactly. The same holds for all nine movies with a
  track, at both block sizes.
- **`ffmpeg` decodes the result and rejects the alternatives.** 865 blocks of
  744 bytes decode to 1,771,520 samples - 865 x 2048, again exact - at
  40.17 s against the header's declared 40.04 s, the difference being the
  padding of a whole final block. Leaving the eight-byte header on and calling
  the block 752 fails with "frame data doesn't match channel configuration" on
  every frame rather than decoding noise, and stripping only the two-byte sync
  word fails the same way.
- **The payload lines up with a `.at3`'s.** `PSP_GAME/SND0.AT3`'s data chunk
  begins `3a 63 8f 80` and contains no `0f d0` anywhere in its 25,760 bytes;
  every movie frame's byte at `+0x08` is likewise `3a`. So the sync word is the
  `.PMF`'s framing and a RIFF `.at3` has none.

Reproduce with `just play <image> --screenshot out.png --dump-audio out.wav`,
which reports the block count and size it measured.

## What pins the layout down

Three arithmetic checks, from opposite ends of the file, all of which a wrong
reading fails. They run as
[a ground-truth test](../../crates/assets/tests/pmf_ground_truth.rs) over all 17
movies.

1. **`stream_offset + stream_size` equals the file length, exactly.** 17 of 17.
   For the intro that is 2048 + 5,734,400 = 5,736,448 bytes.
2. **The program-stream walk accounts for every byte.** Zero stray bytes in all
   17. A demuxer that mis-sizes one packet loses sync and the stray count
   explodes.
3. **The access-unit count matches the duration.** Counting NAL type 9 in the
   demuxed video gives 1200 for the intro; the header's presentation timestamps
   give 3,693,600 - 90,000 = 3,603,600 ticks, which at 30000/1001 Hz is 1200
   frames. All 17 agree to within one frame.

Check 3 is the strong one: the frame count and the duration are produced by
completely different code paths over completely different parts of the file.

For the intro, the slice types corroborate it independently: 1129 P-slices plus
71 IDR slices is 1200, matching the 1200 access unit delimiters.

Confidence **95**. Everything above is measured against real files. The one
inference is that the PSP presents at 30000/1001 Hz rather than exactly 30, which
is what makes check 3 land on 1200 instead of 1198.

**Validated against two discs**: `pulse-psp-usa` and `pure-psp-usa`. Pure's
archives hold 14 movies, and `stream_offset == 0x800` and
`stream_offset + stream_size == file length` hold exactly on 14/14, each with
two streams keyed `0xe0` and `0xbd`. Pure ships version string `"0012"` only,
where Pulse ships both `"0012"` and `"0014"` - so the version field varies
within a title, not between titles, and neither reading of it changes the
layout. See the [Pure probe](pure-status.md).

## The movies on the PSP Pulse disc

All 17 are entries in `Data.wad`, `PSMF0012` or `PSMF0014`, 480x272 for the
full-screen ones and 144x80 for the save-data icons.

| Entry | Frames | Duration | Streams | Identified as |
| --- | ---: | ---: | ---: | --- |
| `Data\Movies\Intro.PMF` | 1200 | 40.04 s | video + audio | The intro, played by `LogoFMV` and `Play Intro` |
| `Data\Movies\Backdrop.PMF` | 270 | 9.01 s | video | The looping menu backdrop |
| `Data\Movies\Tutorial.PMF` | 1200 | 40.04 s | video + audio | Named by hash search; not referenced by any XML found |
| three unnamed | 260 | 8.68 s | video + audio | **The dev/pub reel**; see below |
| two unnamed | 48, 32 | 1.60 s, 1.07 s | video + audio | Short stings |
| three unnamed | 2847-2904 | 95-97 s | video + audio | Long reels, probably extras |
| `Data\FE\Profile\*_movie.pmf` and `Icon1.PMF` | - | ~4-5 s | video | Save-data icons, 144x80 |

Names were recovered by hashing candidates against the archive directory; see
[the WAD name hash](wad.md#the-name-hash). Eight of the 17 are still unnamed,
which does not stop them being read, since the game itself looks entries up by
hash.

### The 260-frame reels are the intro state's reel

`0x088d7e1c` pauses at frame 144, again at 231, and sets its finish flag at 260.
Three movies on the disc are **exactly 260 frames long**. That is what turns
those three numbers from constants in a decompiled function into frame numbers of
a real reel, and it is asserted by
[a test](../../crates/assets/tests/pmf_ground_truth.rs).

Why there are three of them is settled: they are **regional cuts**. Decoded, all
three show the same two cards and differ in one line - `SONY COMPUTER
ENTERTAINMENT EUROPE / INC. / AMERICA PRESENTS` at frame 144, then
`A STUDIO LIVERPOOL GAME` at 231 - which lifted the "these are the dev/pub logo
cards" reading from **82** to **95**, from the picture rather than from the
constants. What is *not* settled is which cut runs when: the same three ship
byte-identically on *Wipeout Pure*'s USA disc, so the disc's region cannot be
what picks one, and the mechanism is unread. Nor is it settled what plays them
at all - the state whose counters they fit is never entered during boot. See
[frontend boot](../architecture/frontend-boot.md#the-devpub-reel) and
[frontend video](../ghidra/functions/psp-pulse-usa/frontend-video.md).

## Not determined

- **The names of eight movies**, including the dev/pub reel. Their `Movie`
  widgets are not in any XML on the disc, and the only `.PMF` strings in the
  executable are the save-icon ones plus the `.PMF` and `_US.PMF` suffixes.
- The fields at `+0x50`, `+0x60` to `+0x7c`. Reading `+0x50` as a total size does
  not work: it is 78 for the intro.
- Whether the EP map is needed for seeking. Playback here is sequential.
- What the four zero bytes at `+0x04` of a frame header are for. They are zero on
  every frame of every movie, so nothing distinguishes a reserved field from one
  this disc never uses.
- Whether a frequency code other than 2 (44,100 Hz) exists. Only code 2 appears,
  and anything else is reported as unknown rather than guessed at.
