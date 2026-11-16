---
status: measure
titles: [hd]
updated: 2026-11-16
summary: "Sol 2 sky band fixed (cube faces on 128-byte boundaries); left: original frame of a moved face, Sol 2 flipped nadir, Zone arena skies"
---

# HD sky cubemaps: what the Sol 2 band fix left open

The band on Sol 2's sky was `oag_texture::gtf` reading cubemap faces unaligned:
each face's mip chain is padded to a 128-byte boundary
(`docs/formats/gtf.md`, "Each face starts on a 128-byte boundary"). Fixed and
pinned by `gtf::tests::a_mip_chained_cubemap_reads_each_face_from_its_aligned_start`
and `gtf_ground_truth`'s `CubemapFaces` seams. Three things were not settled.

## Open

- **No frame of the original on a face the fix moved.** The matched pair
  `data/reference/compare-page/hd/sol-2` looks at face 0, which both readings
  agree on (0.7 RMSE either way); the other `hd-capture/sol2-*` frames are
  blown-out sky. The seam evidence (39.21 to 2.62 on all four side seams) is
  what carries the fix.
- **Sol 2's nadir is stored flipped top to bottom** against the GL face table
  (`gtf.md`, "Sol 2's nadir is stored flipped, and is drawn as stored"). We draw
  it as stored, on the reading that the RSX does too; not checked against the
  original. Shows only as a crease in a sky-only render.
- **The Zone arenas' own `sky.gtf` (`zone_1..4`) are another face layout**
  (`gtf.md`, "The Zone arenas' skies are another layout"). Dead art in Zone
  mode (a Zone race draws `ZoneSky.gtf`); whether any other mode puts an
  arena's own sky on screen is unchecked. If one does, `mesh::sky_cube` draws
  it with mismatched seams.

## Next Steps

1. RPCS3 (`docs/reverse-engineering/emulator-recipes.md`, HD capture recipes):
   Sol 2 Time Trial, turn the camera to open sky away from the start's face 0
   (world yaw roughly -90 to -150 from the start heading); render ours at the
   read camera pose with `--camera-pose` and compare the sky patch. A shifted
   face reading would show a 36 to 180 texel offset.
2. Same session: look down past the track edge for the nadir crease, to
   confirm or refute "drawn as stored".
3. Only if a mode draws an arena's own sky: work out the arena layout from the
   edge matches in `gtf.md` (face 0 top = face 3 bottom, face 1 top = face 2
   bottom, face 2 left = face 4 top, face 1 right = face 5 left) and give
   `sky_cube` a per-file face table.
