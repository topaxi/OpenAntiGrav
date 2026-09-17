# The intro movie is silent on PS2 Pulse and HD/Fury, and both carry audio the loader does not reach

2026-09-17. Reported from play by the user: "the pulse PS2 intro does not
play any sound; on PSP, Pure and Pulse do play the intro video sound. HD/Fury
also do not." Both absences are real and both have a known shape:

- **HD/Fury (`.bik`)**: already recorded - Bink's audio is *measured and not
  played*: six of 37 files carry tracks, the two logo reels four each at
  48 kHz stereo DCT, asserted against the disc, and `Studio Logo` boots with
  `audio: Studio Logo plays silently`. The blocker is the shape of
  `Movie::audio` (an ATRAC3+-only type) and a second `ffmpeg` route through
  `boot::load_movie_sound`. See
  [wipeout-hds-video-decodes-and-the-logo-reel.md](../rendering/wipeout-hds-video-decodes-and-the-logo-reel.md)
  and [bik.md](../../docs/formats/bik.md).
- **PS2 Pulse (`.PSS`)**: **the intro carries PS2 ADPCM audio in MPEG private
  stream 1, and nothing here demuxes it.** Measured 2026-09-17 on
  `54748/DATA/MOVIES/INTRO640.PSS` (`oag-unpack extract` out of
  `pulse-ps2-eu.chd`): 4,676 video PES packets (`0x1e0`) and **1,795 private
  stream 1 packets (`0x1bd`, sub-stream id `0xff`)**; the first private
  payload opens with a Sony `SShd` header - `u32 header_len 24, u32 format 1,
  u32 sample_rate 48000, u32 channels 2, u32 interleave 512` - followed by
  `SSbd` and a `u32` body length of `7,303,168` bytes: 48 kHz stereo
  PS-ADPCM, 512-byte channel interleave, ~38 s (the movie is 38.07 s).
  `ffprobe` lists only the video stream, which is why "decodes but silent"
  was easy to mistake for "has no track". The decoder already exists:
  `oag_formats::sblk::decode_adpcm` decodes PS-ADPCM for the PS2 sound
  banks. What is missing is the demux (walk the `0x1bd` PES packets, strip
  the `SShd`/`SSbd` framing, de-interleave the two channels at 512 bytes)
  and, as for Bink, a `Movie::audio` that can hold PCM from a source other
  than ATRAC3+.

Both share the fix's shape: widen `Movie::audio` from "an ATRAC3+ block
stream" to "a decoded track, however it was got", and `load_movie_sound`
plays whichever the container yields. The PSP path is untouched by that.

## Open

- PS2: whether `Intro512.pss` (PAL) carries the same track at the same rate.
- PS2: the private stream's per-packet framing after the first (`SShd` once
  at the head, or per packet?) - read from the file, not assumed.
- HD: which of the two reels a PS3 plays is still the which-`skin.xml`
  question on the rendering thread; the audio fix applies to both.

## Next Steps

1. `oag_video::pss` (or beside `ipf`): demux private stream 1 into a
   PS-ADPCM byte stream with the `SShd` parameters; a ground-truth test
   asserts the 7,303,168-byte body and the 1,795-packet count.
2. Widen `Movie::audio`; decode PS2 through `sblk::decode_adpcm`, Bink
   through the `ffmpeg` cache route ADR-0024 already allows.
3. `just play data/images/pulse-ps2-eu.chd` boots with sound; a WAV of the
   first seconds in `data/shots/` for the ear.
