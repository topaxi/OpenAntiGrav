# Asset pipeline

## Shape

```
disc image (.chd / .iso)
        |  oag-disc
        v
   ISO 9660 files
        |  oag-formats
        v
  container entries (WAD, ...)
        |  oag-formats
        v
   decoded assets (textures, models, tracks, audio)
        |  oag-assets
        v
  normalised runtime types
        |
        v
   renderer / simulation
```

Each arrow is a separate, independently testable step. The point of the split is
that a bug is attributable: a wrong texture is either a container bug, a decode
bug or a normalisation bug, and the boundaries say which.

## Normalisation

The PSP and PS2 releases of Pulse ship different assets. PS2 has higher
resolution textures and larger video files; PSP uses swizzled texture formats
and ATRAC3 audio.

`oag-assets` converts both into the same runtime types. Downstream code never
branches on platform.

```sh
oag-game --assets psp      # PSP assets
oag-game --assets ps2      # PS2 assets
```

**Gameplay must be identical regardless.** Only presentation differs. If
switching asset sets changes how a ship handles, that is a bug: it means a
gameplay constant is being read from a place it should not be.

Where the two platforms genuinely differ in content rather than fidelity, the
difference gets documented in [comparisons](../comparisons/) before any code
accommodates it.

## Loading from originals only

A user points the engine at their own disc image, or at a directory previously
extracted with `oag-unpack`. Both work; the image is authoritative and the
directory is faster.

Most assets are decoded in process, every run, and nothing is cached. The
exception is an asset whose *decoder* is more expensive to reproduce than the
asset is to convert, which in practice means video: see
[ADR-0004's amendment](adr/0004-asset-pipeline.md#amendment-2026-07-26-convert-where-reproducing-the-decoder-is-the-wrong-trade).
Those go to `data/cache/`, keyed on the source entry and safe to delete.

**Converted assets are never redistributed**, cached or not. Converting is a
local operation on content the user already owns. See
[legal](../overview/legal.md).

## Tools

| Tool | Status | Purpose |
| --- | --- | --- |
| `oag-unpack` | working | Inspect and extract disc images |
| `oag-view` | M1 | Preview decoded assets |
| `oag-convert` | M1 | Convert assets to standard formats for inspection |
| `oag-repack` | later | Development only, for testing modified assets |

`oag-repack` is explicitly a development tool. It is not a modding pipeline and
is not supported as one.

## What is known so far

Very little, and that is the honest state of it. See the
[format status table](../formats/README.md) for the current inventory.

The one substantive finding is the [WAD container](../formats/wad.md), whose
directory structure is now readable and which appears in the same shape across
Pure PSP, Pulse PSP and Pulse PS2. That shared shape is the first concrete
evidence that a single asset pipeline can serve the whole lineage, which is the
premise the project is built on.
