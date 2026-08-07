# ADR-0019: Decode ATRAC3+ out of process, into the movie cache

## Status
Accepted

## Context

Almost all of the audio on the PSP disc is ATRAC3+ and none of it can be played
without a decoder the workspace does not have. A magic scan of all 1,461 WAD
entries counts **93 ATRAC3+ streams totalling 48,154,064 bytes** - 16 stereo
tracks of 177-204 s that are the soundtrack, 12 shorter stereo menu stings, and
64 mono clips that are voice-over. Every movie's audio track is ATRAC3+ as well,
and `pmf::demux` has been handing those frames over intact, to nothing, since it
was written.

The executable's paths for this material are `Data\Music\FEMusic\frontend%d.at3`,
`music.at3`, `Data\Sound\PreRaceDialogue\%d_Track.at3` and
`Data\FE\Profile\SND0.AT3`.

This is the same problem H.264 posed, and ADR-0004's 2026-07-26 amendment
already reasoned it: *"A PSP `.PMF` is MPEG-4 AVC plus ATRAC3+. Playing one in
process means an H.264 decoder in the workspace... the asset is not the problem;
reproducing its decoder is."* ADR-0008 built the answer - transcode at the
boundary into `data/cache/`, never commit the result - and ADR-0017 added
GStreamer as an optional in-process path behind the `native-video` feature.

Since `native-video` already existed and already carried a media pipeline, the
obvious move was to extend it to audio. **That was tried first and measured, and
it does not work.**

- `gst-inspect-1.0 | grep -i atrac` returns `avdec_atrac1` and `avdec_atrac3`.
  ATRAC3 and ATRAC3+ are different codecs; `avdec_atrac3` accepts only
  `audio/x-vnd.sony.atrac3`.
- Across all **1,401** installed elements, the number advertising `atrac3plus`
  or `atrac3p` caps is **zero**.
- The cause is in the library, not the installation:
  `strings libgstlibav.so | grep -i atrac` returns exactly three entries -
  `audio/x-vnd.sony.atrac1`, `audio/x-vnd.sony.atrac3`, `audio/atrac3` - and
  none for ATRAC3+. gst-libav's codec map has no `AV_CODEC_ID_ATRAC3P` entry, so
  no element is ever generated, **even though it links the very
  `libavcodec.so.62` that implements the decoder.**
- Feeding a real file confirms it: `PSP_GAME/SND0.AT3` through
  `filesrc ! decodebin ! audioconvert ! wavenc` fails with
  `wavparse: No caps found for format 0x0, 0 channels, 0 Hz` - the
  `WAVE_FORMAT_EXTENSIBLE` ATRAC3plus GUID is unmapped. There is no second shape
  to try, because there is no element to feed.

`ffmpeg` decodes the same file correctly: 376,910-byte WAV, 2 channels at
44,100 Hz, 2.136 s, peak -2.4 dBFS and RMS 5,710 - audio, not silence.

One further measurement shapes the implementation. ffmpeg's `mpegps` demuxer
surfaces **only the H.264 video** from `Intro.PMF`, at probe sizes up to 50 MB;
the `private_stream_1` (`0xbd`) ATRAC3+ track never appears, and ffmpeg has no
PSMF demuxer. So movie audio has to come from our own `pmf::demux` regardless of
which decoder is chosen.

## Decision

**Decode ATRAC3+ out of process with `ffmpeg`, caching the result under
`data/cache/audio/`.** Same lineage as ADR-0004's amendment and the same cache
discipline as ADR-0008: transcode on first use, read from cache afterwards,
never commit the output.

Two input shapes:

- **RIFF-wrapped `.at3`** files go to `ffmpeg` unmodified.
- **Bare ATRAC3+ frames** from `pmf::Demuxed::audio` are wrapped first, in the
  52-byte `WAVE_FORMAT_EXTENSIBLE` header read off `SND0.AT3`: `wFormatTag`
  `0xFFFE`, `cbSize` 34, `validBitsPerSample` 2048 (the ATRAC3+ samples per
  block), channel mask 3, subformat GUID
  `E923AABF-CB58-4471-A119-FFFA01E4CE62`, and 12 bytes of codec extra data.
  `blockAlign` is per-file and must be taken from the stream rather than copied.

**Movie playback is audio-clocked when the movie has an audio stream, and
tick-clocked otherwise.** `Backdrop.PMF` is video-only *and* its front-end
widget is `sound="false"`, so it has no playhead to pace against - and it is the
movie that plays most.

**`native-video` is unchanged and still governs video only.**

## Alternatives considered

**GStreamer behind `native-video`.** The original plan, and the reason this ADR
exists. Rejected on measurement, not preference: the codec mapping is absent
from gst-libav, so there is nothing to configure and nothing to feed.

**Linking `libavcodec` in process** (`ffmpeg-next`, `rusty_ffmpeg`). It is the
same `libavcodec.so.62` gst-libav already pulls in, and it would avoid both the
cache and the subprocess. Rejected for now: it puts an FFI dependency into a
workspace whose lints set `unsafe_code = "deny"`, and it turns a runtime
dependency into a build-time one on Windows and macOS. Worth revisiting if the
cache proves to be the wrong shape for streaming music.

**Writing an ATRAC3+ decoder.** Rejected on ADR-0004's stated position - the
asset is not the problem, reproducing its decoder is. It is weeks of MDCT, QMF
and Huffman work whose only available ground truth would be ffmpeg's output.

**Patching gst-libav upstream** to add the missing caps mapping. A genuine fix,
and worth doing on its own merits, but every user would need a patched
GStreamer until it shipped. Not viable as this project's decode path.

## Consequences

**No `ffmpeg` on `PATH` means no PSP music and no movie sound.** The game still
runs; the missing tool is named on stdout, exactly as the video path already
degrades. This is a worse failure than the video path's, because video has a
second route (the AV1 cache) and audio has none.

**Better than the plan it replaced, in one respect.** Because it does not use
`native-video`, audio works on every platform with `ffmpeg` rather than on Linux
only. The rejected GStreamer route would have made the entire soundtrack a
Linux-only feature.

**A first run is slow and the cache is large.** 48 MB of ATRAC3+ decodes to
several hundred MB of PCM. `data/cache/` is gitignored and covered by `just
audit-leakage`, but disk use is real and cache eviction is not designed.

**Streaming is now two problems, not one.** A cached decode is a file on disk,
so playing a 3-minute track means streaming from the cache rather than from the
archive - the `Archive` ranged-read work solves the archive half and not this
half.

**Movie A/V sync gets its own failure mode.** Making audio the clock means
`movie::Player` becomes something two consumers pace against instead of one, in
the file that only recently had a two-playhead bug and a wrapped-frame-index bug
closed. The backdrop's continuity across `Show Logo` to the menus is the
regression to watch.
