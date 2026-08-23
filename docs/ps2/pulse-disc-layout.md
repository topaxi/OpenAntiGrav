# Wipeout Pulse (PS2) disc layout

Source: `pulse-ps2-eu.chd`, serial **SCES-54748**, volume date 2009-05-15.
See [source images](../reverse-engineering/source-images.md).

The PS2 release was Europe-only, published roughly 16 months after the PSP
version.

Reproduce with:

```sh
just unpack container data/images/pulse-ps2-eu.chd
just unpack list      data/images/pulse-ps2-eu.chd --dirs
just unpack sniff     data/images/pulse-ps2-eu.chd
```

## Contents

20 files in 4 directories, 3.6 GiB. **2.75 GiB of that is one padding file**, so
actual content is around 900 MiB.

| Size | LBA | Path | Signature | Entropy |
| ---: | ---: | --- | --- | ---: |
| 56 | 546869 | `SYSTEM.CNF` | boot config text | 4.84 |
| **1,991,160** | 545896 | **`SCES_547.48`** | **ELF** | **5.67** |
| 2,751,941,562 | 546870 | `PADZ2` | padding | 7.99 |
| 89,325,308 | 30996 | `54748/PRERACE.WAD` | unknown | **0.08** |
| 585,799,372 | 74612 | `54748/PS2MUSIC.WAD` | unknown | 7.19 |
| 377,308,096 | 360647 | `54748/WADS2.WAD` | [WAD](../formats/wad.md) | 6.14 |
| 1,447,168 | 544880 | `54748/WADSP.WAD` | [WAD](../formats/wad.md) | 7.91 |
| 6,548,432 | 298 | `54748/DATA/MOVIES/BG512.IPF` | [`IPUF`](../formats/ipf.md) | 3.91 |
| 8,899,232 | 3496 | `54748/DATA/MOVIES/BG640.IPF` | [`IPUF`](../formats/ipf.md) | 3.10 |
| 20,905,988 | 7842 | `54748/DATA/MOVIES/INTRO512.PSS` | MPEG-2 PS | 5.84 |
| 26,509,316 | 18051 | `54748/DATA/MOVIES/INTRO640.PSS` | MPEG-2 PS | 2.42 |
| 15,653 | 545587 | `IOP/DBCMAN.IRX` | ELF | 4.66 |
| 12,157 | 545595 | `IOP/DS2O_D.IRX` | ELF | 4.77 |
| 278,353 | 545601 | `IOP/IOPRP310.IMG` | `RESET` module archive | 4.97 |
| 30,085 | 545737 | `IOP/LIBSD.IRX` | ELF | 4.93 |
| 27,981 | 545752 | `IOP/MC2_S1.IRX` | ELF | 4.97 |
| 96,484 | 545766 | `IOP/SCREAM.IRX` | ELF | 5.32 |
| 11,289 | 545814 | `IOP/SIO2D.IRX` | ELF | 4.29 |
| 5,217 | 545820 | `IOP/SIO2MAN.IRX` | ELF | 4.18 |
| 148,900 | 545823 | `IOP/STREAM.IRX` | ELF | 5.54 |

## The container is a DVD packed as a CD

Worth stating separately, because it is a trap.

`chdman` reports this image as a CD: 2448-byte units, one `MODE1` track of
1,900,848 frames.

```
units          1900848 x 2448 bytes  (CD + 96-byte subcode)
metadata       [CHT2] TRACK:1 TYPE:MODE1 SUBTYPE:NONE FRAMES:1900848 PREGAP:0
```

1,900,848 sectors is 3.6 GiB. No CD holds that. This is a **DVD image converted
with `chdman createcd` instead of `createdvd`**.

The trap is in where the user data sits. `createcd` uses a fixed 2448-byte frame
regardless of the source, but the *position* of the user data inside that frame
depends on the **track type**, not the unit size:

| Track type | Source sector | User data offset |
| --- | --- | ---: |
| `MODE1` | 2048, cooked | **0** |
| `MODE1_RAW` | 2352, raw frame | 16 |
| `MODE2_FORM1` | 2048, cooked | 0 |
| `MODE2_RAW` | 2352, raw frame | 24 |

This track is `MODE1`, not `MODE1_RAW`, so the sectors are ordinary 2048-byte
user data written at offset 0 and zero-padded out to the frame size.

Reading at offset 16 because the container says "CD" yields data that is
plausibly structured and entirely wrong. `oag-disc` derives the offset from the
track type and **refuses track types whose layout has not been verified**,
rather than guessing. See
[`chd_source.rs`](../../crates/disc/src/chd_source.rs).

## Findings

### `SCES_547.48` is a plain ELF

Entropy 5.67, ELF magic, 1.99 MiB. No encryption, so it loads straight into
Ghidra as an Emotion Engine binary.

Note it is **half the size** of the PSP `BOOT.BIN` (3.85 MiB). On the face of it
that is surprising for a more capable platform. Likely explanations: more code
lives in the IOP modules, or the PS2 build links less statically. Not
investigated.

Confidence: **98** that it is an unencrypted ELF; **below 40** on any
explanation of the size difference.

### Different WAD organisation from the PSP

The PS2 release does not use the PSP's `Data.wad` / `FE.wad` / `FEData.wad`
split. Instead:

| File | Size | Header | Notes |
| --- | --- | --- | --- |
| `WADS2.WAD` | 360 MiB | `01 00 00 00  20 1c 00 00` | Same format as the PSP WADs; 7,200 entries |
| `WADSP.WAD` | 1.4 MiB | `01 00 00 00  c1 00 00 00` | Same format; 193 entries |
| `PS2MUSIC.WAD` | 559 MiB | `10 00 00 00  b0 b1 ba c1` | **Different header.** Music, given the name and size |
| `PRERACE.WAD` | 85 MiB | `20 00 00 00  64 25 d4 7d` | **Different header**, entropy **0.084** |

`WADS2.WAD` and `WADSP.WAD` share the PSP's container format, which is the
strongest evidence so far that one parser can serve the whole lineage.

`PS2MUSIC.WAD` and `PRERACE.WAD` are something else. `PRERACE.WAD`'s entropy of
0.084 is extraordinary: 85 MiB that is almost entirely one repeated byte.
Preallocated scratch space, or a file whose real content is a small header
followed by an enormous reservation.

Confidence that `WADS2`/`WADSP` share the PSP format: **85**, from the header
shape alone; the record layout has not been validated against these files yet.

### 2.75 GiB of padding

`PADZ2` is 76% of the disc. The name says padding, and the size is what you get
when you fill a DVD-5 to capacity.

It is high entropy (7.994) rather than zeros, and its first bytes happen to
match the ZIP magic `PK\x03\x04`, which is why `oag-unpack sniff` labels it a ZIP
archive. That is a false positive from four coincidental bytes; do not read
anything into it.

Padding a disc to capacity was standard practice: it pushes real data toward the
faster outer edge and keeps seek distances predictable.

### IOP modules

Nine modules for the I/O processor, all standard Sony middleware:

| Module | Purpose |
| --- | --- |
| `SCREAM.IRX` | Sony's SCREAM audio engine |
| `STREAM.IRX` | Streaming |
| `LIBSD.IRX` | Sound driver |
| `SIO2MAN.IRX`, `SIO2D.IRX` | Controller and memory card I/O |
| `MC2_S1.IRX` | Memory card |
| `DBCMAN.IRX` | Disc / buffer management |
| `DS2O_D.IRX` | DualShock 2 |
| `IOPRP310.IMG` | Bundled IOP module archive, magic `RESET` |

Relevant when audio (M5) or save data (M7) come up. `SCREAM` in particular means
the PS2 audio format is likely a documented Sony bank format rather than
something bespoke.

### Video

Two resolutions of each, 512 and 640 wide:

- `.PSS` files are MPEG-2 program streams, confirmed by the `00 00 01 BA` pack
  header. `ffmpeg` reads them directly - no separate demux step the way a
  `.PMF`'s H.264 elementary stream needs, since a program stream carries its
  own container. `oag_game::movie::open_mpeg2_ps` decodes them.
- `.IPF` files carry an `IPUF` magic and hold IPU video - the intra-only
  MPEG-2 the PS2's Image Processing Unit decodes - in a bespoke fixed-slot
  container. **Both are now decoded**: see [`ipf.md`](../formats/ipf.md) for the
  container and the transcode route. They are the looping menu backdrop, 225
  frames at 25 fps and 270 at 29.97, the same nine seconds twice.

**Both containers are picked between by one function, and it is the same one
for both.**
[`Movie_ResolveSourcePath`](../ghidra/functions/ps2-pulse-eu/movie-paths.md)
(`0x0019b168`, confidence 90, formerly `FUN_0019b168`) rewrites a `Movie`
widget's `src` in place: `Data\Movies\Intro.pss` becomes `Intro512.pss` or
`Intro640.pss`, and `Data\Movies\Backdrop.ipf` becomes `bg512.ipf` or
`bg640.ipf`, both on the value of `0x0027a85c`. So neither name in the
front-end XML is a file on the disc, and the PAL/NTSC split below governs the
backdrop as well as the intro.

**512 is PAL, 640 is NTSC - measured, not presumed.** `ffprobe`:
`INTRO512.PSS` is 512x512 at 25 fps; `INTRO640.PSS` is 640x448 at 29.97 fps
(`30000/1001`). Both declare a 4:3 display aspect (`INTRO512.PSS`'s pixels are
non-square: `SAR 4:3` on a square 512x512 decode). Confirmed against
`SCES_547.48` in Ghidra, confidence **85**: `Data\Movies\Intro512.pss` and
`Data\Movies\Intro640.pss` are both literal strings, selected in
`Movie_ResolveSourcePath` by a global (`0x0027a85c`) - nonzero picks `512`, zero
picks `640`. That global's sole writer, `FUN_0010b030`, sets a PAL-shaped
non-square pixel-aspect correction (`0.8`/`1.1428`) and issues a GS mode-setup
call when passed `1`, and a `1.0`/no-correction set when passed `0` - matching
the measured PAL/NTSC split exactly. Not fully traced: the ultimate trigger
that decides which value `FUN_0010b030` is called with (a numbered
event-dispatcher case in `FUN_00186ed8`, itself not read further). For the
EU/PAL disc this project reads, `512` is what `oag-game` tries first.

**The `.IPF` backdrops have no rate of their own and inherit this split**, which
is the only reason `BG512.IPF` is played at 25 Hz and `BG640.IPF` at 29.97: the
same global picks between them, and the resulting durations are 9.000 s and
9.009 s. See [`ipf.md`](../formats/ipf.md#the-frame-rate-is-inferred-not-read).

**The encode is anamorphic - but to the frame, not to the 4:3 it declares.**
This page said 4:3 until 2026-08-23, on the strength of a symmetric UI mark
looking right when corrected to it. That mark is a stylised helix whose correct
proportions were assumed; the film's closing card is the `wipEout PULSE` logo,
which is the same artwork as `pulse_logo.mip` and can therefore be compared
against the PSP front end drawing that very texture. Its ink measures **4.20**
wide-to-tall there, **4.23** with the film scaled to 16:9, and **3.14** at the
declared 4:3. So `512`/`640` are a space-saving encode of a *wide* picture, and
the declared SAR is a tag the picture does not honour. Confidence 88; the
measurement, and what it means for the 2D layer around it, is
[aspect-ratio.md](aspect-ratio.md).

**The original's front end does not stretch it either.** Checked against
`Skin.xml`: the PS2's `Movie` widget declares no `width`/`height` at all
(unlike the PSP's, which does), and the black `Image` behind it in
`LogoFMV`/`Play Intro` is `640x448` - the NTSC cut's own decoded resolution,
exactly. So the original's front end and its video shared one native buffer,
with nothing to stretch, and that buffer is itself shown wide. `oag-game` draws
these cuts at the frame's aspect for that reason (`movie::PS2_DISPLAY_ASPECT`);
`docs/tools/oag-game.md` has the history of the boxing that got them there.

## Comparison with the PSP release

| | PSP | PS2 |
| --- | --- | --- |
| Serial | UCUS-98712 | SCES-54748 |
| Date | 2008-01-04 | 2009-05-15 |
| Executable | 3.85 MiB ELF | 1.99 MiB ELF |
| Content | 354 MiB | ~900 MiB |
| Archives | `Data`, `FE`, `FEData`, `BEData` | `WADS2`, `WADSP`, `PS2MUSIC`, `PRERACE` |
| Video | 1 PMF (icon animation) | 2 PSS + 2 IPF |
| Audio | ATRAC3 | SCREAM engine |

**The region confound:** our PSP copy is US and our PS2 copy is EU, so every
difference above is also a region difference. Do not attribute anything to the
platform until region is ruled out. See
[source images](../reverse-engineering/source-images.md#region-asymmetry).
