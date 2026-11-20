---
status: re-first
titles: [omega]
updated: 2026-11-20
summary: "Omega crowd rigs stand at every crowd entity in the bind pose (loader group table read); the clip, density and LOD shapes are open"
size: M (35)
difficulty: medium (45)
---

# Omega's crowd rigs: pose, density and the original's flag bits

2026-11-20, lane `omega-crowd-rigs` (`docs/formats/omega-status.md`, "The crowd rigs"; the pairing in `docs/formats/2048-rcsmodel.md`).
Landed: the 82 unpaired pointers pair (PS4 gap 200), and `psp2::crowd_rigs` draws one `Data/Crowd/Rigs` figure per
`crowdentity` shape (Sol 203, Amphiseum 907, Vineta K 758) at its node, in the bind pose, the rig chosen by the loader's own
group table (digits, `s`, `w` letters; read from `FUN_0162a390` and `FUN_0163b720`). Sol's slot-5 mask: mean 26.7 to 29.1
against 48.5, bias -21.7 to -19.3, mean abs 35.8 to 36.8 (worse), whole frame 28.65 to 28.66.

## Open

1. **The pose.** The figures stand with arms out (bind pose). The group's clip (`male_01_folded_arms`,
   `male_02..04_frontrow_aggressive`, `male_05_aggressive_long`, `female_05_cheering`; `Data/Crowd/Animations`, 6 `.rcsanimclip`) is not
   played. Skin weights and bone indices are decoded (`SubMesh::skin_weights`/`skin_indices`); the skeleton and clip readers exist.
2. **Density.** The original's stands look fuller (hundreds of figures on Sol's left stand against about 100): whether an entity stands
   for more than one instance is unread (the instance record is `{group, entity, scale, start offset}`; the reciprocal of the node's
   rms scale, `fVar25`, is stored in it and its use is unread).
3. **The draw bits** `FUN_0162a390` clears on the placeholder's flag word (`+0x24`: `& 0xfffffff9` animated crowds on, `& 0xfffffff8`
   off) and what draws in the placeholder's place with animated crowds off (`Animation.Animated crowd in split-screen`).
4. **LODs.** `LODs.rcsmodel`'s `|crowdentity<g>|LOD1|LOD1Shape` and `LOD2Shape` (60 groups) are the distance shapes; unused.
5. The mask's mean absolute grey rose: thin posed figures do not line up with the original's animated ones. Judge pose work on a
   figure-by-figure comparison of the right stand's front row, not on the mask alone.

## Next Steps

- **Pose first (cheap, offline):** pose each entity's rig at a frozen frame of its group's clip on the CPU: `rig::NodeMotion::sample`
  per skeleton node, bone palette = pose world x inverse(model bind world) (both read: the model's node `bind` and the skeleton's
  `bind_world`), blend four weights, vertices x 100, then the entity node. The clip index per group is the second element of the
  group table (`0x0201e500`: groups `0..14` clips `0 0 1 1 2 2 3 3 4 4 5 5 5 5 5`, repeated in each block of 15).
- The start offset is drawn from `rand()` and the pair `(5.0, 0.5)` at `0x0201bf4c` (`0x3f00000040a00000`): read `FUN_0163d3c0` for
  how the instance record uses it before choosing a frame.
- Playing the clip needs a bone palette in the vertex stage (a new vertex format); do that only after the frozen frame reads right.
