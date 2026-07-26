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
| `00 00 01 BD` | `private_stream_1` | one ATRAC3+ frame per packet |

A PES packet's payload starts after two flag bytes and a length byte, plus that
many bytes of optional fields.

Video comes out as **H.264 in Annex B form**, Main profile: the intro's first
NAL units are an access unit delimiter then an SPS with `profile_idc` 77 and
`level_idc` 21. Audio is ATRAC3+, which `sceMpegAtracDecode` handles on hardware;
see [frontend video](../ghidra/functions/psp-pulse/frontend-video.md).

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

Why there are three of them is not established. Regional publisher cards is the
obvious guess and it is only a guess.

## Not determined

- **The names of eight movies**, including the dev/pub reel. Their `Movie`
  widgets are not in any XML on the disc, and the only `.PMF` strings in the
  executable are the save-icon ones plus the `.PMF` and `_US.PMF` suffixes.
- The fields at `+0x50`, `+0x60` to `+0x7c`. Reading `+0x50` as a total size does
  not work: it is 78 for the intro.
- Whether the EP map is needed for seeking. Playback here is sequential.
- The audio's exact ATRAC3+ parameters. Frames are 1103, 1410, 2017 or 2020
  bytes; the demuxer hands them over whole and nothing decodes them.
