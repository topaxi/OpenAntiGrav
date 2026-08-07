# ADR-0016: Platform-native H.264 decode as an accelerated path, on Linux first

## Status

Superseded by [ADR-0017](0017-gstreamer-native-video.md). Kept as the record
of why hand-rolled `libva` was tried first and what it cost - the decision
itself no longer holds; nothing in `crates/game` still implements it.

Originally: Accepted. Additive: does not supersede
[ADR-0004](0004-asset-pipeline.md) or [ADR-0008](0008-av1-movie-cache.md), which
stand unchanged as the default and universal fallback.

## Context

An external request (`VIDEO_REFACTOR.md` at the repository root) asked for the
`.PMF` movie pipeline to be replaced with platform-native hardware H.264
decode - Media Foundation on Windows, VideoToolbox on macOS, MediaCodec on
Android, VA-API on Linux - with a software H.264 decoder (OpenH264 named
explicitly) as the fallback where no platform decoder exists.

That request only partly fits what is actually here:

- Only the PSP disc's `.PMF` movies are H.264. The PS2 disc's `.PSS` (a raw
  MPEG-2 program stream) and `.IPF` (Sony's proprietary intra-only IPU
  variant, see [`docs/formats/ipf.md`](../../formats/ipf.md)) are not, and no
  platform H.264 API can touch them. Whatever changes for the PSP path, the
  PS2 path is untouched.
- The software-fallback half of the ask directly re-opens a question
  ADR-0004 and ADR-0008 already answered. ADR-0004's asset-pipeline table
  rejected `openh264`/`ffmpeg-next` as in-process decoders: both are C, and
  "H264 raises a patent question that AV1 does not" for a project dual
  licensed MIT OR Apache-2.0. ADR-0008's "Alternatives considered" restates
  the same rejection by name. That reasoning does not weaken just because the
  request repeats it - it is decided, and this ADR is additive precisely so
  it does not need re-litigating.

What changed since ADR-0008: a platform decoder calls an OS-owned decoder. It
bundles no patent-encumbered code and ships no H.264 content - the content is
the player's own disc, per [`legal.md`](../../overview/legal.md) - so the
concern ADR-0004 raised does not apply to it. That is the actual opening this
ADR uses; it is narrower than the original request.

## Decision

**Add an optional, Linux-only VA-API decoder for the PSP's `.PMF` H.264
content. Drop the software-fallback ask entirely: when no platform decoder is
available, playback falls back to the existing ffmpeg-transcoded AV1 cache,
unchanged.** The result is a three-tier ladder, where the bottom tier already
existed and is documented at `justfile:80-82`:

```
platform HW decode (VA-API, Linux, this ADR)
  -> ffmpeg-transcoded lossless AV1 cache, rav1d decode (ADR-0008, all platforms, always for PS2)
    -> black backdrop / no picture (existing, ffmpeg unavailable)
```

Windows (Media Foundation), macOS (VideoToolbox) and Android (MediaCodec) are
left as a documented seam - a `cfg`-gated probe function with a `None` arm -
rather than stubbed implementations. None of those three platforms is built,
linted or tested anywhere in this repository or its CI today (see
[`workspace-layout.md`](../workspace-layout.md) and the CI matrix in
`.github/workflows/ci.yml`), so a stub for any of them would be uncompiled,
untestable code with nothing to catch it going stale.

### Where the decoder lives

`VideoDecoder`, a trait shaped after `oag-disc`'s `SectorSource` (boxed,
`Debug` as a supertrait for the same overlap reason), sits in `oag-game`'s
`movie` module - not a new crate. `oag-game`'s own `Cargo.toml` already
documents that "the composition root is the only crate that plays video", and
no video/media crate is anticipated in `workspace-layout.md`'s table. The AV1
cache path is re-expressed through the trait as `Av1CacheDecoder`, a thin
wrapper with no behaviour change (verified byte-identical against the real
disc via the existing ground-truth tests). The VA-API path is a second
implementation, `vaapi::VaapiDecoder`, added behind a `vaapi` Cargo feature,
default off, so `cargo build --workspace` and CI never touch `libva`.

### Why hand-rolled `libva`, not GStreamer

VA-API's H.264 entry point is slice-level (VLD): confirmed directly from the
installed `/usr/include/va/va.h` - `VAPictureParameterBufferH264` and
`VASliceParameterBufferH264` expect SPS/PPS-derived fields and a caller-built
DPB (`ReferenceFrames[16]`, `RefPicList0/1[32]`). `libva` parses no
bitstream; an SPS/PPS/slice-header parser and reference-picture-list
management are required regardless of which VA-API binding is used.

GStreamer's `va` plugin (`gst-plugins-bad`) already has that parser and DPB
logic, and would additionally cover the PS2's `.PSS` (a standard MPEG-2
program stream, and MPEG-2 is one of the oldest, most universal VA-API
profiles) through the same dependency - a real argument, considered
explicitly. Rejected for this pass:

- GStreamer core is LGPL-2.1-or-later. Every other dependency in this
  workspace is permissive (`re_rav1d` BSD-2, `cros-libva` BSD-3,
  `h264-reader` MIT/Apache) - this would be the first copyleft dependency.
- It is a materially heavier runtime: multiple shared libraries, `glib`, and
  a plugin discovery mechanism (`GST_PLUGIN_PATH` and friends), versus one
  `dlopen`-style system library for `libva` alone.
- The word "GStreamer" is exactly what `VIDEO_REFACTOR.md` asked not to
  introduce as a dependency. Gating it as optional and Linux-only would
  technically satisfy "not mandatory," but not the request's evident intent.
- The measured content this decoder actually has to handle - the PSP intro,
  probed directly via `ffprobe` against `data/images/pulse-psp-usa.chd` - is
  Main profile, level 2.1, 270 frames of which 265 are P and 5 are I, **zero
  B-frames**. That is a sliding-window DPB over a single reference list, not
  general-purpose H.264 decode, which narrows what the hand-rolled parser
  actually has to cover.
- `.IPF` needs ffmpeg (or `gst-libav`, which is ffmpeg linked in-process
  rather than shelled out to) either way - GStreamer does not unify the
  PS2 story, only the PSP and PS2-`.PSS` one.

Chosen: `cros-libva` (BSD-3-Clause, ChromeOS's safe wrapper over `libva`,
confirmed by reading its vendored source to expose real
`PictureParameterBufferH264`/`SliceParameterBufferH264`/`IQMatrixBufferH264`
wrappers with a 1:1 field mapping onto the raw VA-API structs) for the FFI
boundary, and `h264-reader` (MIT/Apache-2.0, pure Rust, no `libclang`
build-time dependency) for SPS/PPS/slice-header parsing. Both are Linux-only,
`vaapi`-feature-gated dependencies in `crates/game/Cargo.toml`.

### The vendored `cros-libva` patch

`cros-libva` 0.0.13 (last published December 2024) does not compile as
published against this host's `libva` 1.24: `VAEncPictureParameterBufferVP9`
gained two fields (`seg_id_block_size`, `va_reserved8`) in a `libva` release
since. Nothing in this codebase calls VP9 encode - the struct is dead weight
we never construct - but Rust still requires every field initialised, and the
crate has no feature to compile it out. Patched by zero-filling the two
fields at the one call site; see [`vendor/README.md`](../../../vendor/README.md)
and the patch comment in `vendor/cros-libva-0.0.13/src/buffer/vp9.rs` for the
exact change and the condition for dropping it (an upstream release that
includes the fix).

## Alternatives considered

**Ship the software fallback the request asked for (OpenH264/ffmpeg-next).**
Rejected; see Context. ADR-0004 and ADR-0008's reasoning is unchanged by this
ADR.

**GStreamer's `vaapih264dec`/`va` plugin.** Considered twice, once for PSP
alone and once with PS2's `.PSS` folded in. Rejected both times; see
"Why hand-rolled `libva`, not GStreamer" above.

**A new `oag-video` crate.** Rejected on the same grounds ADR-0008 already
settled: no video/media crate is anticipated by `workspace-layout.md`, and
`oag-game`'s own `Cargo.toml` already states it is the only crate that plays
video. Introducing one now would be a second, unrelated architectural change
bundled into this one.

**Stub Windows/macOS/Android implementations alongside the real Linux one.**
Rejected: none of those three platforms is built or tested anywhere in this
repository today, so a stub would be unverifiable dead code. The `cfg`-gated
probe function is the seam; the implementations are separate future work,
each needing the same kind of platform-specific verification this ADR did for
Linux (what does the actual API require, what does the actual content need).

## Consequences

**Good.** The PSP's H.264 content can play through the platform's own decoder
where one is available, without any transcode step or cache file for that
path - the accelerated tier bypasses `movie::run_ffmpeg` entirely, demuxing
straight from the `.PMF`'s H.264 elementary stream. Falls back to exactly
today's behaviour (ffmpeg-transcoded AV1 cache, or no picture) everywhere
else, so nothing regresses for PS2 content, for non-Linux platforms, or for a
Linux machine without a VA-API-capable driver.

**Bad.**

- `vendor/cros-libva-0.0.13/` puts a full third-party crate's source in the
  repository, with a maintenance obligation to drop it once upstream catches
  up. Tracked in `vendor/README.md`, not just this ADR, so it is
  discoverable without reading every ADR first.
- Building with `--features vaapi` now also needs `libclang` (for
  `cros-libva`'s `bindgen` build step) in addition to `libva`'s development
  headers - a build-time requirement beyond the runtime `libva` dependency,
  and beyond what any other optional feature in this workspace needs.
- The decoder's own `unsafe_code` is scoped to the `vaapi` module with a
  written justification (the FFI boundary `cros-libva` does not itself
  cover), an exception to the workspace's `unsafe_code = "deny"` lint that
  every other crate in the tree is free of.
- Two H.264-adjacent code paths now exist for the same content - the
  hand-rolled slice/DPB logic here, and nothing shared with PS2's MPEG-2
  path, which is a different codec with its own (simpler, not yet written)
  parser if it is ever given the same treatment. This ADR does not unify
  them, deliberately - see "Alternatives considered."
- `libva`'s H.264 entry point is slice-level and undocumented in the
  friendly sense: the exact semantics of `slice_data_bit_offset` (bits from
  the start of the NAL unit, in the RBSP domain, after emulation-prevention
  removal - confirmed from `va.h`'s own comment, cross-checked against
  `ffmpeg`'s `vaapi_h264.c`) are the kind of subtlety that is easy to get
  wrong silently. The conformance test this ADR's implementation adds
  (byte-identical against the same `ffmpeg` reference `the_cache_is_lossless`
  already uses, since H.264's inverse transform and deblocking are
  bit-exactly specified) exists specifically to catch that.
