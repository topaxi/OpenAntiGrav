---
status: blocked-maintainer
titles: [pulse, pure, hd]
updated: 2026-11-19
summary: "ATRAC3+ (Pulse, Pure) and Bink audio (HD movies) still decode through ffmpeg; carta5 (ISC) is the one ATRAC3+ candidate"
difficulty: hard (70)
---

# ATRAC3+ and Bink audio still need `ffmpeg`

2026-11-19, lane `inprocess-audio`. Every other codec decodes in process now:
ATRAC9 (2048 and Omega, `docs/formats/atrac9.md`), MP3 (HD), Wwise media
(Omega), AAC (2048's `intro.mp4`, `docs/formats/mp4.md`), PCM (Pulse PS2).
GitHub #26 (no music in Pulse without `ffmpeg`) and Android are what is left.

## Open

- **ATRAC3+**: the census is `docs/formats/atrac3plus.md`. Every decoder is
  LGPL except **carta5** (github.com/aynik/carta5, ISC, JavaScript, 37k lines,
  provenance unstated). Its decoder was not run: executing the downloaded code
  was refused by the sandbox, so it is unmeasured against FFmpeg.
- **Decision for the maintainer**: port carta5's decoder half to Rust under ISC
  (keep its notice, roughly 5,000 lines), or keep ATRAC3+ on `ffmpeg`. A port
  is our own decoder for a codec we did not invent, the category
  [ADR-0024](../../docs/architecture/adr/0024-in-process-codecs-and-ffmpeg-as-a-last-resort.md)
  forbids, so a yes needs a superseding ADR as part of the decision.
- **Bink audio**: no permissive decoder; the MultimediaWiki page leaves the
  coefficient packing and the RLE table's use out (`docs/formats/bik.md`, "Why
  Bink audio is still decoded by `ffmpeg`"). RAD's decoder is in HD's EBOOT.

## Next Steps

1. Get the maintainer's answer on carta5, and **explicit approval before
   executing its code**: the sandbox refused running the downloaded JavaScript
   once already. With that approval, decode Pulse's `frontend1.at3` with it
   (node, no dependencies: `decodeWavePcm` from `codec/index.js`) against
   `ffmpeg -f s16le` to see the tolerance before porting.
2. Port the decoder half into `oag-music` (`at3.rs` decode side, ahead of the
   `ffmpeg` resolver); prove it per file over every `Data.wad` RIFF ATRAC3+
   entry of both Pulse pressings and both Pure pressings, and every `.PMF`
   audio track, against FFmpeg; then `MovieAudioKind::Atrac3Plus` decodes in
   process too.
3. Bink audio only if wanted: read RAD's decoder in the HD EBOOT.
