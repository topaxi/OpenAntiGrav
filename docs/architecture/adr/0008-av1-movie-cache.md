# ADR-0008: Cache movies as lossless AV1, and decode them in process

## Status

Accepted.

Supersedes the **cache format** chosen in
[ADR-0004](0004-asset-pipeline.md)'s 2026-07-26 amendment. Everything else in
ADR-0004 stands unchanged: originals are still the source, conversion still
happens once at the boundary, the cache is still derived content that is never
committed and never redistributed, and `ffmpeg` is still optional.

## Context

ADR-0004 decided that `.PMF` video is transcoded once by `ffmpeg` and cached,
rather than decoding H.264 in process. It chose **raw `yuv420p` frames** as the
cache format, because raw needs no decoder at playback time and indexes by
seeking.

That decision also predicted what we would do if we ever shipped our own video:
AV1, decoded by `rav1d`, because it is the only production-grade codec with a
pure-Rust decoder and because it is royalty-free. That left the project with two
video formats in its future - raw for derived content, AV1 for authored - and a
decoder that would arrive eventually anyway.

The cost of raw is size. Measured on the real disc, with the flags in this ADR,
verified bit-exact by decoding the result and comparing it byte for byte with the
raw frames it replaces:

| Clip | Frames | raw `yuv420p` | lossless AV1 | ratio | encode |
| --- | ---: | ---: | ---: | ---: | ---: |
| Intro, default extent | 261 | 48.8 MiB | 1.17 MiB | 41.6x | 8 s |
| Intro, all 1200 frames | 1200 | 224 MiB | 33.0 MiB | 6.8x | 81 s |
| Dev/pub reel | 260 | 48.6 MiB | 173 KiB | 287x | 3 s |

The spread is content, not luck: the first 261 frames are logo cards, the rest of
the intro is real footage, and the reel is nearly static. For reference, lossless
AV1 of the whole intro is about seven times *larger* than the lossy H.264 it came
from, which is the expected direction.

Two objections to AV1 were raised when ADR-0004 was written and both turned out
to be wrong under measurement:

- **"The encode is much slower."** It is 8 seconds for the default extent and 81
  seconds for the whole intro, against a few seconds for raw. Real, but not a
  reason.
- **"Random access is lost."** The `FrameStore` API is index-shaped, but the
  *usage* is not: the player only ever increments, and loops back to zero.
  Sequential decode with a rewind serves it exactly.

## Decision

**Cache movies as lossless AV1 in an IVF container, and decode them in process
with `re_rav1d`.**

- The transcode is still `ffmpeg`, still out of process, still optional.
- **Lossless, not merely high quality.** The picture stays bit-for-bit what the
  H.264 decoder produced, so ADR-0004's fidelity rule - a conversion may not
  change how the game looks - is satisfied by construction rather than by
  judgement.
- The decoder is [`re_rav1d`](https://crates.io/crates/re_rav1d), a Rust port of
  `dav1d`, with **`default-features = false`**. The default `asm` feature
  requires `nasm` at build time; without it a 480x272 movie still decodes at
  roughly 900 frames per second, thirty times faster than playback.
- The IVF parser and the AV1 decoder live in `oag-formats`, behind an `av1`
  cargo feature that is **off by default**, so anything depending on
  `oag-formats` for Wipeout containers alone still pulls in no dependencies.
  Note that this does *not* save a `--workspace` build any work: cargo unifies
  features across the workspace, and `oag-game` turns `av1` on, so
  `cargo build --workspace` compiles the decoder regardless. The gate is for
  consumers of the crate, not for build time here.
- The encoder identity is part of the cache **filename** (`-av1ll-`), so
  changing the encoder or its flags invalidates by name.

The flags are load-bearing and are recorded here because getting them wrong
fails silently:

```sh
ffmpeg -f h264 -i in.h264 -frames:v 261 \
  -c:v libaom-av1 -b:v 0 -crf 0 -qmin 0 -qmax 0 -aom-params lossless=1 \
  -cpu-used 6 -row-mt 1 -pix_fmt yuv420p -f ivf out.ivf
```

## Alternatives considered

**Keep raw `yuv420p`.** No decoder on the derived path, and O(1) seeking. The
cache is gitignored scratch sitting next to a multi-GB disc image, so 48.8 MiB is
not painful in itself. Rejected because the decoder arrives anyway the moment we
ship any video of our own, and once it exists, 41x is free.

**Compress the raw frames** with `zstd` or `lz4`. Genuinely competitive at the
default extent - `zstd -9` gives 1.66 MiB against AV1's 1.17 MiB, in under a
second, with no codec and per-frame indexing preserved. Rejected because it loses
badly on the whole intro (53.6 MiB against 33.0 MiB), and because it is a third
format that does nothing for authored content.

**FFV1.** Lossless, fast, and already in `ffmpeg`: 2.6 MiB for the default
extent. Rejected because there is no Rust decoder, which is the whole problem
this is meant to avoid.

**High-quality lossy AV1.** Smaller still, and visually indistinguishable.
Rejected because "indistinguishable" is a judgement that has to be re-made every
time someone touches the pipeline, and because it puts a quality knob into a
project whose asset conversions are supposed to be verifiable rather than
tasteful.

**Ship the decoder for H.264 instead** (`openh264`, `ffmpeg-next`). Would remove
the transcode entirely. Rejected for the reasons ADR-0004 already gave: both are
C, and H.264 raises a patent question that AV1 does not.

## Consequences

**Good.** The cache is 41x smaller at the default extent. The decoder the project
needs for its own content now exists, is used on a path that is exercised every
boot, and is covered by tests that run in CI without a disc image. No C toolchain
and no `nasm` enters the build. Lossless keeps ADR-0004's fidelity rule provable
rather than argued.

**A trap in that CI coverage.** The `av1` tests only compile when the feature is
on, and feature unification is what turns it on: they run under
`cargo nextest run --workspace`, which is what `just test` does, because
`oag-game` is in the graph. `cargo test -p oag-formats` on its own runs **zero**
of them and reports success. If CI is ever "simplified" to per-crate runs, the
decoder and its rewind path lose their coverage without a single test failing.

**Bad.** A decoder now runs on the derived path, which previously needed none.
Playback decodes on the render thread; at 480x272 there is roughly thirty times
the headroom needed, but the per-frame worst case inside a frame budget has not
been measured, only the whole-clip throughput. Seeking backwards costs a replay
from the start, which is invisible for a 260-frame loop and would not be for a
long one. `ffmpeg` builds without `libaom-av1` can no longer produce a cache at
all, where before any `ffmpeg` would do - the failure is reported and playback
falls back to the black backdrop, as it already did when `ffmpeg` was missing
entirely.

**Stale caches are left alone.** Old `*.raw` files do not match the new name and
are simply never read again. Deleting them is the user's call; `data/cache/` is
safe to remove wholesale at any time.

**Open question.** Whether to use the same path for video we author ourselves, or
whether ADR-0004's advice - that a title card is a texture and a fade, not a
video - makes the question moot. Nothing here commits us either way.
