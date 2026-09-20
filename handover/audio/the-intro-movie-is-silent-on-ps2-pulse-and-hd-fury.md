# The intro movie is silent on HD/Fury, and its audio is measured but not played

2026-09-17. Reported from play by the user: "the pulse PS2 intro does not
play any sound; on PSP, Pure and Pulse do play the intro video sound. HD/Fury
also do not." Both absences were real; **the PS2 one is fixed** (2026-09-20),
and HD/Fury's remains open.

- **PS2 Pulse (`.PSS`) - fixed.** The intro's audio rides in MPEG
  `private_stream_1`, and it is **16-bit PCM, not PS-ADPCM** - the framing was
  read right the first time and the codec wrong: decoding the measured bytes
  as PS-ADPCM leaves under 6% of blocks in spec, what unrelated bytes give by
  chance. Reading them as PCM instead gives a track whose duration, sample
  smoothness and block-boundary continuity all agree with the container - see
  [`docs/formats/pss.md`](../../docs/formats/pss.md) for the full evidence.
  `oag_video::pss` demuxes the container framing, `crate::movie::track` widened
  `Movie::audio` to hold either an undecoded ATRAC3+ block stream or
  already-decoded PCM, and `load_movie_sound` plays whichever the container
  yielded through the one route. Verified with `--dump-audio`: the WAV is not
  silence and the boot log reports the track decoding to 38.04 s against the
  container's own 38.07 s.
- **HD/Fury (`.bik`) - still open.** Bink's audio is *measured and not
  played*: six of 37 files carry tracks, the two logo reels four each at
  48 kHz stereo DCT, asserted against the disc, and `Studio Logo` boots with
  `audio: Studio Logo plays silently`. `Movie::audio` no longer blocks this by
  itself - it can hold a decoded track from any source now - but Bink's own
  audio sits inside the video container rather than beside it the way `.PMF`
  and `.PSS` do, and playing it needs a second `ffmpeg` route through
  `boot::load_movie_sound` that has not been built. See
  [wipeout-hds-video-decodes-and-the-logo-reel.md](../rendering/wipeout-hds-video-decodes-and-the-logo-reel.md)
  and [bik.md](../../docs/formats/bik.md#the-audio-is-inside-the-video-file).

## Open

- HD: the `ffmpeg` route for Bink's muxed audio - decode the whole container
  through `ffmpeg`'s own Bink demuxer, the way `.PSS` video already does, and
  extract just the audio stream. ADR-0024 already allows the `ffmpeg` cache
  route; nothing has built it for audio yet.
- HD: which of the two reels a PS3 plays is still the which-`skin.xml`
  question on the rendering thread; the audio fix, once built, applies to
  both.
- PS2: what a `.PSS` `format` code other than 1 would decode as - not
  observed on this disc, so unhandled on principle rather than guessed at.

## Next Steps

1. HD: a `Movie::audio` variant (now trivial to add - see
   `crate::movie::track::MovieAudioKind`) fed by decoding a `.bik`'s audio
   through `ffmpeg` and the cache, the same shape ATRAC3+ already uses.
2. HD: wire `boot::load_movie_sound` to call it for a Bink movie and confirm
   with `--dump-audio` the same way the PS2 fix was confirmed.
3. Once both landed, delete this thread and its `HANDOVER.md` index line.
