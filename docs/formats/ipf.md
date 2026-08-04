# PS2 IPU video (`.IPF`)

**Status: understood.** Container parsed in
[`oag-formats::ipf`](../../crates/formats/src/ipf.rs) and validated against both
files on the PS2 disc; the video inside is IPU - the PS2 Image Processing Unit's
intra-only MPEG-2 - and is [transcoded out of process](../architecture/adr/0008-av1-movie-cache.md)
rather than decoded here, exactly like `.PMF` and `.PSS` video.

This is the format the PS2 release's **looping menu backdrop** ships in. There
are two files, and they are the PAL and NTSC cuts of one nine-second loop:

| File | LBA | Size | Picture | Frames | Rate | Slot stride |
| --- | ---: | ---: | --- | ---: | --- | ---: |
| `54748/DATA/MOVIES/BG512.IPF` | 298 | 6,548,432 | 512x512 | 225 | 25/1 | 29,104 |
| `54748/DATA/MOVIES/BG640.IPF` | 3,496 | 8,899,232 | 640x448 | 270 | 30000/1001 | 32,960 |

Both sit loose in the ISO filesystem, in no WAD, beside the two `.PSS` intro
cuts of exactly the same two sizes. See
[the PS2 disc layout](../ps2/pulse-disc-layout.md).

Reproduce the numbers with:

```sh
cargo nextest run -p oag-game --run-ignored all -E 'binary(ipf_ground_truth)'
just play data/images/pulse-ps2-eu.chd --movie 'Data\Movies\Backdrop.ipf' \
    --screenshot /tmp/ps2-backdrop.png --ticks 90
```

## Layout

A 32-byte file header, then one fixed-size slot per frame to the end of the
file. Everything is little-endian.

```text
+0x00  u8[4]   "IPUF"
+0x04  u16le   width
+0x06  u16le   height
+0x08  u32le   frame count
+0x0c  u32le   frame stride, bytes per slot
+0x10  u32le   16 in both files - read as an alignment, see below
+0x14  u32[3]  zero
+0x20          the first slot
```

Each slot is `frame stride` bytes:

```text
+0x00  u32le   payload length
+0x04  60      zero
+0x40          payload, then zero padding out to the stride
```

The payload is one IPU picture. It opens with a four-byte picture header and
closes on `00 00 01 B0`, the IPU end-of-frame marker.

## What pins the layout down

Four checks, run over every frame of both files by
[`ipf_ground_truth.rs`](../../crates/game/tests/ipf_ground_truth.rs).

1. **`32 + frames * stride` is the file length, exactly.** 6,548,432 and
   8,899,232, to the byte. This is the check that settles the header size,
   which is otherwise genuinely ambiguous: bytes `0x24`-`0x5f` are zero in both
   files, so a 96-byte flat header with no slot headers reads just as
   plausibly - and misses by 64 bytes at the end of the file, in both. The
   32-byte reading has no residue at all.
2. **The `u32` at the head of each slot equals the extent of that slot's
   non-zero bytes**, on all 225 and all 270 frames. Not approximately: the byte
   at `payload - 1` is non-zero and every byte from `payload` to the end of the
   slot is zero, 495 times out of 495. A field that is not the length does not
   do that.
3. **The stride is the longest payload plus the 64-byte slot header, rounded up
   to the value at `+0x10`.** `29,026 + 64 = 29,090 -> 29,104` and
   `32,895 + 64 = 32,959 -> 32,960`, both exact at 16. That is what the fixed
   slots are *for*: a seek to frame *n* is a multiply, which is what a streamer
   reading off a DVD wants.
4. **`ffmpeg`'s own IPU demuxer, which knows nothing about the slots, cuts the
   concatenated payloads at exactly the boundaries the slot headers declared.**
   225 for 225 and 270 for 270 packet sizes, identical lists. This is the
   strongest of the four, because it is two entirely independent readings of the
   same bytes - one from the container's length fields, one from the `00 00 01
   B0` markers inside the bitstream - agreeing 495 times.

Confidence **92** on the container. Held under 95 because the field at `+0x10`
is `16` in both files, so "alignment" is inferred from check 3 rather than seen
varying; a third file with a different value would settle it. The twelve zero
bytes at `+0x14` are likewise unexplained rather than known to be padding.

## The payload is IPU, and that is what decodes it

The bitstream is not an MPEG program stream and has no start-code framing at the
top level, so `ffmpeg` cannot read an `.IPF` as it reads an `.PSS`. What it can
read is Sony's own `.IPU`: `ffmpeg` carries both an `ipu` demuxer and an `ipu`
decoder, and the payloads go straight through them once the slot framing is
removed and its 16-byte `ipum` header is put on.

That wrapper is `ffmpeg`'s, not Wipeout's, which is why it is built in
[`crate::movie::ipum`](../../crates/game/src/movie.rs) rather than in
`oag-formats`:

```text
+0x00  u8[4]   "ipum"
+0x04  u32le   payload length
+0x08  u16le   width
+0x0a  u16le   height
+0x0c  u32le   frame count
+0x10          the bitstream
```

**The width and height are at `+0x08`, not `+0x04`.** That was found by
experiment - four bytes are skipped after the magic - and a wrapper built the
obvious way decodes as `0x0` and fails with `Picture size 0x0 is invalid`.

Both files then decode end to end with no errors and no dropped frames:
225 of 225 and 270 of 270, at the dimensions the container declares. The cache
for the PAL cut is 3.34 MiB of lossless AV1 against 88.5 MiB of raw
`yuv420p`, a 26.5x saving in line with
[ADR-0008](../architecture/adr/0008-av1-movie-cache.md)'s measurements.

The four-byte picture header at the head of each payload is **not** decoded
here. Its first two bytes are `00 43` on every frame of both files; the other
two vary across a small set (`f0 08` through `f1 10`). `ffmpeg`'s decoder reads
the first `u16` as a flags word carrying the intra DC precision, the scan order
and the VLC table selection, which is consistent with it being constant for a
single encode, but nothing here has confirmed that reading. Confidence **40**,
recorded as an observation rather than a layout.

## The frame rate is inferred, not read

**An `IPUF` declares no frame rate.** Neither does the IPU bitstream: on real
hardware the picture is handed to the GS at the display's own rate.

The pairing comes from the executable.
[`Movie_ResolveSourcePath`](../ghidra/functions/ps2-pulse-eu/movie-paths.md)
(`0x0019b168`, confidence 90) rewrites the front-end XML's
`Data\Movies\Backdrop.ipf` to `Data\Movies\bg512.ipf` or `bg640.ipf` on the
**same global** (`0x0027a85c`) and with the same sense as it picks
`Intro512.pss` against `Intro640.pss` - and those two do declare their rates,
measured with `ffprobe` at 25/1 and 30000/1001. So `bg512` is the PAL cut and
`bg640` the NTSC one.

It checks out arithmetically as well, which is the part that would catch a
swapped pairing: 225 frames at 25 Hz is 9.000 s and 270 at 30000/1001 is
9.009 s, so the two files are the same loop. Swapped, one would be 10.8 s and
the other 7.5 s.

Confidence **85**, inherited from the PAL/NTSC reading of `0x0027a85c` rather
than measured here. Nothing visible depends on it being exact: the backdrop
loops.

Display aspect is taken the same way - `4:3`, which both `.PSS` cuts declare in
their own sequence headers, neither `512x512` nor `640x448` being 4:3 as a pixel
grid. The PS2 front end draws its `Movie` widget over a `640x448` black `Image`
and declares no size on the widget itself, so on real hardware the video and the
frame buffer were the same rectangle; see
[the disc layout](../ps2/pulse-disc-layout.md) for why that is a deliberate
anamorphic encode rather than a stretch to undo.

## Where the name comes from

The front-end XML never names these files. Its `FE Screen` `Movie` widget says
`src="Data\Movies\Backdrop.ipf"` - with the extension, unlike the PSP, which
gives an extensionless `src` and has `.PMF` appended. `Movie_ResolveSourcePath`
maps that name onto the region's file, so:

| Asked for | PAL | NTSC |
| --- | --- | --- |
| `Data\Movies\Backdrop.ipf` | `Data\Movies\bg512.ipf` | `Data\Movies\bg640.ipf` |
| `Data\Movies\Intro.pss` | `Data\Movies\Intro512.pss` | `Data\Movies\Intro640.pss` |

`crate::boot::LOOSE_MOVIES` is that table. A loader that goes looking for a file
called `Backdrop.ipf` finds nothing, and one that appends `.PMF` to the widget's
`src` asks for `Data\Movies\Backdrop.ipf.PMF`, which is what this build did
before.

## Open questions

- **What the field at `+0x10` really is.** `16` in both files. Read as an
  alignment because the stride is the longest payload padded to it, but two
  files cannot separate that from a constant.
- **The twelve zero bytes at `+0x14`.** Reserved, or fields no shipped file
  uses.
- **The picture header's second `u16`.** Varies per frame across a small set;
  see above.
- **Whether `.IPF` appears anywhere else in the lineage.** Only Pulse PS2 is in
  scope and it has exactly these two files. Neither PSP disc has anything with
  this magic.
- **Whether the front end ever plays it as a loop in this build.** The
  container decodes and the movie renders, but nothing in the boot sequence
  reaches `FE Screen` yet - the sequence goes `LogoFMV` to the language picker
  to `Launch Game`. `--movie 'Data\Movies\Backdrop.ipf'` plays it today; wiring
  it as the menu backdrop is front-end work, not format work.
