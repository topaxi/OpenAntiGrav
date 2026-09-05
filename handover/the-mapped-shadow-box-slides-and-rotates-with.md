# The `mapped` shadow box slides and rotates with the player, and nothing snaps it to texels

2026-09-05, **reported from play by the maintainer**: *"the mapped shadows
setting implemented shadow move based on the camera moving, which seems
wrong."* This project treats a from-play observation as a real oracle, and this
one is corroborated by a source read before anything was measured - two
mechanisms in `render_depth_map` each produce exactly that symptom.

**The light itself is not the problem, which is worth saying first.**
`mapped_light` (`crates/game/src/race/scene/frame/shadow.rs:288`) returns the
circuit's authored sun when the light is enabled, and Pulse's own
`AUTHORED_AXIS` otherwise. Neither is camera-derived. The direction the shadow
is cast *along* is right; what moves is the **box the depth map is fitted to**.

## Open

- **The fit centre slides continuously with the player, and nothing snaps it to
  a texel grid.** `render_depth_map` builds it as

  ```rust
  let player = race.ship_model_matrix_of(0);
  let centre = player.transform_point3(Vec3::ZERO)
      + player.transform_vector3(Vec3::Z).normalize_or_zero() * MAPPED_AHEAD;
  ```

  and `crates/render/src/shadow/map.rs` contains no `snap`, `round` or `floor`
  against the map's texel size anywhere. A directional light's orthographic box
  whose centre moves by arbitrary sub-texel amounts re-quantises every shadow
  texel every frame, and the standard symptom is shadows that crawl or swim
  under a moving view. **This alone is sufficient to explain the report.**

- **The offset also *rotates* the box, which texel snapping cannot fix.** The
  `MAPPED_AHEAD` term is taken along the player's own forward
  (`transform_vector3(Vec3::Z)`), so as the craft turns, the box's centre swings
  around it. A box that rotates relative to the world re-samples the scene no
  matter how well its translation is snapped. The usual construction keeps the
  ortho box axis-aligned **in light space** and only ever translates it, in whole
  texel steps.

- **Whether the box should follow the player at all is a separate question and
  is not settled here.** Following the view is the normal reason a single
  cascade exists - the comment in the code says so: "one cascade's texels go
  where the camera is pointed". The bug is not that it follows; it is that it
  follows *smoothly and rotatably* rather than in snapped, world-locked steps.

- **What the original does is unread.** `shadowMapTexSize` is a real engine
  parameter whose value has never been recovered (`map.rs`'s own module docs say
  `SIZE` is a choice and the fit is ours). So there is no ground truth yet for
  the box's size, its centring, or whether the original moved it at all.

## Next Steps

1. **Reproduce it headlessly before changing anything.** Capture two frames a
   few ticks apart with the craft stationary and the camera moving, then with
   both moving, and diff the shadow region. That separates "shadows swim under
   camera motion" from "shadows swim under craft motion" - the report says
   camera, the code says player, and those are different bugs with different
   fixes. `--no-audio` and the game's own `--screenshot` reach real frames
   headlessly; `crates/render/tests/shadow_map_coverage.rs` is the existing
   harness to extend.
2. **Snap the fit centre to whole texels in light space.** Project the centre
   into the light's basis, round to `radius * 2.0 / SIZE` increments on both
   axes, project back. This is the standard fix and it is testable without a
   GPU: the same centre input, nudged by a fraction of a texel, must produce a
   byte-identical matrix.
3. **Then decide the rotation.** Either drop the player-forward term and centre
   on the craft, or keep a lead offset but compute it in a world-fixed basis so
   the box never rotates. Measure the difference rather than choosing by taste.
4. **Only then consider matching the original.** Recovering `shadowMapTexSize`
   and how the original centred its map would let the fit stop being ours - see
   `map.rs`'s module docs for what is already known to be a choice rather than a
   measurement.

## Notes

- `oag-render` is **exempt** from the determinism rules - `mul_add`, SIMD `glam`
  and the algebraic float ops are all fair game here. A shadow change must not
  move the committed state hash; if it does, something has been put in a
  simulation crate by mistake. Never edit `crates/core/src/hash.rs`'s reference
  constants.
- The `coverage` tier (the quad-based one, `crates/render/src/shadow.rs`) is a
  different mechanism and is **not** implicated: its quad axes come from the
  surface, not the camera. Do not change it while fixing this.
- `handover/shadows-are-planned-and-pulses-occluder-payload-closes.md` and
  `docs/rendering/shadows.md` are the surrounding context.
