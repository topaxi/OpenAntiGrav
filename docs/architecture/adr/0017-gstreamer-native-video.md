# ADR-0017: GStreamer for platform-native H.264 decode, not a hand-rolled `libva` decoder

## Status

Accepted. Supersedes [ADR-0016](0016-platform-native-h264-decode.md) on how the
Linux accelerated tier is implemented. Does not change ADR-0016's own framing:
still additive, still does not supersede [ADR-0004](0004-asset-pipeline.md) or
[ADR-0008](0008-av1-movie-cache.md), which stand as the default and universal
fallback.

## Context

ADR-0016 chose to hand-roll a VA-API H.264 decoder: parse SPS/PPS/slice
headers with `h264-reader`, build the H.264 DPB and reference-picture-list
logic from scratch, and drive `libva`'s slice-level (VLD) entry point directly
through a vendored, patched `cros-libva`.

That implementation was built end to end - dependency vendoring, DPB sliding
window with unit tests, SPS/PPS/slice-header parsing verified against 100% of
the real intro stream's slices, full VA-API surface/context/config/buffer
wiring - and ran on real hardware (Intel Arc, `iHD` driver) without any
VA-API-level error at any stage: config creation, surface pool, context,
per-frame buffers, `begin`/`render`/`end`/`sync` all succeeded. The decoded
picture was still wrong. Four specific causes were ruled out one at a time
(zero-copy `vaDeriveImage` vs `vaCreateImage`+`vaGetImage`, a missing
`VAConfigAttribRTFormat` query, NV12 image layout/addressing, an entropy-mode
mislabelling) without finding the actual fault. This is the failure mode a
slice-level API is built to allow: nothing in the call chain reports "your
field mapping is subtly wrong," because libva does no bitstream parsing of its
own to check the caller's work against.

At that point the person maintaining this codebase does not have, and does
not want to build, the H.264 domain expertise to keep debugging - or later
maintaining - a from-scratch decoder: bitstream parsing, DPB/reference-list
bookkeeping, picture-order-count arithmetic, and scaling-list handling are all
exactly the kind of "vast amount of custom code of fairly complex material"
this project should not be carrying for a problem a well-maintained library
already solves. That is a maintainability argument ADR-0016 did not have
available to it - it was written before any of the above was known - not a
reversal of anything ADR-0016 got wrong given what it knew at the time.

### Validating the pivot before writing any of it

Before rewriting anything, the demuxed elementary stream that produced the
hand-rolled decoder's wrong pixels was run through a plain `gst-launch-1.0`
pipeline (`filesrc ! h264parse ! decodebin ! videoconvert ! filesink`,
software `avdec_h264` - this machine does not have the VA-API GStreamer
plugin, `gst-plugin-va`, installed) and compared byte-for-byte against the
same `ffmpeg`-decoded reference `the_cache_is_lossless` already uses. It
matched exactly. Two things follow: the demux itself was never the bug (ruled
out, not just assumed), and GStreamer decodes this content correctly, which
is the actual precondition for choosing it over debugging further.

## Decision

**Replace the hand-rolled `vaapi::VaapiDecoder` with a `gst::GstDecoder` that
drives a GStreamer pipeline and does no H.264 parsing of its own.** The
three-tier fallback ladder from ADR-0016 is unchanged in shape:

```
platform decode via GStreamer (Linux, this ADR)
  -> ffmpeg-transcoded lossless AV1 cache, rav1d decode (ADR-0008, all platforms, always for PS2)
    -> black backdrop / no picture (existing, ffmpeg unavailable)
```

Only the top tier's implementation changes. `VideoDecoder`, `VideoFrame`,
`PixelFormat`, and the `Av1CacheDecoder` wrapper from ADR-0016's Phase A are
untouched - they were already a clean, zero-behaviour-change abstraction, and
that is exactly why this pivot is cheap: `GstDecoder` is a second, independent
implementation of the same trait.

### The pipeline

`appsrc` (the whole demuxed elementary stream, pushed once) → `h264parse` →
`decodebin` → `videoconvert` → `appsink` capped to `video/x-raw,format=I420`.
`decodebin` is left to pick the decoder element itself rather than this code
naming `vah264dec` and ranking it above a software one - that ladder (hardware
first, software fallback) is exactly what `decodebin`'s own autoplugging
already does by element rank, and writing rank-selection code here would be
more of the custom code this ADR exists to avoid. `GstDecoder::open` decodes
every wanted frame eagerly into memory and tears the pipeline down; `frame`
and `rewind` just index into the result. That is simpler than keeping a live
pipeline and seeking it, and affordable at cutscene scale (the PSP intro's 270
frames at 480x272 I420 is about 53 MiB).

### The software-decode question this reopens, and why it's still fine

If no VA-API-capable GPU/driver is present, `decodebin` will select
`avdec_h264` from `gst-libav` - which is `ffmpeg`'s `libavcodec`, linked
in-process. That is software H.264 decode, the exact thing ADR-0004 and
ADR-0008 rejected shipping as a bundled dependency. The reason it is fine
here is the same reason ADR-0016 gave for VA-API itself: `gst-libav` is a
system-installed runtime plugin this code merely calls into via GStreamer's
plugin discovery, not code vendored or statically linked into this
repository or its binary. Nothing about H.264 decoding ships as source or as
a linked object in this workspace either way. This is stated explicitly
rather than left implicit, and no plugin-ranking code is added to prevent
`decodebin` from ever choosing it - doing so would itself be more custom
code, against this ADR's own reasoning.

### The license question this adds

GStreamer core is LGPL-2.1-or-later - ADR-0016 flagged this as a reason to
reject GStreamer, and it is a real change: every other dependency in this
workspace is permissive (`re_rav1d` BSD-2-Clause, `wgpu` MIT/Apache-2.0). The
`gstreamer`/`gstreamer-app`/`gstreamer-video` Rust bindings this crate
depends on are themselves MIT/Apache-2.0; only the GStreamer shared libraries
they call into at runtime are LGPL. Dynamic linking against an LGPL library
without modifying it does not require this project's own code to adopt LGPL
terms - the standard basis on which every LGPL library is used from
permissively-licensed applications. `native-video` links dynamically (via
`pkg-config`-discovered system libraries, `dlopen`-style at the OS level, no
static linking, no vendored GStreamer source) and modifies nothing in
GStreamer itself, so this project's own MIT/Apache-2.0 dual license is
unaffected. This is a materially different situation from bundling an H.264
codec's source, which is what ADR-0004/ADR-0008 rejected.

### What was removed

- `crates/game/src/movie/vaapi.rs` (~950 lines) and
  `crates/game/src/movie/dpb.rs` (~230 lines, unit-tested, committed) - the
  hand-rolled slice parsing, DPB, and VA-API buffer construction.
- `vendor/cros-libva-0.0.13/` and `vendor/h264-reader-0.8.0/` - both vendored
  solely to support the decoder above; neither is needed once it's gone.
- Both `[patch.crates-io]` entries in the root `Cargo.toml`.

About 1200 lines of custom H.264/VA-API code and two vendored crate forks are
gone. `crates/game/Cargo.toml`'s Linux-only, default-off feature is renamed
`vaapi` → `native-video` to stop naming an implementation detail; its shape
(gated, Linux-only, opt-in) is unchanged.

## Alternatives considered

**Keep debugging the hand-rolled `libva` decoder.** The pipeline ran without
any API-level error, so the remaining bug is a silent, wrong field mapping
somewhere across several hundred lines of manually-populated VA-API structs -
there was no shorter path forward than field-by-field comparison against a
reference decoder's own internal values (e.g. `ffmpeg`'s `vaapi_h264.c`).
Rejected: even successful debugging would leave the exact maintenance burden
this ADR's Context section describes, for a decoder this project's
maintainer does not have the domain expertise to keep correct going forward.

**Raw `libva-sys` FFI instead of `cros-libva`.** Would not have changed
anything - the bug was in bitstream-to-parameter-buffer field mapping, not in
`cros-libva`'s wrapper, and raw FFI is more custom code, not less.

**A ranked/forced hardware decoder** (naming `vah264dec` explicitly and
failing over to `avdec_h264` in this code rather than letting `decodebin`
autoplug). Rejected: `decodebin`'s own rank-based autoplugging already does
this, and duplicating it here is exactly the kind of code this ADR is trying
to not carry.

## Consequences

**Good.** The custom H.264 bitstream/DPB/VA-API code and its vendored
dependencies are gone; what remains is a pipeline-construction shim around a
library that already has a wide, actively-maintained H.264 decode surface
across hardware and software backends. The existing `the_cache_is_lossless`
ground-truth pattern (GStreamer decode vs. `ffmpeg` CLI reference,
byte-identical) already caught what the hand-rolled decoder could not get
right, on the first real run.

**Bad.**

- GStreamer is now a real, if optional and Linux-only, runtime dependency:
  `pkg-config` plus `gstreamer-1.0`/`gstreamer-app-1.0`/`gstreamer-video-1.0`
  development headers are needed to build with `--features native-video`,
  and the target system needs GStreamer core plus `gst-plugins-base`,
  `gst-plugins-good`/`bad` (for `h264parse`/`decodebin`), and either
  `gst-plugin-va` or `gst-libav` installed at runtime for the pipeline to
  produce a decoder element at all. None of this touches the default build.
- The first LGPL dependency in this workspace, discussed above under "The
  license question this adds." Acceptable because linking is dynamic and
  nothing is modified, but worth a reader's attention if this project's
  licensing posture is ever audited.
- `decodebin`'s decoder choice is opaque from this code's side: which element
  actually decodes a given frame (hardware VA-API vs. software `avdec_h264`)
  depends on what's installed on the machine running it, not on anything
  this crate controls or can assert in a test beyond "the output is correct
  either way."
- PS2's `.PSS` (MPEG-2 program stream) is GStreamer-decodable by the same
  pipeline shape and was raised as an argument for GStreamer during ADR-0016
  before being explicitly deferred both times. It remains deferred here too:
  nothing in this ADR touches `open_mpeg2_ps` or `open_ipuf`, which still
  always go through the AV1 cache. Left as clearly-scoped future work rather
  than folded in, so this pivot stays about fixing the PSP path, not about
  also picking up PS2 in the same change.
- ADR-0016 stays in the tree, marked superseded rather than deleted or
  rewritten, per this project's ADR convention - it is still the accurate
  record of what was tried, on what evidence, and why it did not pan out.
