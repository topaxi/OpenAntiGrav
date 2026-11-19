# ATRAC9

Sony's ATRAC9 is the audio codec of every Wipeout 2048 (Vita) sound and of
every Omega Collection (PS4) Wwise media file. It is decoded in process on every
target by a vendored copy of the `atrac9dec` crate, fixed against LibAtrac9.
Two containers carry it:

| Container | Title | Reader | Census |
| --- | --- | --- | --- |
| RIFF `.at9` (`WAVE_FORMAT_EXTENSIBLE`, ATRAC9 GUID) | 2048 | [`oag_music::at9`](../../crates/music/src/at9.rs) | 9,486 files |
| Wwise `.wem` (`fmt ` tag `0xFFFC`), loose and inside banks | Omega | [`oag_music::wem`](../../crates/music/src/wem.rs), [wwise.md](wwise.md#the-wem) | 5,544 streams |

Both go through `oag_music::at9::decode_superframes`, which refuses a
configuration word that disagrees with its container on channels, rate or
superframe size.

## Census

Measured 2026-11-19 by a sniff of every entry of every archive (RIFF magic, then
the `fmt ` chunk), not by extension, then reproduced by
`crates/music/tests/atrac9_ground_truth.rs` and
`crates/music/tests/omega_wem_ground_truth.rs`.

**2048**: 9,486 RIFF ATRAC9 files, every one named `.at9`, and no RIFF ATRAC9
under any other name. 4,561 in each region's base `PSP2/data.psarc` (EU
`PCSF00007`, USA `PCSA00015`) and 182 in each region's v1.04 patch
`data2.psarc`; the patch's `data1.psarc` and the three DLC packs carry none.
Every one is stereo, 48,000 Hz, four 256-sample frames to a 384-byte superframe
(configuration word `FE 74 0B F0`, channel-config index 2, superframe index 2).
Per region: the eleven race tracks (`data/audio/music/<NN>/music_stereo.at9`),
the front end's and the demo front end's `frontend_stereo.at9` (`FEMusic`,
`FEDemoMusic`), and 4,730 speech streams under
`data/audio/sound/streams/atrac/<language>/` (grouped by directory over the
archives' own path lists). The `.fft` beside each music file
is not audio: it opens `AFEX`, the spectrum data `oag_music::spectrum` reads.

**Omega**: 1,448 loose `.wem` and 4,096 media inside the `BKHD` banks of the
base and patch archives, by layout:

| Channel config (index) | Rate | Streams |
| --- | --- | --- |
| mono (0) | 24,000 Hz | 496 |
| mono (0) | 48,000 Hz | 1,087 |
| stereo (2) | 24,000 Hz | 94 |
| stereo (2) | 48,000 Hz | 3,751 |
| 5.1 (3): Stereo, Mono, LFE, Stereo | 48,000 Hz | 4 |
| 7.1 (4): Stereo, Mono, LFE, Stereo, Stereo | 48,000 Hz | 36 |
| quad (5): Stereo, Stereo | 48,000 Hz | 76 |

Omega uses one frame per superframe throughout.

## The RIFF `.at9` header

Measured on all 9,486 files.

| Chunk | Offset in body | Type | Meaning |
| --- | --- | --- | --- |
| `fmt ` | `+0x00` | `u16` | `0xFFFE`, `WAVE_FORMAT_EXTENSIBLE` |
| | `+0x02` | `u16` | channels |
| | `+0x04` | `u32` | sample rate |
| | `+0x0C` | `u16` | block align: the superframe size in bytes |
| | `+0x18` | GUID | `{47E142D2-36BA-4D8D-88FC-61654F8C836C}`, ATRAC9 |
| | `+0x28` | `u32` | version, 1 |
| | `+0x2C` | `u8[4]` | the ATRAC9 configuration word (big-endian bit fields) |
| | `+0x30` | `u32` | reserved, 0 |
| `fact` | `+0x00` | `u32` | samples per channel a player hears |
| | `+0x04` | `u32` | 256 on every file; unread |
| | `+0x08` | `u32` | encoder delay in samples per channel: 256 on every file |
| `data` | | | whole superframes |

`fact`'s arithmetic holds on every file: the decode is the delay, then the
declared samples, then under one superframe (1,024 samples) of padding.
`oag_music::at9::decode` skips the delay and trims the padding, so the front
end's track is 14,496,000 samples, exactly 302.000 s. Which of the two 256-sample
words is the delay is inferred (both are 256 everywhere); the arithmetic fits
either. Confidence 80.

The configuration word, read MSB first: 8 bits sync `0xFE`, 4 bits sample-rate
index, 3 bits channel-config index, 1 validation bit (0), 11 bits frame bytes
minus one, 2 bits superframe index (frames per superframe is `1 << index`).

## The decoder

[ADR-0024](../architecture/adr/0024-in-process-codecs-and-ffmpeg-as-a-last-resort.md)'s
order puts a library first, and ATRAC9 has two permissive references:

| Source | Licence | What it is | Used how |
| --- | --- | --- | --- |
| [LibAtrac9](https://github.com/Thealexbarney/LibAtrac9) | MIT | C and C# decoder by Alex Barney | Read and ported (the fixes below); built as a test oracle, never linked |
| [`atrac9dec`](https://crates.io/crates/atrac9dec) 0.9.1 | MIT | A Rust port of LibAtrac9 | Vendored with fixes at `vendor/atrac9dec`, `[patch.crates-io]` |
| FFmpeg's `atrac9dec.c` | LGPL | An independent decoder | Run as a separate program to compare output; not read |

### What the published crate got wrong

Each fix is a port of the named LibAtrac9 function and carries an "OpenAntiGrav
patch" comment at its site.

1. **The LFE block was a placeholder** (`UnpackLfeBlock`): it read the wrong
   number of bits, so every 5.1 and 7.1 stream desynchronised at its third block.
   This is what refused the six Omega cues.
2. **Joint-stereo signs were read from unit 0** rather than from the stereo
   unit (`ReadStereoParams`), and never cleared when absent, so any stream
   that codes them desynchronised.
3. **Band extension was skipped** (`ReadExtensionParams`, `BexReadHeader`,
   `BexReadData`, and the whole of `band_extension.c`): the stereo header was
   read as two unconditional bits and the payload never applied, so a stream
   that enables it lost its high band.
4. **Intensity stereo ran after scaling** (`DecodeFrame`'s order is
   dequantise, intensity stereo, scale, band extension, IMDCT), so the
   secondary channel's intensity units carried the primary's scale factors.
5. **The fine residual used the coarse step table** (`QuantizerFineStepSize`).

Before the fixes, against LibAtrac9: 4,076 of 2048's 9,486 files failed
part-way with a bitstream error, 3,218 more decoded but differed, and 2,192
were identical; every Omega stream of four channels or more was refused.

The vendored crate stays a separate package with its own `MIT` manifest and
LibAtrac9's licence beside it (`vendor/atrac9dec/LICENSE-LibAtrac9`). The fixes
are upstreamable; no upstream contact was made from this project.

## Conformance

**Against LibAtrac9: sample-identical on every stream.** All 9,486 2048 files
and all 5,544 Omega streams, the whole untrimmed decode, every layout in the
census, 0 differing samples. The oracle is LibAtrac9 built from source
(`scripts/atrac9-oracle.c`); the sweep is
`atrac9_against_libatrac9_sweep`:

```sh
git clone https://github.com/Thealexbarney/LibAtrac9 <dir>
cc -O2 -I <dir>/C/src -o target/atrac9-oracle scripts/atrac9-oracle.c <dir>/C/src/*.c -lm
OAG_SWEEP=1 OAG_ATRAC9_ORACLE=target/atrac9-oracle \
  cargo nextest run -p oag-music --run-ignored only atrac9_against_libatrac9_sweep --no-capture
```

**Against FFmpeg: within one LSB, negated, on every 2048 file**: ours is the
negation of FFmpeg's to within 1 on every sample of all 9,486 files
(`atrac9_ground_truth`, which runs FFmpeg 64 files to a process). On Omega,
the stereo, quad and 7.1 files `the_decoder_agrees_with_ffmpeg_up_to_polarity`
picks agree the same way on every channel. The polarity is a difference between
two implementations; which one matches Sony's decoder is not known, and it is
inaudible on its own.

### The LFE channel

On some 7.1 media LibAtrac9 and FFmpeg decode the LFE channel differently.
Omega media `144136475` (one of the six cues): the seven other channels agree to
one LSB negated, but the LFE's RMS is 2,881 in LibAtrac9 and 6,263 in FFmpeg,
which clips it at full scale; LibAtrac9's is a smooth signal around 33 Hz. The
loose 7.1 file the FFmpeg test picks agrees on all eight channels. This follows
LibAtrac9; which matches Sony's decoder is open, and it does not reach a player
today: `oag_audio` plays the first two channels of a multi-channel sound.

## What a player gets

- **2048**: the front end's and the race's music decode in process, with no
  `ffmpeg` and no cache. The speech streams decode too but nothing plays them:
  the speech banks name them ([2048-audio.md](2048-audio.md#speech__ngp-a-pool-of-stream-names-not-adpcm)).
- **Omega**: every `.wem` decodes. Six posted race cues that stopped at their
  7.1 media now play (four hits, the Quake launch, and the death loop): 30 of
  43 race cues play their Wwise event, up from 24. The eleven songs stored as
  one eight-channel file decode but are not played, since the fold to stereo is
  unread ([wwise.md](wwise.md#what-is-not-read)).

## Cross-title

2048 and Omega: **ported**. Both titles' ATRAC9 goes through the one decoder,
with a ground truth each (`atrac9_ground_truth`, `omega_wem_ground_truth`).
