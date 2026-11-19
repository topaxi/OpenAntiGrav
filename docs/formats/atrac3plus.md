# ATRAC3+ (ATRAC3plus)

Sony's ATRAC3+ is the music, front-end loops, voice clips and movie sound of
Wipeout Pulse and Pure on the PSP: RIFF-wrapped `.at3` entries in `Data.wad`
(`WAVE_FORMAT_EXTENSIBLE`, 44,100 Hz, 2,048 samples a block, 560 bytes a block
stereo and 280 mono), and the audio stream of every `.PMF` movie (560 or 744
bytes a block, [pmf.md](pmf.md#audio-framing)). Pulse PS2 does not use it: its
music and voice are PCM ([ps2-audio.md](ps2-audio.md)).

It is the one game-audio codec still decoded out of process, by `ffmpeg`
([ADR-0019](../architecture/adr/0019-atrac3plus-out-of-process.md),
`oag_music::at3`). Without `ffmpeg` (Android, the browser, a desktop without
it) Pulse and Pure play no music. This page is the census of what could
replace it.

## Census of decoders, 2026-11-19

The licence bar is the project's (`MIT OR Apache-2.0`): MIT, Apache-2.0, BSD,
zlib, ISC and MPL-2.0 may be taken or learned from; GPL and LGPL code may be
neither linked nor ported.

| Implementation | Language | Licence | ATRAC3+ | Derived from | Usable |
| --- | --- | --- | --- | --- | --- |
| FFmpeg `atrac3plus*.c` | C | LGPL-2.1+ | full decoder | its own reverse engineering | no: run as a program only, as today |
| PPSSPP `at3_standalone` | C++ | LGPL-2.1+ | full decoder | FFmpeg | no |
| [`atrac3p-decoder`](https://github.com/tarkah/atrac3p-decoder) 0.1.2 | Rust | LGPL-2.1+ | decoder, `.at3` only | FFmpeg | no |
| [`codec-atrac`](https://github.com/pschatzmann/codec-atrac) | C++ | LGPL-2.1+ | decoder | FFmpeg | no |
| [atracdenc](https://github.com/dcherednik/atracdenc) | C++ | LGPL-2.1 | encoder | its own | no |
| [atrac3_decoder](https://github.com/bobrost/atrac3_decoder) | C | MIT | **ATRAC3 only** (MiniDisc LP2) | its own | no: a different codec |
| [atrac-encoder](https://github.com/kai-h/atrac-encoder) | C99, Python | MIT | **ATRAC3 only** (LP2/LP4) | its own | no: a different codec |
| [ATRACTool](https://github.com/XyLe-GBP/ATRACTool) | C# | MIT | converter | Sony's own command-line tools, run as programs | no: not a decoder |
| [**carta5**](https://github.com/aynik/carta5) 0.6.0 | JavaScript | **ISC** | encoder and decoder; mono, stereo, 5.1, 7.1; 41 profiles incl. 44.1 kHz stereo 96 kbps (560 bytes) and mono 48 kbps (280 bytes) | **not stated** | **the one candidate** |

Sony publishes no ATRAC3+ bitstream specification. The
[MultimediaWiki page](https://wiki.multimedia.cx/index.php/ATRAC3plus)
describes the decoder's stages (16-band PQF, MDCT, gain compensation, tone
synthesis, power compensation) but none of the variable-length codebooks,
quantiser tables or the PQF prototype filter, and a decoder cannot be written
without them. Outside the LGPL code those exist only in Sony's firmware and
tools.

### carta5

ISC, 37,611 lines of JavaScript under `codec/`, no runtime dependencies, one
author, first published 2026-08-19 with the whole codec in its first commit.
Its README and history state no provenance, and nothing in it names FFmpeg or
atracdenc. Two structural signs point away from FFmpeg: its tables are stored
as float32 bit patterns (`float32FromBits`), and its QMF synthesis is laid out
as delay rows and modulation coefficients rather than FFmpeg's DCT-IV form.
Both are consistent with tables read out of a Sony binary, which is how
LibAtrac9 came to be, but this is **inferred, not shown** (confidence 40).

**Not measured.** Running its decoder on a Pulse track to compare it with
FFmpeg was not done in this lane: executing the downloaded code was refused by
the sandbox. A port to Rust would be roughly 5,000 lines (the decoder half of
`codec/syntax`, `codec/dequantization`, `codec/synthesis`, `codec/signal` and
the tables it reads).

## What is decided and what is open

- **Open, a maintainer decision**: whether to port carta5 under its ISC
  licence given its unstated provenance, after measuring it against FFmpeg on
  every Pulse and Pure stream and PMF track.
- Until then ATRAC3+ stays on `ffmpeg`, and a missing `ffmpeg` is a `warn`
  naming Pulse's and Pure's music (`oag_music::at3`).
