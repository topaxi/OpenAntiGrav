---
status: ready-to-wire
titles: [pulse]
updated: 2026-11-12
summary: "Gemini Dam tunnel ceiling light z-fights ours only: the authored Batch::depth_bias (3, 6) is read, not applied; texture path matches"
size: M (50)
difficulty: medium (45)
---

# Gemini Dam Black: a tunnel ceiling light flickers rapidly

The maintainer saw this in play on 2026-11-12 (Pulse, PSP, Gemini Dam Black = `12_Track` forward). The tunnel was not named; the
measure lane (2026-11-12) found the cause at the long tunnel at spline rows about 1,180 to 1,430 and ruled out the time path.

## Settled (measured against the original)

- **Cause, confidence 75** (the bias was not applied in a test render; the signature and the data agree): the ceiling light strips of that tunnel (`Anotherblue_GLOW`, nodes 121, 122, 123, 133 of the pack's
  `track.vex`) are authored with a **depth bias** (`Batch::depth_bias` 3 on 121/122/123, 6 on 133) and lie on the ceiling
  plates. The original shifts the batch's depth range by `6 * bias` (`mesh-draw.md`, "The per-batch depth bias is file data"), so
  the glow wins; ours reads the byte and does not apply it, so the glow z-fights the ceiling. At a pose matched to about 1 unit
  (spline row 1205, craft held at the lifted sample) the original's band is one continuous cyan strip; ours is cut into dashes
  that **change pattern with a 0.05 unit camera step** (four steps, four different patterns, one with speckle), which is
  z-fighting, and shimmers in motion. `--pvs false` and `true` give the same picture, so it is not visibility. Each of those
  meshes' batches is also listed twice in list A (`pass 0x1891` alpha-tested and `0x1011` opaque, same vertices) beside a list-B
  copy (`0x1092`); that is an authored triple and may be intended, but it doubles the tie.
- **The `TEXOFFSET` path is not the cause.** The original's per-frame offsets match `oag_vex::TexTransform::sample` (lerp, same
  keys and loops; `texture-animation.md`, "The updater's output measured per frame on Gemini Dam"), and the band's brightness
  follows the same 60-frame sawtooth in both. `12_Track` has no block with the step bit, so the 60-confidence snap direction of
  `sample_with` is untouched by this.
- `12_Track` is PSP DLC (Harimau `PACK3.edat`), run in the original with upper-case `.EDAT` names under
  `msroot:\PSP\GAME\UCES00465\`; Gemini Dam Black is Track Select entry 30 of 32 (`docs/formats/dlc-pack.md`).
  The grid state is `data/saves/pulse-psp/single-race-gemini-dam-black-grid.ppst` (USA serial).

## Open

- **Applying the bias in `oag-mesh`**: the original's bias is `sceGuDepthRange(1001 + 6 * bias, 64946 + 6 * bias)` on a 16-bit
  `GREATER` depth buffer with near 1.2 and far 2000, so it is a **constant in the original's depth units, not in world
  units** (about 1.2 depth levels per unit at 250 units). It needs a per-batch constant offset in ours' depth encoding
  (a per-batch uniform or vertex attribute added in the vertex shader after projection, or one pipeline per bias value `1..=9`
  with `DepthBiasState`), and the two-pass opaque/cutout draw of a duplicated batch tied the same way. Not done here: the change
  reaches every biased batch on the disc (5,605 of 67,638), so it wants a pass over the glow, additive-cone, tree and
  `greenfade` surfaces, which is the open item of the flyby thread (`pulse-flyby-window-stipple`).
- **The arch ceiling lights** (`screen1_GLOW` 4 cells at 12.5 frames, `screen3` 60-frame loop; nodes 89, 181, 275, 393, 405)
  were measured in the original's texture updater only; their frames were not compared (they hang above the chase camera).
  They carry no depth bias in the header read here, so they are not this defect; if the maintainer's light is one of them,
  the next step is a nose-up pose in the original.

## Next Steps

1. Wire `Batch::depth_bias` into the mesh draw (address of the data: `oag_vex::vex::Batch::depth_bias`, `crates/vex/src/vex.rs`;
   the original's setter `FUN_0891e988`, `mesh-draw.md`). Ship it as a render-profile setting only if it moves other circuits'
   pictures; see `docs/rendering/batch-cull.md`.
2. Acceptance: ours at spline row 1205 (`--pose=-653.037,-153.7,214.494`, `--anim-seconds 0.5`) shows the continuous strip at
   four camera offsets of 0.05 units, the same crop of the original. A test pins the bias value fed to the draw for a
   batch with `depth_bias 3` and `0`, red before.
3. Re-run `oag-game --race` on `12_Track` with the DLC (`--dlc`) and check that the other biased surfaces
   (`col_lights`, `greenfade`, additive cones) did not move for the worse.
