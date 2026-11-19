# oag-ffmpeg

Where the game finds the `ffmpeg` and `ffprobe` it shells out to (Pulse and Pure
music, every movie), and how it fetches a pinned LGPL build of them when the
player has none and says yes.

One resolver: `PATH` first, then the downloaded copy, then none. A system
`ffmpeg` is never replaced and never prompts a download; only the config file's
`use_pinned` prefers ours. The composition root (`oag-game`) owns consent, the
prompt and the cache directory and hands them in; this crate asks nobody
anything. A codec decoded in process never reaches it.

Leaf crate: `anyhow` and `log`, plus an HTTP client and archive readers off wasm
and Android. See [ffmpeg](../../docs/architecture/ffmpeg.md).
