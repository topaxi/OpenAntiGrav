# ADR-0004: Load from originals, normalise per platform

## Status

Accepted.

## Context

Pulse shipped on PSP and PS2 with different assets: different texture
resolutions and formats, different audio codecs, different video containers. The
project must support both, and must not distribute either.

There is a choice about *when* platform differences are resolved: at load time,
or throughout the codebase.

## Decision

**Load from the user's own originals.** Either a disc image or a directory
previously extracted with `oag-unpack`. Nothing is redistributed and nothing is
cached to disk by default.

**Normalise at the boundary.** `oag-assets` converts PSP and PS2 assets into the
same runtime types. Nothing downstream branches on platform.

```sh
oag-game --assets psp
oag-game --assets ps2
```

**Gameplay is identical across asset sets.** Only presentation differs. If
switching asset sets changes handling, that is a bug: a gameplay constant is
being read from the wrong place.

Where the platforms differ in *content* rather than fidelity, the difference is
documented in [comparisons](../../comparisons/) before any code accommodates it.

## Alternatives considered

**Ship converted assets.** Simplest for users, and the conversion happens once
rather than on every load. Rejected outright: it distributes copyrighted
content. Not negotiable.

**Convert to an intermediate format on first run, then cache.** Faster startup
after the first launch, and a clean separation between decoding and rendering.
Deferred rather than rejected. It adds a cache to invalidate and a second format
to maintain, and until the formats are actually decoded there is nothing to
convert. Worth revisiting in M1 if load times prove painful.

**Branch on platform throughout the codebase.** Rejected. It puts
`if psp { ... } else { ... }` into rendering, gameplay and audio, and guarantees
the two paths drift apart. The bug it produces - PS2 assets behaving subtly
differently in play - is exactly the one the normalisation boundary prevents.

**Support only PSP assets.** Genuinely tempting: PSP is the primary reverse
engineering target and supporting one path is half the work. Rejected because
the PS2 release is a second, independent view of the same game, and disagreements
between them are evidence. Discarding it would discard the cross-validation the
project's verification story depends on.

## Consequences

**Good.** No legal exposure. One code path downstream of the loader. The PS2
assets serve as cross-validation for the PSP findings. Users with either release
can play.

**Bad.** Every format must be decoded twice, once per platform, which is real
duplicated work in M1. Normalisation means we sometimes upconvert PSP assets
into a representation richer than the original, and we must be careful that this
does not change appearance. Loading from a compressed disc image is slower than
loading pre-converted files, and users will notice.

**Open question.** Whether the two releases' *gameplay data* - handling
constants, track splines, AI parameters - are identical is not yet known. If
they differ, "gameplay is identical across asset sets" needs qualifying, and the
difference becomes a finding worth documenting rather than a bug to fix.
