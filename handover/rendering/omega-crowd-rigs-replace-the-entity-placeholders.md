---
status: re-first
titles: [omega]
updated: 2026-11-20
summary: "Omega draws nothing at the track's crowd entities; the original instances animated Data/Crowd/Rigs there (skinned, 82 files, 6 clips)"
size: L (40)
difficulty: hard (55)
---

# Omega's crowd rigs: what replaces the crowd-entity placeholders

2026-11-20, lane `omega-sol-silhouettes` (`docs/formats/omega-status.md`, "The crowd-entity shapes"). The track's 203 (Sol), 907
(Amphiseum), 758 (Vineta K) `crowdentity` mesh objects (flags `0x0401`) are placeholders the original's loader replaces with rig
instances; ours no longer draws the placeholders (they drew a flat `Colour` white), and draws nothing in their place. Under Sol's
silhouette mask ours is 22 grey too dark (26.5 against 48.3) because the stand behind them is empty.

## Open

1. **The rigs**: `Data/Crowd/Rigs` (`male01..10`, `female01..10` and `_shadow` twins: `.rcsmodel` + `.rcsskeleton`, 82 files with
   `LODs`), `Data/Crowd/Animations` (6 `.rcsanimclip`), materials `Crowd_Diffuse_Normal_BoneCount4(_NormalizeTS)`,
   `Crowd_Diffuse_Normal_Shadow_BoneCount4_NormalizeTS`, `BasicMaterial_BoneCount4`. The model reader leaves the rigs' 82
   relocation pointers unpaired (`data00_rcsmodels_decode_with_only_the_crowd_rigs_unpaired`).
2. **Which rig where**: `FUN_0162a390` (v1.07 eboot) reads the digits and `w`/`s` letters after `CrowdEntity` in each
   object's name, picks a group, draws `rand()` for the instance and pushes a record (`FUN_0163d3c0`). The letters' meaning and
   the record's use are unread; `|crowdentity%d|LOD1|LOD1Shape` (60 pairs) are `LODs.rcsmodel`'s shapes.
3. **The draw bits** it clears on the placeholder (`& 0xfffffff9` animated, `& 0xfffffff8` not): their meaning is unread (70), and
   whether the non-animated mode (`Animation.Animated crowd in split-screen` off) draws the placeholder is unknown.
4. The skinned materials' programs (`third_axis_is_eye` reads them as not eye-framed) are not wired; the skinning and clip playback
   exist for node-bound props only.

## Next Steps

- Read `FUN_0162a390`'s tail (from the `Data/Crowd/Rigs/` use at `0x0163138a`) and `FUN_0163d3c0` for the instance record, then
  decide the instance count and clip choice; draw one rig per entity at its node's matrix.
- Score on the same mask: `OAG_SKIP_SLOT=5` on the pre-change build gives it (Sol 23,774 px).
