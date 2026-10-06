# HD animates a texture in two ways now; the rest of its shader families still draw still

2026-10-06, `hd-anim-textures`. HD has no keyframe block. The fragment `time`
glow (119 slots) was already wired; this lane wired the **vertex** half,
`uv + time * rate` with the rate authored on the material, as
`mesh::AnimTrack::Scroll` (76 materials on 12 circuits, pinned per circuit by
`crates/render/tests/hd_vertex_scroll_ground_truth.rs`). Law and census:
`docs/formats/rcsmaterial.md`, "HD's vertex programs scroll the texture
coordinate". `crates/render/examples/hd_anim_family_census.rs` lists every family.

## Open

- **No frame of a scrolled surface in motion exists.** The whole-circuit camera
  paints 25 and 13 pixels for Tech de Ra and Anulpha Pass, 0 for Amphiseum; a
  matched race camera at an Amphiseum hologram was not framed (`--camera-pose`
  from `hd_scroll_where`'s vertex samples put the eye inside a wall; the
  vertices may not share the pose's frame). Needs a camera that lands on one.
- **Not wired, by named reason**: second-UV-set scrolls (`animlights`,
  `animhexlights`, `v[8]`); two-texture materials (`cf_uvanim_emssive_glowtint*`,
  `mr_uvanim_em_*`, `cf_waterfall`, `dc_hologramwithstatic2`); shader-literal
  rates (`nr_twinblend`, `reflectplane_dc_seawater`, `cf_chenghou_sign`,
  `sign_emissive_glow2uv`) - readable from the microcode, not read; fragment-only
  `time` (`nr_crowd_bustle` 33, `nr_billboardholographicscanlines` 35,
  `scanlinebillboard*`, `uvdistortion_*`, `water`).
- The admitted set is a rate-hash table read off the vertex microcode by
  `scripts/ps3-microcode.py`, not a Rust microcode decode. A Rust read of
  `o[TCn] = MAD(time, rate, v[2])` in `oag_rcs::rcsmaterial::vertex` would admit
  the other two-texture and literal-rate cases honestly.
- Not checked on a PS3 capture: the sign and direction of the scroll against the
  original, and whether the `v[2]` attribute is the coordinate set this renderer
  carries as `texcoord` (the same question the fragment glow records).
- Omega: `basic_uv_scroll`/`scrollingalpha` carry HD's names but the programs are
  GCN and unread - checked, applies in name, not wired.

## Next Steps

1. Frame one scrolled hologram or `basic_uv_scroll` strip in a race (find its
   placed position from the draw list) and difference two clocks.
2. Teach `vertex.rs` to read the multiply-add so the table goes.
3. Take a PS3 capture of Tech de Ra's orange strips to settle the direction.
