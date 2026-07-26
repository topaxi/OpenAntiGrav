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
previously extracted with `oag-unpack`. Nothing is redistributed, and nothing is
cached to disk unless caching is what makes the asset usable at all: see the
[amendment](#amendment-2026-07-26-convert-where-reproducing-the-decoder-is-the-wrong-trade).

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

**Revisited, and adopted for video** (see the amendment below). Load time was
never the argument that mattered.

**Branch on platform throughout the codebase.** Rejected. It puts
`if psp { ... } else { ... }` into rendering, gameplay and audio, and guarantees
the two paths drift apart. The bug it produces - PS2 assets behaving subtly
differently in play - is exactly the one the normalisation boundary prevents.

**Support only PSP assets.** Genuinely tempting: PSP is the primary reverse
engineering target and supporting one path is half the work. Rejected because
the PS2 release is a second, independent view of the same game, and disagreements
between them are evidence. Discarding it would discard the cross-validation the
project's verification story depends on.

## Amendment, 2026-07-26: convert where reproducing the decoder is the wrong trade

The original decision said "nothing is cached to disk by default", and read as
though every asset should be decoded in process, every run. That holds for the
formats decoded so far, which are cheap: a `.mip` texture or a `.vex` mesh is a
few hundred lines of parsing and no dependencies.

**It does not hold for video, and video is where it breaks.** A PSP `.PMF` is
MPEG-4 AVC plus ATRAC3+. Playing one in process means an H.264 decoder in the
workspace, which is either a large non-Rust dependency or a year of work, to show
two logo cards at boot. The asset is not the problem; reproducing its *decoder*
is.

So the rule is sharpened rather than reversed:

**Read from the user's originals, and convert at the boundary. Where a
conversion is expensive or needs a tool we should not vendor, do it once and
cache the result.**

- The cache lives under `data/cache/`, already gitignored, alongside the disc
  images it was derived from.
- It is **derived content and it is never committed, never redistributed and
  never a build input**. ADR-0006 is unchanged and unqualified: a converted
  texture or transcoded video is still game content. The only thing that changed
  is *when* we decode, not what we may ship.
- Cache entries are keyed on the source entry, so a different disc image cannot
  silently reuse another's output, and deleting the cache is always safe.
- External tools are optional, never required to build. If `ffmpeg` is absent the
  game must still run and say plainly what is missing.

This also generalises the other way, and deliberately: an asset may be converted
into a representation **more modern than the original** rather than merely a
different encoding of it. We already do this without naming it. `.vex` meshes are
decoded from pre-batched PSP display lists into portable vertex buffers, because
GE command lists cannot be replayed on a modern GPU. Textures are expanded from
4bpp palettes to RGBA8888. Those are the same move.

The constraint on it is fidelity, not purity: a conversion may not change how the
game *plays*, and where it changes how the game *looks* that has to be a
deliberate, documented choice rather than a side effect. Normalisation that
quietly improves filtering or gamma is the failure mode to watch, because it is
invisible in code review and obvious side by side with the original.

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
