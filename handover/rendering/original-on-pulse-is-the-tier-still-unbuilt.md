# All four shadow tiers draw; `mapped` has one named gap and 2048 has none of it

2026-09-04. Rewritten from the file that split off
[shadows-are-planned-and-pulses-occluder-payload-closes.md](shadows-are-planned-and-pulses-occluder-payload-closes.md)
on 2026-09-03 - **steps 2 and 3 of
[`docs/rendering/shadows.md`](../../docs/rendering/shadows.md)'s plan have landed
and this file is what is left**, which is step 5.

## What landed

- **The setting.** `oag_display::display::Shadows`, `off` and `blob`, in
  `[render_profiles.<title>] shadows`; a `SHADOWS` row on the GRAPHICS page;
  `--shadows` as a per-run override. `original` and `mapped` are refused by
  `FromStr`, and `display/tests.rs` names both so whoever lands one deletes
  their line beside a new variant.
- **`blob`.** `oag_render::shadow` draws it, `oag_game::race::shadow` places
  it. HD's own nine `ambient_shadow.gtf` are the silhouette where the source
  ships them, a generated falloff elsewhere, and the load report says which
  happened per slot. **Diffed `off` against `blob` on four titles** rather
  than eyeballed: Pulse PSP 8,595 pixels changed at a worst delta of 43,
  Pulse PS2 4,637 at 75, Pure 16,924 at 166, HD/Fury 25,022 at 77.

Two things worth carrying forward from doing it:

- **A shadow can be uploaded, drawn, and invisible.** The first fade faded
  linearly from the ground, so a craft resting at its own ride height came out
  at strength `0.030` - 5,262 pixels changed by a maximum of 2, which reads
  exactly like "the pass never ran". An `off`-versus-`blob` PNG diff with the
  threshold at **zero** is what told the two apart; at any threshold above 4
  the frames looked byte-identical.
- **`raycast` returns the nearest hit of any surface**, so the placement casts
  `raycast_all` and takes the nearest `Floor`/`MagFloor`. A nearest-hit query
  puts a craft's shadow up the barrier it is scraping.

## Open

- ~~Step 5, `original` on Pulse~~ **Landed 2026-09-04.** The craft's own hull,
  its silhouette taken in hull space against `oag_pulse::shadow::AUTHORED_AXIS`,
  projected through the craft's world matrix onto the surface its downward cast
  found, fanned from each ring's centroid. Diffed against both other tiers at
  one camera pose: `blob` against `original` changes 5,720 pixels at a worst
  delta of 126, which is the check that says they draw different things.
- ~~`original` on HD~~ **Landed 2026-09-04**, and its mechanism is not Pulse's:
  the craft render into a **coverage** map from the `.envsettings` sun and the
  track samples it projectively. That the map holds coverage rather than depth
  is the microcode's own statement - `TXP R1.x, f[TC0] unit2` then
  `ADD H0.w, -R1.xxxx, {1}`, with nothing compared anywhere - so there is no
  depth attachment, no bias and no comparison sampler in the pass.
- **`original` on 2048**: the `track_proximity_shadow` pair and the
  precomputed environment shadows, and it needs 2048 to race first.
- **`mapped` landed 2026-09-04 with a gap that is written down rather than
  tuned away: a craft's own shadow does not appear on the road.** Everything
  else does - scenery on scenery, and the frame at large. Ruled out by separate
  runs: the depth pass writes depth (the texels are read back in
  `shadow_map_coverage.rs`), the map has content at the road's texels, the road
  computes a `uv` inside the map, road and craft land 13 texels apart at 2048
  exactly as the light's tilt predicts, and the craft's caster carries 13
  ranges and 4,335 indices. **The first thing to check** is the one difference
  between the two vertex paths: `caster.wgsl` does `mvp * position` where
  `mesh.wgsl` does `model * (node_anims.transform[xform] * position)`, and the
  caster pass binds no node matrices at all - so anything baked in an anim
  node's space is cast from the wrong place.
- **Two bugs the `mapped` work turned up, both of which shadowed *something***
  and so read as working: the map lookup was mirrored vertically (glam's
  `rh::proj::directx` projections are **Y-down**, and HD's coverage tier had
  the same bug hidden by a caster sitting near its own map's centre), and the
  shadow term reached one fragment entry point of five - **a race draws through
  the `_velocity` pair**, which is why the road went untouched while the
  scenery did not.
- **HD's `LiveStencilShadow` path is not built either**, and it is a separate
  thing from the map: a two-sided depth-fail stencil test over a rigidly
  shifted box proxy, decoded down to all 39 `shadow.stencilvolume` files. What
  it is *for* - which craft, when - is unread.
- **Two bugs that still drew a plausible dark shape**, both fixed, and worth
  carrying because neither would have failed a "was anything drawn" test: a
  centroid fan over a *non-convex* silhouette (53 of 129 hulls are concave)
  drew a crumpled star, and the hull was projected without the `Transform`
  chain above its node - identity on Pulse's craft, and not on a model where
  it is not. The guard is now a shape comparison: the hull's plan-view
  footprint covers 93 % of the craft mesh's and spills 4 % beyond it.
- **`HULL_DARKNESS` is ours and unevidenced.** What a stencil volume is
  darkened by is decided by the pass that fills it, and that pass is unread; at
  full alpha the tier drew a black hole in the road, so `0.35` is a number
  picked to read as a shadow and derived from nothing. **HD's own
  `MAP_STRENGTH` is the same kind of number** and for the same reason - its
  compositing pass is unread too. Reading either would replace a choice with a
  measurement.
- **Two numbers with no evidence behind them, one per title**: Pulse's
  `HULL_DARKNESS` and HD's `MAP_STRENGTH`. Both exist because the pass that
  turns the original's shadow term into pixels is unread on both titles - HD
  puts `1 - shadow` in the fragment's alpha (`ShadowToAlpha`) and composites it
  later, and that compositing pass has never been traced.
- **HD's map size is ours too.** `shadowMapTexSize` is a real engine parameter
  and its value is unread; 1024 is a choice.
- **HD's hull is a receiver now, and half of it is still open - 2026-09-21.**
  [`ship-sun-occlusion.md`](../../docs/ghidra/functions/ps3-hdfury-eu/ship-sun-occlusion.md):
  `Job RenderShips` renders the track within ten units of the sun line
  through each craft, from the sun, through its `SunOcclusionLightmap`/
  `SunOcclusionVertex` technique into a second per-ship map, and the hull's
  `ShadowMap` variant multiplies its sun by `compare(own depth map) *
  occlusion`. `oag_render::shadow::occlusion` draws the occlusion half and
  `mesh.wgsl` gates by it (`OAG_DUMP_SUN_OCCLUSION=<png>` writes the
  player's layer; tunnel 12/255, glass floor 245/255 on Talon's Junction).
  ~~(a) the self-shadow half~~ **built 2026-09-21**:
  `oag_render::shadow::self_shadow`, a front-face-culled depth layer per
  craft through the occlusion layer's own matrix, compared in `mesh.wgsl`
  (`OAG_DUMP_SELF_SHADOW=<png>` dumps it; the Feisar's cockpit recess and
  pod inner faces darken at tick 4800, Pulse byte-identical). Open: (b) the original's **chunk
  draw order** decides what a texel holds where geometry overlaps along the
  sun with no depth test, and is unread - this side draws in the model's
  draw order; (c) the six tracks (`17_Track`, `18_Track`, `24_Track`,
  `23_Track`, `07_Track`, `15_Track`) and Zone clear the map **white**
  rather than black in the original, and which disc folders those names
  are was not resolved, so this side clears black everywhere; (d) which
  ship gets which map slot (512 down to 32 texels) is unread - every layer
  is 256; (e) **it reads weaker than the original** because the sun is a
  small share of the hull's light on this side: forcing the gate to zero
  darkens the Feisar hull by 19 % at tick 4800, which is the scene
  calibration thread's question, not this mechanism's. One trap worth the
  reading time: a map fitted to the ship drawable's own bounding radius is
  164 units across on an HD `Ship.vex` (its authored geometry reaches far
  past the hull), and reads as a map of the neighbourhood - the fit is the
  original's fallback cube instead.
- **Nothing shadows anything but the road.** The polygon is the volume's ground
  cap, so a craft under a bridge does not darken the bridge and one craft does
  not shadow another. Whether that is worth a real stencil volume is a
  question about how it looks in motion, which nobody has seen.
- ~~The `m` face-to-vertex index mapping's exact byte layout~~ **Read
  2026-09-04**, and the parser is `oag_vex::shadow_occluder`: `u16[4]` of
  per-edge adjacent faces at `+0x10`, `u16[4]` of vertex indices at `+0x18`,
  the fourth repeating the first on a triangle. Reciprocal on **14,328 of
  14,328** edges disc-wide, every index in range on 4,381 of 4,381 faces, and
  every triangle's declared normal within **0.028 degrees** of the geometry of
  the vertices it indexes. `Occluder::silhouette` is the edge walk step 5
  needs.
- ~~The projection direction is what now blocks the draw~~ **Read
  2026-09-04**: `(1, -10, 2)` normalized (`z / x` bit-exactly `2.0`, and the
  vector `4.8e-6` short of unit - a fast reciprocal square root, so the shipped
  bits are what `oag_pulse::shadow::AUTHORED_AXIS` keeps), a *local* axis the
  node's own world matrix carries into world space. It sits in `.bss` at `g_shadow_direction`
  (`0x08b62540`), written by `Shadow_RegisterClass` (`0x08923518`) - the same
  function that registers class `0x3cb`, `shadow`. **The reason it read as
  unfindable is worth carrying**: both candidate addresses are reached through
  PRX relocations with `addr_base = 1`, so the obvious reading of the
  `lui`/`addiu` pair lands in `.text` and decodes as instructions - the
  `shield-pickup.md` trap, one segment over. Read `.rel.text` before believing
  an address has no bytes behind it.
- **`shadow` `0x3cb` is a direction override, not dead weight**, and the census
  line calling it inert now says so: `g_shadow_node_count` is a reference count
  incremented by its constructor and decremented by its destructor, and while
  one is alive every shadow projects along that node's own negated vector
  instead of the constant. No `.vex` on the disc authors one, so nothing
  shipped ever takes that path. What `0x0892342c` (the function that fills the
  override) belongs to is unread.
- **Whether the 10 unnamed/world-space occluders are a track feature at all**,
  or authored-and-inert the way `DirectionalLight` turned out to be. If step 5
  draws only the 119 named ones, say so in the implementation rather than
  silently dropping the rest.
- **Whether `original` should be the default once it exists.** A design
  question needing eyes on both tiers side by side, which is now possible for
  the first time on one of them.
- **Nobody has looked at `blob` in a window.** Every judgement here is from a
  headless capture; the fade's shape, the falloff's darkness and `LIFT`'s size
  are all ours and none has been seen in motion.
- **HD's `ambientShadowBlendFactor`** is a named engine parameter
  ([`shadows.md`](../../docs/rendering/shadows.md)) whose value has never been
  read. `blob` currently draws the disc's coverage at face value; if that
  constant scales it, ours is too dark or too light by whatever it says.
- **The padded-bbox field's exact selection rule** is still not fully
  explained - does not block drawing, wanted for an exact reproduction.

## Next Steps

- Look at both drawn tiers in a window on one title and judge the four numbers
  that are ours: the fade, the falloff's darkness, the lift, and
  `HULL_DARKNESS`. Every judgement so far is from a headless capture.
- Read the compositing pass on either title - `RenderModelShadowsOnTrack` on
  HD is the nearer one, since its material flag `ShadowToAlpha` is already
  bound at `0x405d48` - and replace both darkness constants with measurements.
- Bind the node matrices in the caster pass, which is the named gap above and
  the likeliest reason a craft casts nothing onto the road.
- A real cascade ladder: `mapped` is one cascade fitted ahead of the camera,
  which is a choice about where its texels go rather than a solution.
