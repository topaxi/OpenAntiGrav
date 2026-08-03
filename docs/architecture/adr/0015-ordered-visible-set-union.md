# ADR-0015: Order the visible-set union, and never rejoin authored alternatives

## Status

Accepted. Refines [ADR-0014](0014-authored-section-placement.md)'s decision
item 2: the union of visibility sources is now ordered and filtered, not
plain. Everything else in ADR-0014 stands.

## Context

ADR-0014 made placement authored and padding bit-wise, which fixed the
steady-state double-draw: while the craft races a section, its mask alone
decides, and no shipped mask names both halves of a LOD swap. But the
renderer still builds a **union** - craft mask, camera mask, padding bits -
for a chase camera the original does not cull to, and a union has no opinion
about exclusivity. Measured on Moa Therma: the racing line crosses from
section 51 (whose mask shows the far-LOD copy, 62) into section 26 (whose
mask shows the detail) right before the loop, and while the craft leads the
camera across that boundary the plain union contains both halves. For those
frames the copies z-fight exactly as the placement bug did, just briefly.
The same shape recurs on ten more circuits - 0 to 25 swap pairs per track
file, on both discs.

The relation "these two sections are alternatives" is not named anywhere on
the disc, but it is fully determined by two authored facts: no single mask
ever names the pair together, and their geometry shares bit-identical vertex
positions - the same surface exported twice. Moa Therma's loop pair shares
~4,900 positions; unrelated sections share none. Two weaker geometric tests
(bounding boxes touching, boxes mostly containing each other) were tried and
rejected measured: box-touch inflated one circuit's table to 155 pairs and
cut its visible set by a third with false positives, and containment cannot
tell a stacked copy from a tunnel interior nested inside its mountain -
rightly exclusive, wrongly filtered, since nested surfaces do not fight.

## Decision

At load, `oag_render::pvs::SwapConflicts` records every section pair that is
**mask-exclusive and shares at least four bit-identical vertex positions**.
Per frame, `VisibleSet::around` builds the set in priority order, and a later
source never contributes a bit that is an alternative of one already
accepted:

1. the craft's mask, whole - the source the original uses;
2. the camera's mask, minus alternatives of anything accepted;
3. the padding bits, minus alternatives of anything accepted.

Craft outranks camera because the camera frames the craft: what the shot
mostly shows is what the craft's region sees, and across a swap boundary the
craft's side is the one the original would show. The subordinated sources
lose only their conflicting bits; everything else still unions.

## Consequences

- The straddle windows close: entering or leaving a swap region can no
  longer draw both copies for the frames the craft and camera disagree.
  Pinned on real data in `magstrip_ground_truth.rs::far_lod_sections` - the
  unfiltered union provably contains both halves at Moa's 51/26 boundary,
  the filtered one only the craft's.
- Swap-side pops become instant at the boundary, as in the original, rather
  than blended across the straddle. That is a fidelity gain, not a cost.
- Detection is exact-position matching, so a copy re-exported with different
  vertices would evade it; no shipped pair does (the counts match the
  independent coincidence census on every circuit, 0-25 pairs per file,
  both discs, with 04 and 10 clean on PSP as the census predicted).
- Tier-one culling tightens again: ~13-26% of draw calls survive on the
  live camera-trailing average, against ~20-36% under the plain union,
  because vantage masks unioned in by padding were re-admitting whole LOD
  groups.
- The table costs 512 bytes and a load-time vertex walk; the per-frame cost
  is two `partners_of` fold-ORs over set bits.
