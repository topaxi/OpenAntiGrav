# ADR-0024: Decode in process where a light library exists; `ffmpeg` is the last resort

## Status
Accepted. Narrows [ADR-0019](0019-atrac3plus-out-of-process.md) to the codec it
was written for, rather than superseding it.

## Context

[ADR-0019](0019-atrac3plus-out-of-process.md) put ATRAC3+ decoding out of
process behind `ffmpeg`, into the cache [ADR-0008](0008-av1-movie-cache.md)
built. Its reasoning was specific and it still holds: **no ATRAC3+ decoder
exists in the Rust ecosystem at all**, GStreamer's codec map has no entry for
it, and reproducing the codec is a project in itself.

That reasoning was then applied by habit rather than by re-derivation. When
Wipeout HD's music turned out to be **plain MPEG-1 Layer III** - 36 files under
`/data/music/`, 48 kHz stereo, in no console container whatsoever - the obvious
move was to hand the bytes to the `ffmpeg` path that was already there, because
it probes by content and needed no new code.

That is the wrong default, and the difference matters to a player rather than
only to a reader:

- **`ffmpeg` is an external program.** It is not a build dependency, it is a
  runtime one. A machine without it on `PATH` gets silence and a line of
  explanation - which is the honest degradation ADR-0019 chose, and the right
  one when the alternative is nothing at all.
- **For MP3 the alternative is not nothing.** Several pure-Rust decoders exist.
  Routing MP3 through `ffmpeg` would have made Wipeout HD's music depend on an
  external program for no reason other than that the pipe was already plumbed.

A first attempt also went the other way and was rejected mid-flight: a
hand-written MPEG frame-header and Xing-tag reader, about 120 lines, to get
durations cheaply while leaving the decode with `ffmpeg`. That is the third
wrong answer. `oag-formats` hand-rolls every *Wipeout* format because nothing
else will; MPEG is a published standard whose failure mode is
plausible-sounding noise, and it is exactly the case
[`miniz_oxide`](0008-av1-movie-cache.md) and `re_rav1d` are already in the tree
for.

## Decision

**Decode in process, with a library, whenever a light one exists. Reach for
`ffmpeg` only when neither we nor anyone else has an implementation to take.**

In order of preference:

1. **A pure-Rust library**, if one exists and is maintained. `symphonia` for
   MPEG, `re_rav1d` for AV1, `miniz_oxide` for deflate.
2. **Our own decoder**, for a format that is *Wipeout's own* - `.vex`, `.pob`,
   `.wad`, `SBlk`, `.rcsmodel`. Nobody else will ever write these, and their
   failure modes are checkable against the disc.
3. **`ffmpeg`, out of process, through the cache.** ATRAC3+ and the video path.
   Only here, and only because 1 and 2 are both unavailable: not because the
   plumbing exists.

A **published standard we did not invent** must never land in category 2. That
is the rule the frame-header reader would have broken.

### The license bar a dependency has to clear

Every dependency in this workspace was permissive until GStreamer;
[ADR-0017](0017-gstreamer-native-video.md) reasoned that departure explicitly
rather than letting it pass in a manifest, and this ADR does the same for the
second one.

`symphonia` is **MPL-2.0**, the tree's first copyleft dependency. MPL-2.0 is
*file-level* copyleft: it covers modifications to symphonia's own sources, and
§3.3 expressly permits combining it into a Larger Work under other terms. This
project neither modifies nor vendors it, so the workspace's own
`MIT OR Apache-2.0` is unaffected - the same structural argument ADR-0017 makes
for linking LGPL GStreamer, applied to a weaker copyleft.

`lofty` (`MIT OR Apache-2.0`) was the license-matching alternative and was
rejected on architecture rather than licence: it reads metadata and does not
decode, so taking it would have added a dependency *and* left `ffmpeg` decoding
MP3 - two mechanisms where one would do.

**The bar, stated so the next case does not have to re-derive it:** prefer a
permissive licence; a weak or file-level copyleft (MPL, LGPL) is acceptable
where it is linked unmodified and the alternative is materially worse; a strong
copyleft that would reach this project's own sources is not acceptable. There
is no `cargo-deny` policy enforcing this, and this ADR is deliberately not
proposing one for two dependencies - it is the record a third would be checked
against.

### And no cache for an in-process decode

[ADR-0008](0008-av1-movie-cache.md)'s cache exists because an out-of-process
`ffmpeg` run costs seconds. `crates/game/src/mp3.rs` decodes far faster than
real time in process, so caching would be a second copy of every track on disk
to save a fraction of a second. The cache follows the *cost*, not the media
type.

## Consequences

- **Wipeout HD's music plays with no `ffmpeg` installed.** The PSP titles still
  need it and still say so when it is missing.
- **Two decode paths, chosen by content and not by title.**
  `oag_game::music::decode` offers a blob to `crate::at3` and then to
  `crate::mp3`; whichever reads it is what it was. A title package names files
  and never describes them, which is what keeps `oag-title` a vocabulary.
- **`symphonia` is a real dependency with a real surface**, taken with
  `default-features = false, features = ["mp3"]` so that the other twenty-odd
  formats and the SIMD paths stay out of the build.
- **ADR-0019 is narrowed, not overturned.** Its measurements about ATRAC3+ are
  unchanged and its conclusion still governs that codec. What changes is that
  "use `ffmpeg`" is no longer inherited by the next format that turns up.
- **The rule is not free.** A future format with a *heavy* Rust decoder, or one
  with several immature ones, is a judgement call this does not settle. It
  settles the default and the order of preference.

## Alternatives considered

- **Everything through `ffmpeg`.** One mechanism, and the plumbing existed.
  Rejected: it makes a runtime external program the price of a codec that does
  not need one.
- **Everything in process.** Would mean writing an ATRAC3+ decoder, which
  ADR-0019 already measured as out of scope.
- **`symphonia` for ATRAC3+ too.** It has no ATRAC3+ decoder either; the gap
  ADR-0019 documents is the ecosystem's, not `ffmpeg`'s.
- **`lofty` for metadata plus `ffmpeg` for decode.** Licence-matching and
  architecturally worse; see above.
- **A hand-written MPEG header reader.** Cheap, and the wrong category. See
  Context.

## See also

- [ADR-0019](0019-atrac3plus-out-of-process.md) - the ATRAC3+ decision this narrows
- [ADR-0008](0008-av1-movie-cache.md) - the cache, and why it follows cost
- [ADR-0017](0017-gstreamer-native-video.md) - the first non-permissive dependency, reasoned the same way
- [`hd-status.md`](../../formats/hd-status.md#music-plain-mp3-declared-the-way-the-psp-titles-declare-theirs) - what HD's music is
