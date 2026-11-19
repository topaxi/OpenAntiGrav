# ffmpeg: the resolver and the pinned download

Pulse's and Pure's music (ATRAC3+), Bink movie audio and every movie transcode go through
`ffmpeg` and `ffprobe` as child processes. `oag-ffmpeg` is the one place that
finds them.

## The resolver

`oag_ffmpeg::command(Tool)` and `resolve(Tool)`: **`PATH` first, then the
downloaded copy, then none** (a bare name, so a missing tool fails with the
`NotFound` each call site already reports). Nothing is latched, so a copy that
arrives mid-session is used by the next decode. Call sites: `oag-music`'s
`at3` (ATRAC9 and the MP4 AAC audio are decoded in process and never reach it), `oag-game`'s `movie::transcode`,
`movie::container_audio`, `movie::mpeg2_ps` and `prefetch`. A system tool is
never replaced; only `[ffmpeg] use_pinned = true` (config file, no menu row)
makes the resolver prefer the pinned copy.

`oag-game` configures it once at startup (`oag_ffmpeg::configure`) with
`use_pinned` and the install root (`oag_source::cache::default_ffmpeg_dir`,
`<cache dir>/oag/ffmpeg`). The pinned copy lives in
`<root>/ffmpeg-<version>/`, so a pin bump never reuses an old binary.

## The pin

One table in `crates/ffmpeg/src/pins.rs`: URL, FFmpeg version, SHA-256, size
and license per OS and architecture; `docs/overview/installing.md` repeats it.
The version is BtbN's month-end `autobuild-YYYY-MM-DD-HH-MM` release tag (those
are kept, the daily ones pruned) plus FFmpeg's own `n8.1.3-27-g<commit>`; the
content is the archive's SHA-256. Static LGPL v3 builds (`--enable-version3`,
no `--enable-gpl`, no x264/x265) with `libaom` (the movie transcode's encoder)
and the native ATRAC3+ decoder. macOS has no pin: no trustworthy static LGPL
build is published, so it keeps the install note. Android and wasm download
nothing.

`scripts/bump-ffmpeg-pin.py` moves the table to the newest month-end build
(hashes from GitHub's asset digests); `.github/workflows/ffmpeg-pin.yml` runs it
monthly and opens a PR, the way Dependabot does for crates. Run
`just test-data` before merging it.

## Consent and the download

`[ffmpeg] download = "ask" | "yes" | "never"`. The front end
(`app::session::ffmpeg`) asks once per run, only when `oag_ffmpeg::wants_download`
(no `ffmpeg` anywhere and a pin for this OS), as a yes/no prompt naming the size
and source. Yes is stored and starts the download on its own thread with a
status line (BACK or a right click cancels); no is stored too. OPTIONS, AUDIO,
DOWNLOAD FFMPEG (shown only when there is something to offer) asks again.
`oag-game --fetch-ffmpeg` does it without a window. Nothing downloads without a
yes.

`oag_ffmpeg::install` downloads to a scratch directory, hashes as it reads,
unpacks only `ffmpeg`, `ffprobe` and `LICENSE.txt` (tar.xz through `lzma-rust2`,
zip through `zip`; HTTP is `ureq` on rustls), and renames the directory into
place only when all three are there. A mismatch, a cancel or an error leaves
nothing behind. The simulation knows nothing of any of this.

A movie transcode at boot happens before the offer, so the intro appears from the
next launch; music is decoded when asked for, so it follows the download.

## In-process codecs (`inprocess-audio`)

A stream a Rust decoder handles never calls the resolver. Adding a codec in
process therefore needs no change here: it takes precedence per stream by not
reaching `command`. The resolver stays for what is left (ATRAC3+, movies).
